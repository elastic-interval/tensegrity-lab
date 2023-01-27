use iced_wgpu::Renderer;
use iced_winit::Color;
use iced_winit::widget::{Button,  Row, Slider, Text};
use crate::controls::{Action, Message};

#[derive(Clone, Debug)]
pub struct StrainControl {
    pub strain_nuance: f32,
    pub strain_threshold: f32,
}

impl StrainControl {
    pub fn update(&mut self, message: Message) -> Option<Action> {
        match message {
            Message::MeasureNuanceChanged(nuance) => {
                self.strain_nuance = nuance;
            }
            Message::StrainThreshold(limit) => {
                self.strain_threshold = limit;
            }
            Message::AddPulls => {
                return Some(Action::AddPulls { strain_nuance: self.strain_nuance });
            }
            _ => {}
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
                Slider::new(0.0..=1.0, self.strain_nuance, Message::MeasureNuanceChanged)
                    .step(0.01)
            )
            .push(
                Text::new(format!("{strain_limit:.05}"))
                    .style(Color::WHITE)
            )
            .push(
                Button::new(Text::new("Add Pulls"))
                    .on_press(Message::AddPulls)
            )
    }
}
