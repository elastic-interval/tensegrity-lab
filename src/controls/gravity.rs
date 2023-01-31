use iced_wgpu::Renderer;
use iced_winit::Color;
use iced_winit::widget::{Row, Slider, Text};
use crate::controls::{Action, Message};
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
pub struct GravityState {
    pub nuance: f32,
    pub min_gravity: f32,
    pub max_gravity: f32,
}

impl GravityState {
    pub fn update(&mut self, message: GravityMessage) -> Option<Action> {
        match message {
            NuanceChanged(nuance) => {
                self.nuance = nuance;
                Some(Action::GravityChanged(self.min_gravity * (1.0 - nuance) + self.max_gravity * nuance))
            },
            Reset => {
                self.nuance = 0.0;
                Some(Action::GravityChanged(self.min_gravity))
            }
        }
    }

    pub fn row(&self) -> Row<'_, Message, Renderer> {
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
    }
}
