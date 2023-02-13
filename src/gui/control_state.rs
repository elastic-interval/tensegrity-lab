use std::cell::RefCell;

use iced_wgpu::Renderer;
use iced_winit::{Alignment, Color, Command, Element, Length, Program};
use iced_winit::widget::{Button, Column, Row, Text};

#[cfg(target_arch = "wasm32")]
use instant::Instant;

use crate::build::tenscript::{FabricPlan, FaceAlias, Library};
use crate::fabric::{Fabric, UniqueId};
use crate::gui::fabric_choice::{FabricChoice, FabricChoiceMessage};
use crate::gui::gravity::{Gravity, GravityMessage};
use crate::gui::strain_threshold::{StrainThreshold, StrainThresholdMessage};
use crate::gui::strain_threshold::StrainThresholdMessage::SetStrainLimits;
use crate::scene::Variation;

#[derive(Clone, Copy, Debug)]
pub enum VisibleControl {
    ControlChoice,
    Gravity,
    FabricChoice,
    StrainThreshold,
}

#[derive(Clone, Debug)]
pub enum Action {
    BuildFabric(FabricPlan),
    SelectFace(UniqueId),
    AddBrick { face_alias: FaceAlias, face_id: UniqueId },
    GravityChanged(f32),
    ShowSurface,
    CalibrateStrain,
}

#[derive(Clone, Debug)]
pub struct ControlState {
    debug_mode: bool,
    visible_controls: VisibleControl,
    fabric_choice: FabricChoice,
    strain_threshold: StrainThreshold,
    gravity: Gravity,
    show_strain: bool,
    frame_rate: f64,
    action_queue: RefCell<Vec<Action>>,
}

impl Default for ControlState {
    fn default() -> Self {
        let choices = Library::standard()
            .fabrics
            .into_iter()
            .map(|plan| (plan.name.clone(), plan))
            .collect();
        Self {
            debug_mode: false,
            visible_controls: VisibleControl::FabricChoice,
            fabric_choice: FabricChoice {
                choices,
            },
            strain_threshold: StrainThreshold {
                nuance: 0.0,
                strain_limits: (0.0, 1.0),
            },
            gravity: Gravity {
                nuance: 0.0,
                min_gravity: 1e-8,
                max_gravity: 5e-7,
            },
            show_strain: false,
            frame_rate: 0.0,
            action_queue: RefCell::new(Vec::new()),
        }
    }
}

impl ControlState {
    pub fn take_actions(&self) -> Vec<Action> {
        self.action_queue.borrow_mut().split_off(0)
    }

    pub fn queue_action(&self, action: Action) {
        self.action_queue.borrow_mut().push(action);
    }

    pub fn show_strain(&self) -> bool {
        self.show_strain
    }

    pub fn variation(&self, face_id: Option<UniqueId>) -> Variation {
        if self.show_strain {
            Variation::StrainView {
                threshold: self.strain_threshold.strain_threshold(),
                material: Fabric::BOW_TIE_MATERIAL_INDEX,
            }
        } else {
            Variation::BuildView { face_id }
        }
    }

    pub fn strain_limits_changed(&self, limits: (f32, f32)) -> ControlMessage {
        SetStrainLimits(limits).into()
    }
}

#[derive(Debug, Clone)]
pub enum ControlMessage {
    ToggleDebugMode,
    Reset,
    ShowControl(VisibleControl),
    FabricChoice(FabricChoiceMessage),
    StrainThreshold(StrainThresholdMessage),
    Gravity(GravityMessage),
    Action(Action),
    FrameRateUpdated(f64),
}

impl Program for ControlState {
    type Renderer = Renderer;
    type Message = ControlMessage;

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        let queue_action = |action: Option<Action>| {
            if let Some(action) = action {
                self.action_queue.borrow_mut().push(action);
            }
        };
        match message {
            ControlMessage::ToggleDebugMode => {
                self.debug_mode = !self.debug_mode;
            }
            ControlMessage::Action(action) => {
                queue_action(Some(action));
            }
            ControlMessage::Reset => {
                self.visible_controls = VisibleControl::ControlChoice;
                self.gravity.update(GravityMessage::Reset);
            }
            ControlMessage::ShowControl(visible_control) => {
                self.visible_controls = visible_control;
                match visible_control {
                    VisibleControl::StrainThreshold => {
                        queue_action(Some(Action::CalibrateStrain));
                        self.show_strain = true;
                    }
                    _ => {
                        self.show_strain = false;
                    }
                }
            }
            ControlMessage::FabricChoice(message) => {
                queue_action(self.fabric_choice.update(message));
            }
            ControlMessage::StrainThreshold(message) => {
                queue_action(self.strain_threshold.update(message));
            }
            ControlMessage::Gravity(message) => {
                queue_action(self.gravity.update(message));
            }
            ControlMessage::FrameRateUpdated(frame_rate) => {
                self.frame_rate = frame_rate;
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, ControlMessage, Renderer> {
        let mut right_column = Column::new()
            .width(Length::Fill)
            .align_items(Alignment::End);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let Self { frame_rate, .. } = *self;
            right_column = right_column
                .push(
                    Text::new(format!("{frame_rate:.01} FPS"))
                        .style(Color::WHITE)
                );
        }
        let element: Element<'_, ControlMessage, Renderer> =
            Column::new()
                .padding(10)
                .height(Length::Fill)
                .align_items(Alignment::End)
                .push(
                    Row::new()
                        .height(Length::Fill)
                        .width(Length::Fill)
                        .push(right_column)
                )
                .push(
                    match self.visible_controls {
                        VisibleControl::ControlChoice => {
                            Row::new()
                                .push(Button::new(Text::new("Fabrics"))
                                    .on_press(ControlMessage::ShowControl(VisibleControl::FabricChoice)))
                                .push(Button::new(Text::new("Strain"))
                                    .on_press(ControlMessage::ShowControl(VisibleControl::StrainThreshold)))
                                .push(Button::new(Text::new("Gravity"))
                                    .on_press(ControlMessage::ShowControl(VisibleControl::Gravity)))
                                .into()
                        }
                        VisibleControl::FabricChoice => self.fabric_choice.element(),
                        VisibleControl::StrainThreshold => self.strain_threshold.element(),
                        VisibleControl::Gravity => self.gravity.element(),
                    }
                )
                .into();
        if self.debug_mode {
            element.explain(Color::WHITE)
        } else {
            element
        }
    }
}

pub trait Component {
    type Message: Into<ControlMessage>;
    fn update(&mut self, message: Self::Message) -> Option<Action>;
    fn element(&self) -> Element<'_, ControlMessage, Renderer>;
}

pub fn format_row(row: Row<'_, ControlMessage, Renderer>) -> Element<'_, ControlMessage, Renderer> {
    row
        .padding(5)
        .spacing(10)
        .width(Length::Fill)
        .align_items(Alignment::Center)
        .into()
}
