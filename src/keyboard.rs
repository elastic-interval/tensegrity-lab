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
            LabEvent::AppStateChanged(AppStateChange::SetControlState(ControlState::Viewing)),
            Box::new(|control_state| match control_state {
                ControlState::ShowingJoint(_) | ControlState::ShowingInterval(_) => true,
                _ => false,
            }),
        );
        self.add_action(
            KeyCode::KeyX,
            "X to export CSV",
            LabEvent::DumpCSV,
            Box::new(|_control_state| true),
        );
        self.add_action(
            KeyCode::Space,
            "Space to toggle animation",
            LabEvent::Crucible(CrucibleAction::Experiment(LabAction::ToggleMusclesActive)),
            Box::new(|_control_state| true),
        );
        self.add_action(
            KeyCode::ArrowUp,
            "Up to speed up",
            LabEvent::Crucible(CrucibleAction::SetSpeed(1.1)),
            Box::new(|_control_state| true),
        );
        self.add_action(
            KeyCode::ArrowDown,
            "Down to slow down",
            LabEvent::Crucible(CrucibleAction::SetSpeed(0.9)),
            Box::new(|_control_state| true),
        );
        self.add_action(
            KeyCode::ArrowLeft,
            "Left for previous",
            LabEvent::Crucible(CrucibleAction::Experiment(LabAction::NextExperiment(false))),
            Box::new(|_control_state| true),
        );
        self.add_action(
            KeyCode::ArrowRight,
            "Right for next",
            LabEvent::Crucible(CrucibleAction::Experiment(LabAction::NextExperiment(true))),
            Box::new(|_control_state| true),
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
