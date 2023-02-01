use iced_wgpu::Renderer;
use iced_winit::Color;
use iced_winit::widget::{Button, Row, Slider, Text};
use crate::controls::{Action, Message};
use crate::controls::strain_threshold::StrainThresholdMessage::{*};

#[derive(Debug, Clone)]
pub enum StrainThresholdMessage {
    SetStrainLimits((f32, f32)),
    NuanceChanged(f32),
    Calibrate,
}

impl From<StrainThresholdMessage> for Message {
    fn from(value: StrainThresholdMessage) -> Self {
        Message::StrainThreshold(value)
    }
}

#[derive(Clone, Debug)]
pub struct StrainThresholdState {
    pub nuance: f32,
    pub strain_limits: (f32, f32),
}

impl StrainThresholdState {
    pub fn strain_threshold(&self) -> f32 {
        let (min_strain, max_strain) = self.strain_limits;
        min_strain * (1.0 - self.nuance) + max_strain * self.nuance
    }

    pub fn update(&mut self, message: StrainThresholdMessage) -> Option<Action> {
        match message {
            NuanceChanged(nuance) => {
                self.nuance = nuance;
            }
            SetStrainLimits(limits) => {
                self.strain_limits = limits;
            }
            Calibrate => {
                return Some(Action::CalibrateStrain);
            }
        }
        None
    }

    pub fn row(&self) -> Row<'_, Message, Renderer> {
        let (min_strain, max_strain) = self.strain_limits;
        Row::new()
            .push(
                Text::new("Strain threshold")
                    .style(Color::WHITE)
            )
            .push(
                Text::new(format!("{min_strain:.05}"))
                    .style(Color::WHITE)
            )
            .push(
                Slider::new(0.0..=1.0, self.nuance, |value| NuanceChanged(value).into())
                    .step(0.01)
            )
            .push(
                Text::new(format!("{max_strain:.05}"))
                    .style(Color::WHITE)
            )
            .push(
                Button::new(Text::new("Calibrate"))
                    .on_press(Calibrate.into())
            )
    }
}
