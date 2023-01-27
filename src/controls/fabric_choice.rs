use hashbrown::HashMap;
use iced::{Alignment, Length};
use iced_wgpu::Renderer;
use iced_winit::Color;
use iced_winit::widget::{Button, Row, Text};
use crate::controls::{Action, Message};

#[derive(Clone, Debug)]
pub enum FabricChoiceMessage {
    ChooseFabric(&'static str),
}

impl From<FabricChoiceMessage> for Message {
    fn from(value: FabricChoiceMessage) -> Self {
        Message::FabricChoice(value)
    }
}

#[derive(Clone, Debug)]
pub struct FabricChoiceState {
    pub current: &'static str,
    pub choices: HashMap<&'static str, &'static str>,
}

impl FabricChoiceState {
    pub fn update(&mut self, message: FabricChoiceMessage) -> Option<Action> {
        match message {
            FabricChoiceMessage::ChooseFabric(choice) => {
                self.current = choice;
            }
        }
        None
    }

    pub fn view(&self) -> Row<'_, Message, Renderer> {
        let mut row = Row::new()
            .padding(20)
            .spacing(20)
            .width(Length::Fill)
            .align_items(Alignment::Center);
        for &choice in self.choices.keys() {
            row = row.push(
                Button::new(Text::new(choice)
                    .style(if choice == self.current { Color::WHITE } else { Color::from_rgb(0.0, 1.0, 0.0) })
                )
            );
        }
        row
    }
}