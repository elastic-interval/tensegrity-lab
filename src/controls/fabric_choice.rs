use iced_wgpu::Renderer;
use iced_winit::Element;
use iced_winit::widget::{Button, Row, Text};

use crate::build::tenscript::FabricPlan;
use crate::controls::{Action, Component, ControlMessage, format_row};

#[derive(Clone, Debug)]
pub enum FabricChoiceMessage {
    ChooseFabric(String),
}

impl From<FabricChoiceMessage> for ControlMessage {
    fn from(value: FabricChoiceMessage) -> Self {
        ControlMessage::FabricChoice(value)
    }
}

#[derive(Clone, Debug)]
pub struct FabricChoice {
    pub current: Option<String>,
    pub choices: Vec<(String, FabricPlan)>,
}

impl Component for FabricChoice {
    type Message = FabricChoiceMessage;

    fn update(&mut self, message: Self::Message) -> Option<Action> {
        match message {
            FabricChoiceMessage::ChooseFabric(choice) => {
                self.current = Some(choice.clone());
                Some(Action::BuildFabric(FabricPlan::from_bootstrap(&choice).expect("no such fabric plan")))
            }
        }
    }

    fn element(&self) -> Element<'_, ControlMessage, Renderer> {
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