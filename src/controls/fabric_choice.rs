use iced::{Alignment, Length};
use iced_wgpu::Renderer;
use iced_winit::Color;
use iced_winit::widget::{Button, Row, Text};
use crate::build::tenscript::fabric_plan;
use crate::controls::{Action, Message};

#[derive(Clone, Debug)]
pub enum FabricChoiceMessage {
    ChooseFabric(String),
}

impl From<FabricChoiceMessage> for Message {
    fn from(value: FabricChoiceMessage) -> Self {
        Message::FabricChoice(value)
    }
}

#[derive(Clone, Debug)]
pub struct FabricChoiceState {
    pub current: Option<String>,
    pub choices: Vec<(String, String)>,
}

impl FabricChoiceState {
    pub fn update(&mut self, message: FabricChoiceMessage) -> Option<Action> {
        match message {
            FabricChoiceMessage::ChooseFabric(choice) => {
                self.current = Some(choice.clone());
                Some(Action::BuildFabric(fabric_plan(&choice)))
            }
        }
    }

    pub fn view(&self) -> Row<'_, Message, Renderer> {
        let mut row = Row::new()
            .padding(5)
            .spacing(10)
            .width(Length::Fill)
            .align_items(Alignment::End);
        for (choice, _) in &self.choices {
            row = row.push(
                Button::new(Text::new(choice)
                    .style(
                        match &self.current {
                            None => {
                                Color::WHITE
                            }
                            Some(current) => {
                                if choice == current {
                                    Color::WHITE
                                } else {
                                    Color::from_rgb(0.0, 1.0, 0.0)
                                }
                            }
                        }
                    )
                )
                    .on_press(FabricChoiceMessage::ChooseFabric(choice.clone()).into())
            );
        }
        row
    }
}