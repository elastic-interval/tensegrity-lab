use iced_wgpu::Renderer;
use iced_winit::{Color, Element};
use iced_winit::widget::{Row, Slider, Text};
use crate::crucible::{CrucibleAction, LabAction};
use crate::user_interface::{Action, ControlMessage};
use crate::user_interface::control_state::{Component, format_row};
use crate::user_interface::gravity::GravityMessage::{*};

#[derive(Debug, Clone)]
pub enum GravityMessage {
    NuanceChanged(f32),
    Reset,
}

impl From<GravityMessage> for ControlMessage {
    fn from(value: GravityMessage) -> Self {
        ControlMessage::Gravity(value)
    }
}

#[derive(Clone, Debug)]
pub struct Gravity {
    nuance: f32,
    default: f32,
    min_gravity: f32,
    max_gravity: f32,
}

impl Gravity {
    pub fn new(default: f32) -> Self {
        let min_gravity = default * 0.1;
        let max_gravity = default * 5.0;
        let nuance = (default - min_gravity) / (max_gravity - min_gravity);
        Self {
            nuance,
            default,
            min_gravity,
            max_gravity,
        }
    }
}

impl Component for Gravity {
    type Message = GravityMessage;

    fn update(&mut self, message: Self::Message) -> Option<Action> {
        let gravity = match message {
            NuanceChanged(nuance) => {
                self.nuance = nuance;
                self.min_gravity * (1.0 - nuance) + self.max_gravity * nuance
            }
            Reset => {
                self.nuance = (self.default - self.min_gravity) / (self.max_gravity - self.min_gravity);
                self.default
            }
        };
        Some(Action::Crucible(CrucibleAction::Experiment(LabAction::GravityChanged(gravity))))
    }

    fn element(&self) -> Element<'_, ControlMessage, Renderer> {
        format_row(
            Row::new()
                .padding(20)
                .spacing(20)
                .push(
                    Text::new("Gravity").style(Color::WHITE)
                )
                .push(
                    Slider::new(0.0..=1.0, self.nuance, |value| NuanceChanged(value).into())
                        .step(0.01)
                )
        )
    }
}
