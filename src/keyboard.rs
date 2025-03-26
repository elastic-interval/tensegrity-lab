use crate::application::AppStateChange;
use crate::crucible::{CrucibleAction, TesterAction};
use crate::messages::{ControlState, LabEvent};
use std::fmt::Display;
use winit::event::KeyEvent;
use winit::event_loop::EventLoopProxy;
use winit::keyboard::{KeyCode, PhysicalKey};

struct KeyAction {
    code: KeyCode,
    description: String,
    lab_event: LabEvent,
    event_loop_proxy: EventLoopProxy<LabEvent>,
    is_active_in: Box<dyn Fn(&ControlState) -> bool>,
}

impl KeyAction {
    pub fn execute(&self) {
        self.event_loop_proxy
            .send_event(self.lab_event.clone())
            .unwrap();
    }
}

impl Display for KeyAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description)
    }
}

pub struct Keyboard {
    event_loop_proxy: EventLoopProxy<LabEvent>,
    actions: Vec<KeyAction>,
}

impl Keyboard {
    pub fn new(event_loop_proxy: EventLoopProxy<LabEvent>) -> Self {
        Self {
            event_loop_proxy,
            actions: Default::default(),
        }
    }

    pub fn with_actions(mut self) -> Self {
        self.add_action(
            KeyCode::Escape,
            "ESC to reset",
            LabEvent::AppStateChanged(AppStateChange::SetControlState(ControlState::Animating)),
            Box::new(|state| {
                matches!(
                    state,
                    ControlState::ShowingJoint(_) | ControlState::ShowingInterval(_)
                )
            }),
        );
        self.add_action(
            KeyCode::KeyX,
            "X to export CSV",
            LabEvent::DumpCSV,
            Box::new(|state| matches!(state, ControlState::Viewing)),
        );
        self.add_action(
            KeyCode::Space,
            "Space to stop animation",
            LabEvent::Crucible(CrucibleAction::StopAnimating),
            Box::new(|state| matches!(state, ControlState::Animating)),
        );
        self.add_action(
            KeyCode::Space,
            "Space to start animation",
            LabEvent::Crucible(CrucibleAction::StartAnimating),
            Box::new(|state| matches!(state, ControlState::Viewing)),
        );
        self.add_action(
            KeyCode::ArrowUp,
            "\u{2191} faster",
            LabEvent::Crucible(CrucibleAction::SetSpeed(1.1)),
            Box::new(|_| true),
        );
        self.add_action(
            KeyCode::ArrowDown,
            "\u{2193} slower",
            LabEvent::Crucible(CrucibleAction::SetSpeed(0.9)),
            Box::new(|_| true),
        );
        self.add_action(
            KeyCode::ArrowLeft,
            "\u{2190} previous test",
            LabEvent::Crucible(CrucibleAction::TesterDo(TesterAction::NextExperiment(false))),
            Box::new(|state| matches!(state, ControlState::Testing(_))),
        );
        self.add_action(
            KeyCode::ArrowRight,
            "\u{2192} next test",
            LabEvent::Crucible(CrucibleAction::TesterDo(TesterAction::NextExperiment(true))),
            Box::new(|state| matches!(state, ControlState::Testing(_))),
        );
        self.add_action(
            KeyCode::KeyT,
            "T test tension",
            LabEvent::Crucible(CrucibleAction::StartExperiment(true)),
            Box::new(|state| matches!(state, ControlState::Viewing)),
        );
        self.add_action(
            KeyCode::KeyC,
            "C test compression",
            LabEvent::Crucible(CrucibleAction::StartExperiment(false)),
            Box::new(|state| matches!(state, ControlState::Viewing)),
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
            .filter(|action| (action.is_active_in)(control_state))
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
            event_loop_proxy: self.event_loop_proxy.clone(),
            is_active_in,
        });
    }
}
