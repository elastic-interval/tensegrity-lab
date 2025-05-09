use crate::fabric::physics::presets::AIR_GRAVITY;
use crate::{
    ControlState, CrucibleAction, LabEvent, PhysicsFeature, PhysicsParameter, Radio, StateChange,
    TesterAction,
};
use winit::event::KeyEvent;
use winit::keyboard::{KeyCode, PhysicalKey, SmolStr};

enum KeyAction {
    KeyLabEvent {
        code: KeyCode,
        description: String,
        lab_event: Box<dyn Fn(&ControlState) -> LabEvent>,
        radio: Radio,
        is_active_in: Box<dyn Fn(&ControlState) -> bool>,
    },
    FloatParameter {
        up_code: SmolStr,
        down_code: SmolStr,
        physics_parameter: PhysicsParameter,
        radio: Radio,
        render: Box<dyn Fn(&f32) -> String>,
        is_active_in: Box<dyn Fn(&ControlState) -> bool>,
    },
}

pub struct Keyboard {
    radio: Radio,
    actions: Vec<KeyAction>,
}

impl Keyboard {
    pub fn new(radio: Radio) -> Self {
        Self {
            radio,
            actions: Default::default(),
        }
    }

    pub fn with_actions(mut self) -> Self {
        use ControlState::*;
        use CrucibleAction::*;
        use LabEvent::*;
        self.key_dynamic_lab_event(
            KeyCode::KeyP,
            "Print",
            Box::new(|control_state| {
                if let ShowingInterval(interval_details) = control_state {
                    PrintCord(interval_details.length)
                } else {
                    panic!("expected ShowingInterval state")
                }
            }),
            Box::new(|state| matches!(state, ShowingInterval(_))),
        );
        self.key_lab_event(
            KeyCode::Escape,
            "ESC to cancel selection",
            Crucible(ToViewing),
            Box::new(|state| matches!(state, ShowingJoint(_) | ShowingInterval(_))),
        );
        self.key_lab_event(
            KeyCode::Space,
            "Space to stop animation",
            Crucible(ToViewing),
            Box::new(|state| matches!(state, Animating)),
        );
        self.key_lab_event(
            KeyCode::Space,
            "Space to start animation",
            Crucible(ToAnimating),
            Box::new(|state| matches!(state, Viewing)),
        );
        self.float_parameter(
            "T",
            "t",
            PhysicsParameter {
                feature: PhysicsFeature::IterationsPerFrame,
                value: 100.0,
            },
            Box::new(|value| format!("Time {value:.0}")),
            Box::new(|state| matches!(state, PhysicsTesting(_) | FailureTesting(_))),
        );
        self.float_parameter(
            "P",
            "p",
            PhysicsParameter {
                feature: PhysicsFeature::Pretenst,
                value: AIR_GRAVITY.pretenst,
            },
            Box::new(|value| format!("Pretenst {value:.5}")),
            Box::new(|state| matches!(state, PhysicsTesting(_) | FailureTesting(_))),
        );
        self.float_parameter(
            "C",
            "c",
            PhysicsParameter {
                feature: PhysicsFeature::CycleTicks,
                value: 1000.0,
            },
            Box::new(|value| format!("Cycle {:.0}", value)),
            Box::new(|state| matches!(state, PhysicsTesting(_))),
        );
        self.float_parameter(
            "S",
            "s",
            PhysicsParameter {
                feature: PhysicsFeature::Stiffness,
                value: 0.0,
            },
            Box::new(|value| format!("Stiffness {:.0}", value * 1e4)),
            Box::new(|state| matches!(state, PhysicsTesting(_) | FailureTesting(_))),
        );
        self.float_parameter(
            "M",
            "m",
            PhysicsParameter {
                feature: PhysicsFeature::Mass,
                value: 1.0,
            },
            Box::new(|value| format!("Mass {:.0}", value * 1e2)),
            Box::new(|state| matches!(state, PhysicsTesting(_) | FailureTesting(_))),
        );
        self.float_parameter(
            "L",
            "l",
            PhysicsParameter {
                feature: PhysicsFeature::StrainLimit,
                value: 1.0,
            },
            Box::new(|value| format!("Strain Limit {:.1}", value * 1e2)),
            Box::new(|state| matches!(state, PhysicsTesting(_) | FailureTesting(_))),
        );
        self.key_lab_event(
            KeyCode::KeyY,
            "",
            Crucible(TesterDo(TesterAction::DumpPhysics)),
            Box::new(|state| matches!(state, PhysicsTesting(_))),
        );
        self.key_lab_event(
            KeyCode::ArrowLeft,
            "\u{2190} previous test",
            Crucible(TesterDo(TesterAction::PrevExperiment)),
            Box::new(|state| matches!(state, FailureTesting(_))),
        );
        self.key_lab_event(
            KeyCode::ArrowRight,
            "\u{2192} next test",
            Crucible(TesterDo(TesterAction::NextExperiment)),
            Box::new(|state| matches!(state, FailureTesting(_))),
        );
        self.key_lab_event(
            KeyCode::KeyX,
            "", // hidden
            DumpCSV,
            Box::new(|state| matches!(state, Viewing)),
        );
        self
    }

