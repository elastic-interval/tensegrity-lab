mod strain_threshold;
mod fabric_choice;
mod gravity;

use std::cell::RefCell;
use iced_wgpu::{Backend, Renderer, Settings};
use iced_winit::{Alignment, Clipboard, Color, Command, conversion, Debug, Element, Length, mouse, Program, program, renderer, Size, Viewport};
use iced_winit::widget::{Button, Column, Row, Text};
use wgpu::{CommandEncoder, Device, TextureView};
use winit::dpi::PhysicalPosition;
use winit::event::{ModifiersState, WindowEvent};
use winit::window::{CursorIcon, Window};

#[cfg(target_arch = "wasm32")]
use instant::Instant;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
use crate::build::tenscript::{bootstrap_fabric_plans, FabricPlan};
use crate::controls::fabric_choice::{FabricChoiceState, FabricChoiceMessage};
use crate::controls::gravity::{GravityMessage, GravityState};
use crate::controls::strain_threshold::{StrainThresholdState, StrainThresholdMessage};
use crate::controls::strain_threshold::StrainThresholdMessage::StrainThresholdChanged;

use crate::graphics::GraphicsWindow;

const FRAME_RATE_MEASURE_INTERVAL_SECS: f64 = 0.5;

/// Largely adapted from https://github.com/iced-rs/iced/blob/master/examples/integration_wgpu/src/main.rs
pub struct GUI {
    renderer: Renderer,
    debug: Debug,
    viewport: Viewport,
    staging_belt: wgpu::util::StagingBelt,
    state: program::State<ControlState>,
    cursor_position: PhysicalPosition<f64>,
    clipboard: Clipboard,
    modifiers: ModifiersState,
    resized: bool,
    last_measure_time: Instant,
    frame_number: usize,
}

impl GUI {
    pub fn new(graphics: &GraphicsWindow, window: &Window) -> Self {
        let viewport = Viewport::with_physical_size(
            Size::new(graphics.size.width, graphics.size.height),
            1.0,
        );
        let mut renderer = Renderer::new(Backend::new(
            &graphics.device,
            Settings::default(),
            graphics.config.format,
        ));
        let mut debug = Default::default();
        let controls = ControlState::default();
        let state = program::State::new(
            controls,
            viewport.logical_size(),
            &mut renderer,
            &mut debug,
        );
        let staging_belt = wgpu::util::StagingBelt::new(5 * 1024);
        let cursor_position = PhysicalPosition::new(-1.0, -1.0);
        let clipboard = Clipboard::connect(window);
        let modifiers = ModifiersState::default();
        Self {
            renderer,
            debug,
            viewport,
            staging_belt,
            state,
            cursor_position,
            clipboard,
            modifiers,
            resized: false,
            last_measure_time: Instant::now(),
            frame_number: 0,
        }
    }

    pub fn controls(&self) -> &ControlState {
        self.state.program()
    }

    pub fn render(&mut self, device: &Device, encoder: &mut CommandEncoder, frame: &TextureView) {
        self.renderer.with_primitives(|backend, primitives| {
            backend.present(
                device,
                &mut self.staging_belt,
                encoder,
                frame,
                primitives,
                &self.viewport,
                &self.debug.overlay(),
            );
        });
        self.staging_belt.finish();
    }

    pub fn post_render(&mut self) {
        self.staging_belt.recall();
    }

    pub fn change_state(&mut self, message: Message) {
        self.state.queue_message(message);
    }

    pub fn window_event(&mut self, window: &Window, event: &WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = *position;
            }
            WindowEvent::ModifiersChanged(new_modifiers) => {
                self.modifiers = *new_modifiers;
            }
            WindowEvent::Resized(_) => {
                self.resized = true;
            }
            _ => {}
        }
        if let Some(event) = conversion::window_event(
            event,
            window.scale_factor(),
            self.modifiers,
        ) {
            self.state.queue_event(event);
        }
    }

    pub fn update(&mut self) {
        self.update_frame_rate();

        if self.state.is_queue_empty() {
            return;
        }
        self.state.update(
            self.viewport.logical_size(),
            conversion::cursor_position(
                self.cursor_position,
                self.viewport.scale_factor(),
            ),
            &mut self.renderer,
            &iced_wgpu::Theme::Dark,
            &renderer::Style { text_color: Color::WHITE },
            &mut self.clipboard,
            &mut self.debug,
        );
    }

    fn update_frame_rate(&mut self) {
        self.frame_number += 1;
        let now = Instant::now();
        let time_elapsed = now - self.last_measure_time;
        if time_elapsed.as_secs_f64() < FRAME_RATE_MEASURE_INTERVAL_SECS {
            return;
        }
        self.last_measure_time = now;
        let average_time_per_frame = time_elapsed.as_secs_f64() / (self.frame_number as f64);
        self.frame_number = 0;
        let frame_rate = 1.0 / average_time_per_frame;
        self.state.queue_message(Message::FrameRateUpdated(frame_rate))
    }

    pub fn update_viewport(&mut self, window: &Window) {
        if !self.resized {
            return;
        }
        let size = window.inner_size();
        self.viewport = Viewport::with_physical_size(
            Size::new(size.width, size.height),
            window.scale_factor(),
        );
    }

    pub fn capturing_mouse(&self) -> bool {
        !matches!(self.state.mouse_interaction(), mouse::Interaction::Idle)
    }

    pub fn cursor_icon(&self) -> CursorIcon {
        conversion::mouse_interaction(
            self.state.mouse_interaction(),
        )
    }
}

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
    GravityChanged(f32),
    AddPulls { strain_nuance: f32 },
}

