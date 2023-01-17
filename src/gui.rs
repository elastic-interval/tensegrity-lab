use std::cell::RefCell;
use iced_wgpu::{Backend, Renderer, Settings};
use iced_winit::{Alignment, Clipboard, Color, Command, conversion, Debug, Element, Length, mouse, Program, program, renderer, Size, Viewport};
use iced_winit::widget::{Button, Column, Row, Slider, Text};
use wgpu::{CommandEncoder, Device, TextureView};
use winit::dpi::PhysicalPosition;
use winit::event::{ModifiersState, WindowEvent};
use winit::window::{CursorIcon, Window};

#[cfg(target_arch = "wasm32")]
use instant::Instant;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
use crate::fabric::annealing::MeasureLimits;

use crate::graphics::GraphicsWindow;

const FRAME_RATE_MEASURE_INTERVAL_SECS: f64 = 0.5;

/// Largely adapted from https://github.com/iced-rs/iced/blob/master/examples/integration_wgpu/src/main.rs
pub struct GUI {
    renderer: Renderer,
    debug: Debug,
    viewport: Viewport,
    staging_belt: wgpu::util::StagingBelt,
    state: program::State<Controls>,
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
        let controls = Controls::default();
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

    pub fn controls(&self) -> &Controls {
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
pub enum Showing {
    Nothing,
    StrainThreshold,
}

#[derive(Clone, Copy, Debug)]
pub enum Action {
    AddPulls { measure_nuance: f32 },
}

pub struct Controls {
    showing: Showing,
    measure_nuance: f32,
    lower_limit: f32,
    frame_rate: f64,
    action_queue: RefCell<Vec<Action>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    ShowControls,
    StrainLowerLimit(f32),
    MeasureNuanceChanged(f32),
    AddPulls,
    FrameRateUpdated(f64),
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            showing: Showing::Nothing,
            measure_nuance: 0.0,
            lower_limit: f32::MIN,
            frame_rate: 0.0,
            action_queue: RefCell::new(Vec::new()),
        }
    }
}

impl Controls {
    pub fn take_actions(&self) -> Vec<Action> {
        self.action_queue.borrow_mut().split_off(0)
    }

    pub fn strain_lower_limit(&self, limits: MeasureLimits) -> f32 {
        limits.interpolate(self.measure_nuance)
    }
}

impl Program for Controls {
    type Renderer = Renderer;
    type Message = Message;

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::ShowControls => {
                self.showing = Showing::StrainThreshold;
            }
            Message::MeasureNuanceChanged(nuance) => {
                self.measure_nuance = nuance;
            }
            Message::StrainLowerLimit(limit) => {
                self.lower_limit = limit;
            }
            Message::AddPulls => {
                self.action_queue.borrow_mut().push(Action::AddPulls { measure_nuance: self.measure_nuance });
            }
            Message::FrameRateUpdated(frame_rate) => {
                self.frame_rate = frame_rate;
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Renderer> {
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
        let strain_limit = self.lower_limit;
        let element: Element<'_, Self::Message, Self::Renderer> =
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
                    match self.showing {
                        Showing::Nothing => Row::new(),
                        Showing::StrainThreshold => {
                            Row::new()
                                .padding(20)
                                .spacing(20)
                                .push(
                                    Text::new("Strain threshold")
                                        .style(Color::WHITE)
                                )
                                .push(
                                    Slider::new(0.0..=1.0, self.measure_nuance, Message::MeasureNuanceChanged)
                                        .step(0.01)
                                )
                                .push(
                                    Text::new(format!("{strain_limit:.05}"))
                                        .style(Color::WHITE)
                                )
                                .push(
                                    Button::new(Text::new("Add Pulls"))
                                        .on_press(Message::AddPulls)
                                )
                        }
                    }
                )
                .into();
        // element.explain(Color::WHITE)
        element
    }
}