    pub fn set_float_parameter(&mut self, parameter_to_set: &PhysicsParameter) {
        for action in self.actions.iter_mut() {
            if let KeyAction::FloatParameter {
                physics_parameter, ..
            } = action
            {
                if physics_parameter.feature == parameter_to_set.feature {
                    physics_parameter.value = parameter_to_set.value;
                }
            }
        }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent, control_state: &ControlState) {
        if key_event.state.is_pressed() {
            if let KeyEvent {
                physical_key: PhysicalKey::Code(pressed_key),
                text,
                ..
            } = key_event
            {
                let text = text.unwrap_or_default();
                for action in self.actions.iter_mut() {
                    match action {
                        KeyAction::KeyLabEvent {
                            code,
                            is_active_in,
                            radio,
                            lab_event,
                            ..
                        } => {
                            if *code == pressed_key && is_active_in(control_state) {
                                lab_event(control_state).send(&radio);
                            }
                        }
                        KeyAction::FloatParameter {
                            up_code,
                            down_code,
                            radio,
                            physics_parameter,
                            ..
                        } => {
                            if text == *up_code {
                                StateChange::SetPhysicsParameter(
                                    physics_parameter
                                        .feature
                                        .parameter(physics_parameter.value * 1.1),
                                )
                                .send(radio);
                            }
                            if text == *down_code {
                                StateChange::SetPhysicsParameter(
                                    physics_parameter
                                        .feature
                                        .parameter(physics_parameter.value * 0.9),
                                )
                                .send(radio);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn legend(&self, control_state: &ControlState) -> Vec<String> {
        let mut legend = vec![];
        for action in self.actions.iter() {
            match action {
                KeyAction::KeyLabEvent {
                    is_active_in,
                    description,
                    ..
                } => {
                    if is_active_in(control_state) && !description.is_empty() {
                        legend.push(description.clone());
                    }
                }
                KeyAction::FloatParameter {
                    is_active_in,
                    render,
                    physics_parameter: PhysicsParameter { value, .. },
                    ..
                } => {
                    if is_active_in(control_state) {
                        legend.push(render(value));
                    }
                }
            }
        }
        legend
    }

    fn key_lab_event(
        &mut self,
        code: KeyCode,
        description: &str,
        lab_event: LabEvent,
        is_active_in: Box<dyn Fn(&ControlState) -> bool>,
    ) {
        self.actions.push(KeyAction::KeyLabEvent {
            code,
            description: description.into(),
            lab_event: Box::new(move |_| lab_event.clone()),
            radio: self.radio.clone(),
            is_active_in,
        });
    }

    fn key_dynamic_lab_event(
        &mut self,
        code: KeyCode,
        description: &str,
        lab_event: Box<dyn Fn(&ControlState) -> LabEvent>,
        is_active_in: Box<dyn Fn(&ControlState) -> bool>,
    ) {
        self.actions.push(KeyAction::KeyLabEvent {
            code,
            description: description.into(),
            lab_event,
            radio: self.radio.clone(),
            is_active_in,
        });
    }

    fn float_parameter(
        &mut self,
        up_code: &str,
        down_code: &str,
        physics_parameter: PhysicsParameter,
        render: Box<dyn Fn(&f32) -> String>,
        is_active_in: Box<dyn Fn(&ControlState) -> bool>,
    ) {
        self.actions.push(KeyAction::FloatParameter {
            up_code: up_code.into(),
            down_code: down_code.into(),
            render,
            is_active_in,
            physics_parameter,
            radio: self.radio.clone(),
        })
    }
}
