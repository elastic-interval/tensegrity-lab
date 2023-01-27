use iced_wgpu::Renderer;
use iced_winit::Color;
use iced_winit::widget::{Button, Row, Slider, Text};
use crate::controls::{Action, Message};
use crate::controls::strain_control::StrainControlMessage::{*};

#[derive(Debug, Clone)]
pub enum StrainControlMessage {
    StrainThresholdChanged(f32),
    MeasureNuanceChanged(f32),
    AddPulls,
}

impl From<StrainControlMessage> for Message {
    fn from(value: StrainControlMessage) -> Self {
        Message::StrainControl(value)
    }
}

#[derive(Clone, Debug)]
pub struct StrainControl {
    pub strain_nuance: f32,
    pub strain_threshold: f32,
}

impl StrainControl {
    pub fn update(&mut self, message: StrainControlMessage) -> Option<Action> {
        match message {
            MeasureNuanceChanged(nuance) => {
                self.strain_nuance = nuance;
            }
            StrainThresholdChanged(limit) => {
                self.strain_threshold = limit;
            }
            AddPulls => {
                return Some(Action::AddPulls { strain_nuance: self.strain_nuance });
            }
        }
        None
    }

    pub fn view(&self) -> Row<'_, Message, Renderer> {
        let strain_limit = self.strain_threshold;
        Row::new()
            .padding(20)
            .spacing(20)
            .push(
                Text::new("Strain threshold")
                    .style(Color::WHITE)
            )
            .push(
                Slider::new(0.0..=1.0, self.strain_nuance, |value| MeasureNuanceChanged(value).into())
                    .step(0.01)
            )
            .push(
                Text::new(format!("{strain_limit:.05}"))
                    .style(Color::WHITE)
            )
            .push(
                Button::new(Text::new("Add Pulls"))
                    .on_press(AddPulls.into())
            )
    }
}
