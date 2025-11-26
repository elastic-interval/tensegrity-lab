use crate::fabric::physics::presets::BASE_PHYSICS;
use crate::{
    ControlState, CrucibleAction, LabEvent, PhysicsFeature, PhysicsParameter, Radio, StateChange,
    TestScenario, TesterAction, TweakFeature, TweakParameter,
};
use winit::event::KeyEvent;
use winit::keyboard::{KeyCode, PhysicalKey, SmolStr};
use crate::ControlState::*;

enum KeyAction {
    KeyLabEvent {
        code: KeyCode,
        description: String,
        lab_event: Box<dyn Fn(&ControlState) -> LabEvent>,
        radio: Radio,
        is_active_in: Box<dyn Fn(&ControlState) -> bool>,
    },
    PhysicsParameter {
        up_code: SmolStr,
        down_code: SmolStr,
        parameter: PhysicsParameter,
        radio: Radio,
        render: Box<dyn Fn(&f32) -> String>,
        is_active_in: Box<dyn Fn(&ControlState) -> bool>,
    },
    TweakParameter {
        up_code: SmolStr,
        down_code: SmolStr,
        parameter: TweakParameter,
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
            Box::new(|state| {
                matches!(
                    state,
                    ShowingJoint(_) | ShowingInterval(_)
                )
            }),
        );
        self.key_lab_event(
            KeyCode::Escape,
            "Exit",
            Crucible(ToViewing),
            Box::new(|state| matches!(state, PhysicsTesting(_))),
        );
        // self.key_lab_event(
        //     KeyCode::Space,
        //     "Stop animation",
        //     Crucible(ToViewing),
        //     Box::new(|state| matches!(state, Animating)),
        // );
        // self.key_lab_event(
        //     KeyCode::Space,
        //     "Start animation",
        //     Crucible(ToAnimating),
        //     Box::new(|state| matches!(state, Viewing)),
        // );
        self.key_lab_event(
            KeyCode::KeyT,
            "Physics testing",
            Crucible(ToPhysicsTesting(TestScenario::PhysicsTest)),
            Box::new(|state| matches!(state, Viewing)),
        );
        self.float_parameter(
            "P",
            "p",
            PhysicsParameter {
                feature: PhysicsFeature::Pretenst,
                value: *BASE_PHYSICS.pretenst,
            },
            Box::new(|value| format!("Pretenst {value:.3}")),
            Box::new(|state| matches!(state, PhysicsTesting(_))),
        );
        self.tweak_parameter(
            "M",
            "m",
            TweakParameter {
                feature: TweakFeature::MassScale,
                value: 1.0,
            },
            Box::new(|value| format!("Mass {value:.4}")),
            Box::new(|state| matches!(state, PhysicsTesting(_))),
        );
        self.tweak_parameter(
            "R",
            "r",
            TweakParameter {
                feature: TweakFeature::RigidityScale,
                value: 1.0,
            },
            Box::new(|value| format!("Rigidity {value:.4}")),
            Box::new(|state| matches!(state, PhysicsTesting(_))),
        );
        self.key_lab_event(
            KeyCode::KeyS,
            "Movement stats",
            Crucible(TesterDo(TesterAction::ToggleMovementSampler)),
            Box::new(|state| matches!(state, PhysicsTesting(_))),
        );
        self.key_lab_event(
            KeyCode::KeyJ,
            "Jump",
            Crucible(CrucibleAction::DropFromHeight),
            Box::new(|state| matches!(state, PhysicsTesting(_) | Viewing)),
        );
        self.key_lab_event(
            KeyCode::KeyC,
            "Color by Role",
            UpdateState(StateChange::ToggleColorByRole),
            Box::new(|state| matches!(state, PhysicsTesting(_))),
        );
        self.key_lab_event(
            KeyCode::KeyK,
            "Knots",
            UpdateState(StateChange::ToggleAttachmentPoints),
            Box::new(|state| matches!(state, Viewing)),
        );
        self.key_lab_event(
            KeyCode::Enter,
            "Reload fabric",
            RebuildFabric,
            Box::new(|_| true),
        );
        self
    }

    pub fn set_float_parameter(&mut self, parameter_to_set: &PhysicsParameter) {
        for action in self.actions.iter_mut() {
            if let KeyAction::PhysicsParameter { parameter, .. } = action {
                if parameter.feature == parameter_to_set.feature {
                    parameter.value = parameter_to_set.value;
                }
            }
        }
    }

    pub fn set_tweak_parameter(&mut self, parameter_to_set: &TweakParameter) {
        for action in self.actions.iter_mut() {
            if let KeyAction::TweakParameter { parameter, .. } = action {
                if parameter.feature == parameter_to_set.feature {
                    parameter.value = parameter_to_set.value;
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
                        KeyAction::PhysicsParameter {
                            up_code,
                            down_code,
                            radio,
                            parameter,
                            is_active_in,
                            ..
                        } => {
                            if is_active_in(control_state) {
                                if text == *up_code {
                                    StateChange::SetPhysicsParameter(
                                        parameter.feature.parameter(parameter.value * 1.1),
                                    )
                                    .send(radio);
                                }
                                if text == *down_code {
                                    StateChange::SetPhysicsParameter(
                                        parameter.feature.parameter(parameter.value * 0.9),
                                    )
                                    .send(radio);
                                }
                            }
                        }
                        KeyAction::TweakParameter {
                            up_code,
                            down_code,
                            radio,
                            parameter,
                            is_active_in,
                            ..
                        } => {
                            if is_active_in(control_state) {
                                if text == *up_code {
                                    StateChange::SetTweakParameter(
                                        parameter.feature.parameter(parameter.value * 1.1),
                                    )
                                    .send(radio);
                                }
                                if text == *down_code {
                                    StateChange::SetTweakParameter(
                                        parameter.feature.parameter(parameter.value * 0.9),
                                    )
                                    .send(radio);
                                }
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
                KeyAction::PhysicsParameter {
                    is_active_in,
                    render,
                    parameter: PhysicsParameter { value, .. },
                    up_code,
                    down_code,
                    ..
                } => {
                    if is_active_in(control_state) {
                        // Format as "Key+/Key-: Value" for parameters
                        legend.push(format!("{}/{}: {}", up_code, down_code, render(value)));
                    }
                }
                KeyAction::TweakParameter {
                    is_active_in,
                    render,
                    parameter: TweakParameter { value, .. },
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
            KeyCode::Enter => "Enter".to_string(),
            KeyCode::ArrowLeft => "←".to_string(),
            KeyCode::ArrowRight => "→".to_string(),
            KeyCode::ArrowUp => "↑".to_string(),
            KeyCode::ArrowDown => "↓".to_string(),
            _ => format!("{:?}", code).trim_start_matches("Key").to_string(),
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
        parameter: PhysicsParameter,
        render: Box<dyn Fn(&f32) -> String>,
        is_active_in: Box<dyn Fn(&ControlState) -> bool>,
    ) {
        self.actions.push(KeyAction::PhysicsParameter {
            up_code: up_code.into(),
            down_code: down_code.into(),
            render,
            is_active_in,
            parameter,
            radio: self.radio.clone(),
        })
    }

    fn tweak_parameter(
        &mut self,
        up_code: &str,
        down_code: &str,
        parameter: TweakParameter,
        render: Box<dyn Fn(&f32) -> String>,
        is_active_in: Box<dyn Fn(&ControlState) -> bool>,
    ) {
        self.actions.push(KeyAction::TweakParameter {
            up_code: up_code.into(),
            down_code: down_code.into(),
            render,
            is_active_in,
            parameter,
            radio: self.radio.clone(),
        })
    }
}
