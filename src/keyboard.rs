use crate::application::AppStateChange;
use crate::crucible::{CrucibleAction, LabAction};
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
            LabEvent::AppStateChanged(AppStateChange::SetControlState(ControlState::Viewing)),
        );
        self.add_action(KeyCode::KeyX, "X to export CSV", LabEvent::DumpCSV);
        self.add_action(
            KeyCode::Space,
            "Space to toggle animation",
            LabEvent::Crucible(CrucibleAction::Experiment(LabAction::ToggleMusclesActive)),
        );
        self.add_action(
            KeyCode::ArrowUp,
            "Up to speed up",
            LabEvent::Crucible(CrucibleAction::SetSpeed(1.1)),
        );
        self.add_action(
            KeyCode::ArrowDown,
            "Down to slow down",
            LabEvent::Crucible(CrucibleAction::SetSpeed(0.9)),
        );
        self.add_action(
            KeyCode::ArrowLeft,
            "Left for previous",
            LabEvent::Crucible(CrucibleAction::Experiment(LabAction::NextExperiment(false))),
        );
        self.add_action(
            KeyCode::ArrowRight,
            "Right for next",
            LabEvent::Crucible(CrucibleAction::Experiment(LabAction::NextExperiment(true))),
        );
        self
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        if key_event.state.is_pressed() {
            if let KeyEvent {
                physical_key: PhysicalKey::Code(code),
                ..
            } = key_event
            {
                self.actions
                    .iter()
                    .filter(|action| action.code == code)
                    .for_each(|action| action.execute());
            }
        }
    }

    pub fn legend(&self) -> Vec<String> {
        self.actions.iter().map(|action| action.description.clone()).collect()
    }

    fn add_action(&mut self, code: KeyCode, description: &str, lab_event: LabEvent) {
        self.actions.push(KeyAction {
            code,
            description: description.into(),
            lab_event,
            event_loop_proxy: self.event_loop_proxy.clone(),
        });
    }
}
