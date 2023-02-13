use iced_wgpu::Renderer;
use iced_winit::Element;
use iced_winit::widget::{Button, Row, Text};

use crate::build::tenscript::FabricPlan;
use crate::user_interface::Action;
use crate::user_interface::control_state::{Component, ControlMessage, format_row};

#[derive(Clone, Debug)]
pub enum FabricChoiceMessage {
    ChooseFabric(FabricPlan),
}

impl From<FabricChoiceMessage> for ControlMessage {
    fn from(value: FabricChoiceMessage) -> Self {
        ControlMessage::FabricChoice(value)
    }
}

#[derive(Clone, Debug)]
pub struct FabricChoice {
    pub choices: Vec<(String, FabricPlan)>,
}

impl Component for FabricChoice {
    type Message = FabricChoiceMessage;

    fn update(&mut self, message: Self::Message) -> Option<Action> {
        match message {
            FabricChoiceMessage::ChooseFabric(fabric_plan) =>
                Some(Action::BuildFabric(fabric_plan)),
        }
    }

    fn element(&self) -> Element<'_, ControlMessage, Renderer> {
        let mut row = Row::new();
        for (name, plan) in &self.choices {
            row = row.push(
                Button::new(Text::new(name))
                    .on_press(FabricChoiceMessage::ChooseFabric(plan.clone()).into())
            );
        }
        format_row(row)
    }
}