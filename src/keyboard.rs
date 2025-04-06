use crate::messages::{
    ControlState, CrucibleAction, FailureTesterAction, LabEvent, PhysicsTesterAction, Radio,
};
use std::fmt::Display;
use winit::event::KeyEvent;
use winit::keyboard::{KeyCode, PhysicalKey};

struct KeyAction {
    code: KeyCode,
    description: String,
    lab_event: LabEvent,
    radio: Radio,
    is_active_in: Box<dyn Fn(&ControlState) -> bool>,
}

impl KeyAction {
    pub fn execute(&self) {
        self.lab_event.clone().send(&self.radio);
    }
}

impl Display for KeyAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description)
    }
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
        self.add_action(
            KeyCode::Escape,
            "ESC to cancel selection",
            Crucible(ToViewing),
            Box::new(|state| matches!(state, ShowingJoint(_) | ShowingInterval(_))),
        );
        self.add_action(
            KeyCode::Space,
            "Space to stop animation",
            Crucible(ToViewing),
            Box::new(|state| matches!(state, Animating)),
        );
        self.add_action(
            KeyCode::Space,
            "Space to start animation",
            Crucible(ViewingToAnimating),
            Box::new(|state| matches!(state, Viewing)),
        );
        self.add_action(
            KeyCode::ArrowUp,
            "\u{2191} faster",
            Crucible(AdjustSpeed(1.1)),
            Box::new(|state| !matches!(state, ShowingJoint(_) | ShowingInterval(_))),
        );
        self.add_action(
            KeyCode::ArrowDown,
            "\u{2193} slower",
            Crucible(AdjustSpeed(0.9)),
            Box::new(|state| !matches!(state, ShowingJoint(_) | ShowingInterval(_))),
        );
        self.add_action(
            KeyCode::ArrowLeft,
            "\u{2190} previous test",
            Crucible(FailureTesterDo(FailureTesterAction::PrevExperiment)),
            Box::new(|state| matches!(state, FailureTesting(_))),
        );
        self.add_action(
            KeyCode::ArrowRight,
            "\u{2192} next test",
            Crucible(FailureTesterDo(FailureTesterAction::NextExperiment)),
            Box::new(|state| matches!(state, FailureTesting(_))),
        );
        self.add_action(
            KeyCode::ArrowLeft,
            "\u{2190} previous test",
            Crucible(PhysicsTesterDo(PhysicsTesterAction::PrevExperiment)),
            Box::new(|state| matches!(state, PhysicsTesting(_))),
        );
        self.add_action(
            KeyCode::ArrowRight,
            "\u{2192} next test",
            Crucible(PhysicsTesterDo(PhysicsTesterAction::NextExperiment)),
            Box::new(|state| matches!(state, PhysicsTesting(_))),
        );
        self.add_action(
            KeyCode::KeyX,
            "", // hidden
            DumpCSV,
            Box::new(|state| matches!(state, Viewing)),
        );
        self
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent, control_state: &ControlState) {
        if key_event.state.is_pressed() {
            if let KeyEvent {
                physical_key: PhysicalKey::Code(code),
                ..
            } = key_event
            {
                self.actions
                    .iter()
                    .filter(|action| action.code == code && (action.is_active_in)(control_state))
                    .for_each(|action| action.execute());
            }
        }
    }

    pub fn legend(&self, control_state: &ControlState) -> Vec<String> {
        self.actions
            .iter()
            .filter(|action| (action.is_active_in)(control_state) && !action.description.is_empty())
            .map(|action| action.description.clone())
            .collect()
    }

    fn add_action(
        &mut self,
        code: KeyCode,
        description: &str,
        lab_event: LabEvent,
        is_active_in: Box<dyn Fn(&ControlState) -> bool>,
    ) {
        self.actions.push(KeyAction {
            code,
            description: description.into(),
            lab_event,
            radio: self.radio.clone(),
            is_active_in,
        });
    }
}
