#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use iced::mouse;
use iced_wgpu::{Backend, Renderer, Settings};
use iced_winit::{Clipboard, Color, conversion, Debug, program, renderer, Size, Viewport};
use wgpu::{CommandEncoder, Device, TextureView};
use winit::dpi::PhysicalPosition;
use winit::event::{ModifiersState, WindowEvent};
use winit::window::{CursorIcon, Window};

#[cfg(target_arch = "wasm32")]
use instant::Instant;

use crate::graphics::GraphicsWindow;
use crate::gui::control_state::{ControlMessage, ControlState};

pub mod fabric_choice;
pub mod strain_threshold;
pub mod gravity;
pub mod control_state;

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

    pub fn queue_message(&mut self, message: ControlMessage) {
        self.state.queue_message(message);
    }

    pub fn window_event(&mut self, event: &WindowEvent, window: &Window) {
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
        self.state.queue_message(ControlMessage::FrameRateUpdated(frame_rate))
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

    pub fn cursor_icon(&self) -> CursorIcon {
        conversion::mouse_interaction(
            self.state.mouse_interaction(),
        )
    }

    pub fn capturing_mouse(&self) -> bool {
        !matches!(self.state.mouse_interaction(), mouse::Interaction::Idle)
    }
}
