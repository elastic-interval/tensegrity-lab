use std::cell::RefCell;

use iced_wgpu::Renderer;
use iced_winit::{Alignment, Color, Command, Element, Length, Program};
use iced_winit::widget::{Column, Row, Text};

#[cfg(target_arch = "wasm32")]
use instant::Instant;

use crate::fabric::{Fabric, UniqueId};
use crate::scene::SceneVariant;
use crate::scene::SceneVariant::{Suspended, TinkeringOnFace};
use crate::user_interface::{Action, ControlMessage};
use crate::user_interface::gravity::{Gravity, GravityMessage};
use crate::user_interface::keyboard::Keyboard;
use crate::user_interface::strain_threshold::StrainThreshold;
use crate::user_interface::strain_threshold::StrainThresholdMessage::SetStrainLimits;

#[derive(Clone, Copy, Debug)]
pub enum VisibleControl {
    Nothing,
    Gravity,
    StrainThreshold,
}

#[derive(Debug)]
pub struct ControlState {
    debug_mode: bool,
    keyboard: Keyboard,
    visible_controls: VisibleControl,
    strain_threshold: StrainThreshold,
    gravity: Gravity,
    show_strain: bool,
    frame_rate: f64,
    action_queue: RefCell<Vec<Action>>,
}

impl Default for ControlState {
    fn default() -> Self {
        Self {
            keyboard: Keyboard::default(),
            debug_mode: false,
            visible_controls: VisibleControl::Nothing,
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

    pub fn variation(&self, face_id: Option<UniqueId>) -> SceneVariant {
        if self.show_strain {
            SceneVariant::ShowingStrain {
                threshold: self.strain_threshold.strain_threshold(),
                material: Fabric::BOW_TIE_MATERIAL_INDEX,
            }
        } else {
            match face_id {
                None => Suspended,
                Some(face_id) => TinkeringOnFace(face_id),
            }
        }
    }

    pub fn strain_limits_changed(&self, limits: (f32, f32)) -> ControlMessage {
        SetStrainLimits(limits).into()
    }
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
                self.visible_controls = VisibleControl::Nothing;
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
            ControlMessage::Keyboard(message) => {
                queue_action(self.keyboard.update(message));
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
                        VisibleControl::Nothing => Row::new().into(),
                        VisibleControl::StrainThreshold => self.strain_threshold.element(),
                        VisibleControl::Gravity => self.gravity.element(),
                    }
                )
                .push(
                    Row::new()
                        .width(Length::Fill)
                        .align_items(Alignment::Center)
                        .push(self.keyboard.element())
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
        .padding(10)
        .spacing(20)
        .width(Length::Fill)
        .align_items(Alignment::Center)
        .into()
}
