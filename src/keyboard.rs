use crate::application::AppStateChange;
use crate::crucible::{CrucibleAction, LabAction};
use crate::messages::{ControlState, LabEvent};
use winit::event::KeyEvent;
use winit::event_loop::EventLoopProxy;
use winit::keyboard::{KeyCode, PhysicalKey};

pub struct Keyboard {
    muscles_active: bool,
    event_loop_proxy: EventLoopProxy<LabEvent>,
}

impl Keyboard {
    pub fn new(event_loop_proxy: EventLoopProxy<LabEvent>) -> Self {
        Self {
            muscles_active: false,
            event_loop_proxy,
        }
    }

    pub(crate) fn handle_key_event(&mut self, key_event: KeyEvent) {
        if !key_event.state.is_pressed() {
            return;
        }
        if let KeyEvent {
            physical_key: PhysicalKey::Code(code),
            ..
        } = key_event
        {
            let send = |lab_event: LabEvent| {
                self.event_loop_proxy.send_event(lab_event).unwrap();
            };
            match code {
                KeyCode::KeyX => {
                    send(LabEvent::DumpCSV)
                }
                KeyCode::Space => {
                    self.muscles_active = !self.muscles_active;
                    send(LabEvent::Crucible(CrucibleAction::Experiment(
                        LabAction::MusclesActive(self.muscles_active),
                    )));
                    send(LabEvent::AppStateChanged(AppStateChange::SetMusclesActive(
                        self.muscles_active,
                    )));
                }
                KeyCode::Escape => {
                    send(LabEvent::AppStateChanged(AppStateChange::SetControlState(
                        ControlState::Viewing,
                    )));
                }
                KeyCode::ArrowUp => {
                    send(LabEvent::Crucible(CrucibleAction::SetSpeed(1.1)));
                }
                KeyCode::ArrowDown => {
                    send(LabEvent::Crucible(CrucibleAction::SetSpeed(0.9)));
                }
                KeyCode::ArrowLeft | KeyCode::ArrowRight => {
                    send(LabEvent::Crucible(CrucibleAction::Experiment(
                        LabAction::NextExperiment(code == KeyCode::ArrowRight),
                    )));
                }
                _ => {}
            }
            if code == KeyCode::ArrowRight || code == KeyCode::ArrowLeft {}
        }
    }
}
