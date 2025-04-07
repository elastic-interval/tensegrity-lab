use crate::messages::{
    ControlState, CrucibleAction, FailureTesterAction, LabEvent, PhysicsFeature, PhysicsParameter
    , Radio, StateChange,
};
use winit::event::KeyEvent;
use winit::keyboard::{KeyCode, PhysicalKey, SmolStr};

enum KeyAction {
    SingleKey {
        code: KeyCode,
        description: String,
        lab_event: LabEvent,
        radio: Radio,
        is_active_in: Box<dyn Fn(&ControlState) -> bool>,
    },
    FloatParameter {
        up_code: SmolStr,
        down_code: SmolStr,
        description: String,
        physics_parameter: PhysicsParameter,
        radio: Radio,
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
        self.single_action(
            KeyCode::Escape,
            "ESC to cancel selection",
            Crucible(ToViewing),
            Box::new(|state| matches!(state, ShowingJoint(_) | ShowingInterval(_))),
        );
        self.single_action(
            KeyCode::Space,
            "Space to stop animation",
            Crucible(ToViewing),
            Box::new(|state| matches!(state, Animating)),
        );
        self.single_action(
            KeyCode::Space,
            "Space to start animation",
            Crucible(ToAnimating),
            Box::new(|state| matches!(state, Viewing)),
        );
        self.float_parameter(
            "G",
            "g",
            PhysicsParameter {
                feature: PhysicsFeature::Gravity,
                value: 0.0,
            },
            "Gravity",
            Box::new(|state| matches!(state, PhysicsTesting(_))),
        );
        self.float_parameter(
            "T",
            "t",
            PhysicsParameter {
                feature: PhysicsFeature::IterationsPerFrame,
                value: 100.0,
            },
            "Speed",
            Box::new(|state| matches!(state, PhysicsTesting(_))),
        );
        // self.single_action(
        //     KeyCode::ArrowUp,
        //     "\u{2191} faster",
        //     Crucible(AdjustSpeed(1.1)),
        //     Box::new(|state| !matches!(state, ShowingJoint(_) | ShowingInterval(_))),
        // );
        // self.single_action(
        //     KeyCode::ArrowDown,
        //     "\u{2193} slower",
        //     Crucible(AdjustSpeed(0.9)),
        //     Box::new(|state| !matches!(state, ShowingJoint(_) | ShowingInterval(_))),
        // );
        self.single_action(
            KeyCode::ArrowLeft,
            "\u{2190} previous test",
            Crucible(FailureTesterDo(FailureTesterAction::PrevExperiment)),
            Box::new(|state| matches!(state, FailureTesting(_))),
        );
        self.single_action(
            KeyCode::ArrowRight,
            "\u{2192} next test",
            Crucible(FailureTesterDo(FailureTesterAction::NextExperiment)),
            Box::new(|state| matches!(state, FailureTesting(_))),
        );
        self.single_action(
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
                        KeyAction::SingleKey {
                            code,
                            is_active_in,
                            radio,
                            lab_event,
                            ..
                        } => {
                            if *code == pressed_key && is_active_in(control_state) {
                                lab_event.clone().send(&radio);
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
                KeyAction::SingleKey {
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
                    description,
                    physics_parameter: PhysicsParameter { feature, value, .. },
                    ..
                } => {
                    if is_active_in(control_state) && !description.is_empty() {
                        legend.push(format!("{:?} {:.10}", feature, value));
                    }
                }
            }
        }
        legend
    }

    fn single_action(
        &mut self,
        code: KeyCode,
        description: &str,
        lab_event: LabEvent,
        is_active_in: Box<dyn Fn(&ControlState) -> bool>,
    ) {
        self.actions.push(KeyAction::SingleKey {
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
        description: &str,
        is_active_in: Box<dyn Fn(&ControlState) -> bool>,
    ) {
        self.actions.push(KeyAction::FloatParameter {
            up_code: up_code.into(),
            down_code: down_code.into(),
            description: description.into(),
            is_active_in,
            physics_parameter,
            radio: self.radio.clone(),
        })
    }
}
