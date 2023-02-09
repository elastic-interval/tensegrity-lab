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
    pub choices: Vec<String>,
}

impl Component for FabricChoice {
    type Message = FabricChoiceMessage;

    fn update(&mut self, message: Self::Message) -> Option<Action> {
        match message {
            FabricChoiceMessage::ChooseFabric(plan_name) => {
                let fabric_plan = FabricPlan::preset_with_name(&plan_name).expect("no such fabric");
                Some(Action::BuildFabric(fabric_plan))
            }
        }
    }

    fn element(&self) -> Element<'_, ControlMessage, Renderer> {
        let mut row = Row::new();
        for choice in &self.choices {
            row = row.push(
                Button::new(Text::new(choice.clone()))
                    .on_press(FabricChoiceMessage::ChooseFabric(choice.clone()).into())
            );
        };
        format_row(row)
    }
}