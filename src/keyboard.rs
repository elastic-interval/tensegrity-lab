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
            "Print cord",
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
            "Cancel selection",
            Crucible(ToViewing),
            Box::new(|state| matches!(state, ShowingJoint(_) | ShowingInterval(_))),
        );
        self.key_lab_event(
            KeyCode::Space,
            "Stop animation",
            Crucible(ToViewing),
            Box::new(|state| matches!(state, Animating)),
        );
        self.key_lab_event(
            KeyCode::Space,
            "Start animation",
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
            Box::new(|value| format!("Iterations {value:.0}")),
            Box::new(|state| matches!(state, PhysicsTesting(_) | FailureTesting(_))),
        );
        self.float_parameter(
            "P",
            "p",
            PhysicsParameter {
                feature: PhysicsFeature::Pretenst,
                value: AIR_GRAVITY.pretenst,
            },
            Box::new(|value| format!("Pretenst {value:.3}")),
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
            Box::new(|value| format!("Stiff {:.0}", value * 1e4)),
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
            Box::new(|value| format!("Strain {:.1}%", value * 1e2)),
            Box::new(|state| matches!(state, PhysicsTesting(_) | FailureTesting(_))),
        );
        self.key_lab_event(
            KeyCode::KeyY,
            "",
            Crucible(TesterDo(TesterAction::DumpPhysics)),
            Box::new(|state| matches!(state, PhysicsTesting(_))),
        );
        self.key_lab_event(
            KeyCode::KeyR,
            "Color by Role",
            UpdateState(StateChange::ToggleColorByRole),
            Box::new(|_| true),
        );
        self.key_lab_event(
            KeyCode::KeyO,
            "Projection",
            UpdateState(StateChange::ToggleProjection),
            Box::new(|_| true),
        );
        self.key_lab_event(
            KeyCode::KeyK,
            "Knots",
            UpdateState(StateChange::ToggleAttachmentPoints),
            Box::new(|_| true),
        );
        self.key_lab_event(
            KeyCode::ArrowLeft,
            "Previous test",
            Crucible(TesterDo(TesterAction::PrevExperiment)),
            Box::new(|state| matches!(state, FailureTesting(_))),
        );
        self.key_lab_event(
            KeyCode::ArrowRight,
            "Next test",
            Crucible(TesterDo(TesterAction::NextExperiment)),
            Box::new(|state| matches!(state, FailureTesting(_))),
        );
        self.key_lab_event(
            KeyCode::KeyX,
            "eXport", // hidden
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
                    code,
                    ..
                } => {
                    if is_active_in(control_state) && !description.is_empty() {
                        // Format as "Key: Action" for consistent, brief display
                        let key_name = Self::format_key_name(code);
                        legend.push(format!("{}: {}", key_name, description));
                    }
                }
                KeyAction::FloatParameter {
                    is_active_in,
                    render,
                    physics_parameter: PhysicsParameter { value, .. },
                    up_code,
                    down_code,
                    ..
                } => {
                    if is_active_in(control_state) {
                        // Format as "Key+/Key-: Value" for parameters
                        legend.push(format!("{}/{}: {}", up_code, down_code, render(value)));
                    }
                }
            }
        }
        legend
    }
    
    // Helper function to format key names consistently
    fn format_key_name(code: &KeyCode) -> String {
        match code {
            KeyCode::Space => "Space".to_string(),
            KeyCode::Escape => "Esc".to_string(),
            KeyCode::ArrowLeft => "←".to_string(),
            KeyCode::ArrowRight => "→".to_string(),
            KeyCode::ArrowUp => "↑".to_string(),
            KeyCode::ArrowDown => "↓".to_string(),
            _ => format!("{:?}", code)
                .trim_start_matches("Key")
                .to_string()
        }
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