#[derive(Clone, Debug)]
pub struct ControlState {
    debug_mode: bool,
    visible_controls: VisibleControl,
    fabric_choice_control: FabricChoiceState,
    strain_threshold_control: StrainThresholdState,
    gravity_control: GravityState,
    frame_rate: f64,
    action_queue: RefCell<Vec<Action>>,
}

impl Default for ControlState {
    fn default() -> Self {
        let bootstrap = bootstrap_fabric_plans();
        Self {
            debug_mode: false,
            visible_controls: VisibleControl::FabricChoice,
            fabric_choice_control: FabricChoiceState {
                current: None,
                choices: bootstrap,
            },
            strain_threshold_control: StrainThresholdState {
                nuance: 0.0,
                strain_threshold: 0.0,
            },
            gravity_control: GravityState {
                nuance: 0.0,
                min_gravity: 1e-8,
                max_gravity: 5e-7,
            },
            frame_rate: 0.0,
            action_queue: RefCell::new(Vec::new()),
        }
    }
}

impl ControlState {
    pub fn take_actions(&self) -> Vec<Action> {
        self.action_queue.borrow_mut().split_off(0)
    }

    pub fn get_strain_threshold(&self, maximum_strain: f32) -> f32 {
        maximum_strain * self.strain_threshold_control.nuance
    }

    pub fn strain_threshold_changed(&self, strain_threshold: f32) -> Message {
        StrainThresholdChanged(strain_threshold).into()
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    ToggleDebugMode,
    Reset,
    ShowControl(VisibleControl),
    FabricChoice(FabricChoiceMessage),
    StrainThreshold(StrainThresholdMessage),
    Gravity(GravityMessage),
    FrameRateUpdated(f64),
}

impl Program for ControlState {
    type Renderer = Renderer;
    type Message = Message;

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        let queue_action = |action: Option<Action>|{
            if let Some(action) = action {
                self.action_queue.borrow_mut().push(action);
            }
        };
        match message {
            Message::ToggleDebugMode => {
                self.debug_mode = !self.debug_mode;
            }
            Message::Reset => {
                self.visible_controls = VisibleControl::ControlChoice;
                self.gravity_control.update(GravityMessage::Reset);
            }
            Message::ShowControl(visible_control) => {
                self.visible_controls = visible_control;
            }
            Message::FabricChoice(message) => {
                queue_action(self.fabric_choice_control.update(message))
            }
            Message::StrainThreshold(message) => {
                queue_action(self.strain_threshold_control.update(message))
            }
            Message::Gravity(message) => {
                queue_action(self.gravity_control.update(message))
            }
            Message::FrameRateUpdated(frame_rate) => {
                self.frame_rate = frame_rate;
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Message, Renderer> {
        let Self { frame_rate, .. } = *self;
        let mut right_column = Column::new()
            .width(Length::Fill)
            .align_items(Alignment::End);
        #[cfg(not(target_arch = "wasm32"))]
        {
            right_column = right_column
                .push(
                    Text::new(format!("{frame_rate:.01} FPS"))
                        .style(Color::WHITE)
                );
        }

        let element: Element<'_, Message, Renderer> =
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
                .push(Self::layout_row(
                    match self.visible_controls {
                        VisibleControl::ControlChoice => {
                            Row::new()
                                .push(Button::new(Text::new("Fabrics"))
                                    .on_press(Message::ShowControl(VisibleControl::FabricChoice)))
                                .push(Button::new(Text::new("Measure"))
                                    .on_press(Message::ShowControl(VisibleControl::StrainThreshold)))
                                .push(Button::new(Text::new("Gravity"))
                                    .on_press(Message::ShowControl(VisibleControl::Gravity)))

                        }
                        VisibleControl::FabricChoice => self.fabric_choice_control.row(),
                        VisibleControl::StrainThreshold => self.strain_threshold_control.row(),
                        VisibleControl::Gravity => self.gravity_control.row(),
                    }
                ))
                .into();
        if self.debug_mode {
            element.explain(Color::WHITE)
        } else {
            element
        }
    }
}

impl ControlState {
    fn layout_row(row: Row<'_, Message, Renderer>) -> Row<'_, Message, Renderer> {
        row
            .padding(5)
            .spacing(10)
            .width(Length::Fill)
            .align_items(Alignment::Center)
    }
}

