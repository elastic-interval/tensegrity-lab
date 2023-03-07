use iced_wgpu::Renderer;
use iced_winit::{Color, Element};
use iced_winit::widget::{Row, Slider, Text};
use crate::crucible::{CrucibleAction, LabAction};
use crate::user_interface::{Action, ControlMessage};
use crate::user_interface::control_state::{Component, format_row};

#[derive(Debug, Clone)]
pub enum MuscleMessage {
    NuanceChanged(f32),
    Reset,
}

impl From<MuscleMessage> for ControlMessage {
    fn from(value: MuscleMessage) -> Self {
        ControlMessage::Muscle(value)
    }
}

#[derive(Clone, Debug)]
pub struct Muscle {
    nuance: f32,
}

impl Muscle {
    pub fn new() -> Self {
        Self { nuance: 0.5 }
    }
}

impl Component for Muscle {
    type Message = MuscleMessage;

    fn update(&mut self, message: Self::Message) -> Option<Action> {
        match message {
            MuscleMessage::NuanceChanged(nuance) => {
                self.nuance = nuance;
            },
            MuscleMessage::Reset => {
                self.nuance = 0.5;
            }
        };
        Some(Action::Crucible(CrucibleAction::Experiment(LabAction::MuscleChanged(self.nuance))))
    }

    fn element(&self) -> Element<'_, ControlMessage, Renderer> {
        format_row(
            Row::new()
                .padding(20)
                .spacing(20)
                .push(
                    Text::new("Muscle").style(Color::WHITE)
                )
                .push(
                    Slider::new(0.0..=1.0, self.nuance,
                                |value| MuscleMessage::NuanceChanged(value).into())
                        .step(0.01)
                )
        )
    }
}
