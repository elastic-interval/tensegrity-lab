use iced_wgpu::Renderer;
use iced_winit::{Color, Element};
use iced_winit::widget::{Row, Slider, Text};
use crate::controls::{Action, Component, Message, format_row};
use crate::controls::gravity::GravityMessage::{*};

#[derive(Debug, Clone)]
pub enum GravityMessage {
    NuanceChanged(f32),
    Reset,
}

impl From<GravityMessage> for Message {
    fn from(value: GravityMessage) -> Self {
        Message::Gravity(value)
    }
}

#[derive(Clone, Debug)]
pub struct Gravity {
    pub nuance: f32,
    pub min_gravity: f32,
    pub max_gravity: f32,
}

impl Component for Gravity {
    type LocalMessage = GravityMessage;

    fn update(&mut self, message: Self::LocalMessage) -> Option<Action> {
        match message {
            NuanceChanged(nuance) => {
                self.nuance = nuance;
                Some(Action::GravityChanged(self.min_gravity * (1.0 - nuance) + self.max_gravity * nuance))
            }
            Reset => {
                self.nuance = 0.0;
                Some(Action::GravityChanged(self.min_gravity))
            }
        }
    }

    fn element(&self) -> Element<'_, Message, Renderer> {
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
