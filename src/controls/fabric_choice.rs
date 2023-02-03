use iced_wgpu::Renderer;
use iced_winit::widget::{Button, Row, Text};

use crate::build::tenscript::FabricPlan;
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
                Some(Action::BuildFabric(FabricPlan::from_bootstrap(&choice)))
            }
        }
    }

    pub fn row(&self) -> Row<'_, Message, Renderer> {
        let mut row = Row::new();
        for (choice, _) in &self.choices {
            row = row.push(
                Button::new(Text::new(choice))
                    .on_press(FabricChoiceMessage::ChooseFabric(choice.clone()).into())
            );
        };
        row
    }
}