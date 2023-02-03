use iced_wgpu::Renderer;
use iced_winit::{Color, Element};
use iced_winit::widget::{Button, Row, Slider, Text};
use crate::controls::{Action, Component, ControlMessage, format_row};
use crate::controls::strain_threshold::StrainThresholdMessage::{*};

#[derive(Debug, Clone)]
pub enum StrainThresholdMessage {
    SetStrainLimits((f32, f32)),
    NuanceChanged(f32),
    Calibrate,
    Shorten,
}

impl From<StrainThresholdMessage> for ControlMessage {
    fn from(value: StrainThresholdMessage) -> Self {
        ControlMessage::StrainThreshold(value)
    }
}

#[derive(Clone, Debug)]
pub struct StrainThreshold {
    pub nuance: f32,
    pub strain_limits: (f32, f32),
}

impl StrainThreshold {
    pub fn strain_threshold(&self) -> f32 {
        let (min_strain, max_strain) = self.strain_limits;
        min_strain * (1.0 - self.nuance) + max_strain * self.nuance
    }
}

impl Component for StrainThreshold {
    type Message = StrainThresholdMessage;

    fn update(&mut self, message: StrainThresholdMessage) -> Option<Action> {
        match message {
            NuanceChanged(nuance) => self.nuance = nuance,
            SetStrainLimits(limits) => self.strain_limits = limits,
            Calibrate => {
                return Some(Action::CalibrateStrain);
            }
            Shorten => {
                return Some(Action::ShortenPulls(self.strain_threshold()));
            }
        }
        None
    }

    fn element(&self) -> Element<'_, ControlMessage, Renderer> {
        let (min_strain, max_strain) = self.strain_limits;
        let threshold = self.strain_threshold();
        format_row(
            Row::new()
                .push(
                    Text::new(format!("Strain threshold [{threshold:.04}]"))
                        .style(Color::WHITE)
                )
                .push(
                    Text::new(format!("{min_strain:.04}"))
                        .style(Color::WHITE)
                )
                .push(
                    Slider::new(0.0..=1.0, self.nuance, |value| NuanceChanged(value).into())
                        .step(0.01)
                )
                .push(
                    Text::new(format!("{max_strain:.04}"))
                        .style(Color::WHITE)
                )
                .push(
                    Button::new(Text::new("Calibrate"))
                        .on_press(Calibrate.into())
                )
                .push(
                    Button::new(Text::new("Shorten"))
                        .on_press(Shorten.into())
                )
        )
    }
}
