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
    pub nuance: f32,
    pub min_gravity: f32,
    pub max_gravity: f32,
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
                self.nuance = 0.0;
                self.min_gravity
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
