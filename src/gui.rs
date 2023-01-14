
use iced_wgpu::{Backend, Renderer, Settings};
use iced_winit::{Alignment, Clipboard, Color, Command, conversion, Debug, Element, Length, mouse, Program, program, renderer, Size, Viewport};
use iced_winit::widget::{Column, Row, slider, Text};
use wgpu::{CommandEncoder, Device, TextureView};
use winit::dpi::PhysicalPosition;
use winit::event::{ModifiersState, WindowEvent};
use winit::window::Window;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use instant::Instant;

use crate::graphics::GraphicsWindow;

///
/// Largely adapted from https://github.com/iced-rs/iced/blob/master/examples/integration_wgpu/src/main.rs
///
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

    start_time: Instant,
    frame_number: usize,
}


impl GUI {
    pub fn new(graphics: &GraphicsWindow, window: &Window) -> Self {
        let viewport = Viewport::with_physical_size(
            Size::new(1600, 1200),
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

            start_time: Instant::now(),
            frame_number: 0,
        }
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
        if self.frame_number % 100 != 0 {
            return;
        }
        let now = Instant::now();
        let time_elapsed = now - self.start_time;
        let average_time_per_frame = time_elapsed.as_secs_f64() / (self.frame_number as f64);
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
}

pub struct Controls {
    measure_threshold: f32,
    frame_rate: f64,
}

#[derive(Debug, Clone)]
pub enum Message {
    MeasureThresholdChanged(f32),
    FrameRateUpdated(f64),
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            measure_threshold: 0.0,
            frame_rate: 0.0,
        }
    }
}

impl Program for Controls {
    type Renderer = Renderer;
    type Message = Message;

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::MeasureThresholdChanged(new_threshold) => {
                self.measure_threshold = new_threshold;
            }

            Message::FrameRateUpdated(frame_rate) => {
                self.frame_rate = frame_rate;
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Renderer> {
        let Self { frame_rate, .. } = self;
        Row::new()
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(10)
            .align_items(Alignment::Start)
            .push(
                Column::new()
                    .width(Length::Fill)
                    .align_items(Alignment::Start)
                    .spacing(10)
                    .push(
                        Text::new("Measure threshold")
                            .style(Color::WHITE)
                            .size(14),
                    )
                    .push(
                        slider(0.0f32..=1.0, self.measure_threshold, |new_threshold| {
                            Message::MeasureThresholdChanged(new_threshold)
                        })
                            .step(0.01)
                    )
            )
            .push(
                Column::new()
                    .width(Length::Fill)
                    .align_items(Alignment::End)
                    .push(
                        Text::new(format!("{frame_rate:.01} FPS"))
                            .style(Color::WHITE)
                            .size(12),
                    )
            )
            .into()
    }
}
