use iced_wgpu::Renderer;
use iced_winit::Element;
use iced_winit::widget::{Button, Row, Text};
use crate::build::tenscript::fabric_plan;
use crate::controls::{Action, Component, Message, format_row};

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
pub struct FabricChoice {
    pub current: Option<String>,
    pub choices: Vec<(String, String)>,
}

impl Component for FabricChoice {
    type LocalMessage = FabricChoiceMessage;

    fn update(&mut self, message: Self::LocalMessage) -> Option<Action> {
        match message {
            FabricChoiceMessage::ChooseFabric(choice) => {
                self.current = Some(choice.clone());
                Some(Action::BuildFabric(fabric_plan(&choice)))
            }
        }
    }

    fn element(&self) -> Element<'_, Message, Renderer> {
        let mut row = Row::new();
        for (choice, _) in &self.choices {
            row = row.push(
                Button::new(Text::new(choice))
                    .on_press(FabricChoiceMessage::ChooseFabric(choice.clone()).into())
            );
        };
        format_row(row)
    }
}