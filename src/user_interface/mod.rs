#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use iced::mouse;
use iced_wgpu::{Backend, Renderer, Settings};
use iced_winit::{Clipboard, Color, conversion, Debug, program, renderer, Size, Viewport};
use wgpu::{CommandEncoder, Device, TextureView};
use winit::dpi::PhysicalPosition;
use winit::event::{ModifiersState, VirtualKeyCode, WindowEvent};
use winit::window::{CursorIcon, Window};

#[cfg(target_arch = "wasm32")]
use instant::Instant;

use crate::build::tenscript::FaceAlias;
use crate::build::tinkerer::{BrickOnFace, Frozen};
use crate::crucible::CrucibleAction;
use crate::fabric::UniqueId;
use crate::fabric::face::FaceRotation;
use crate::graphics::GraphicsWindow;
use crate::scene::SceneAction;
use crate::user_interface::control_state::{ControlState, VisibleControl};
use crate::user_interface::gravity::GravityMessage;
use crate::user_interface::keyboard::KeyboardMessage;
use crate::user_interface::strain_threshold::StrainThresholdMessage;

mod strain_threshold;
mod gravity;
mod keyboard;
mod control_state;
mod menu;

const FRAME_RATE_MEASURE_INTERVAL_SECS: f64 = 0.5;

#[derive(Debug, Clone, Copy)]
pub enum MenuAction {
    StickAround,
    ReturnToRoot,
    TinkerMenu,
    UpOneLevel,
}

#[derive(Debug, Clone, Default, Copy)]
pub struct MenuEnvironment {
    pub face_count: usize,
    pub selection_count: usize,
    pub tinkering: bool,
    pub brick_proposed: bool,
    pub experimenting: bool,
    pub history_available: bool,
    pub visible_control: VisibleControl,
}

#[derive(Debug, Clone)]
pub enum ControlMessage {
    ToggleDebugMode,
    Reset,
    ShowControl(VisibleControl),
    Keyboard(KeyboardMessage),
    StrainThreshold(StrainThresholdMessage),
    Gravity(GravityMessage),
    Action(Action),
    FrameRateUpdated(f64),
}

#[derive(Clone, Debug)]
pub enum FaceChoice {
    Left, Right
}

#[derive(Clone, Debug)]
pub enum Action {
    Crucible(CrucibleAction),
    CrucibleFinished,
    Scene(SceneAction),
    Keyboard(MenuAction),
    CalibrateStrain,
    SelectFace(Option<UniqueId>),
    ShowControl(VisibleControl),
    ControlChange,
    SelectAFace,
    ToggleDebug,
    ProposeBrick { alias: FaceAlias, face_rotation: FaceRotation },
    RemoveProposedBrick,
    Connect,
    InitiateJoinFaces,
    Revert,
    RevertToFrozen { frozen: Frozen, brick_on_face: Option<BrickOnFace> },
}

/// Largely adapted from https://github.com/iced-rs/iced/blob/master/examples/integration_wgpu/src/main.rs
pub struct UserInterface {
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

impl UserInterface {
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

    pub fn message(&mut self, control_message: ControlMessage) {
        self.state.queue_message(control_message);
    }

    pub fn key_pressed(&mut self, keycode_pressed: &VirtualKeyCode) {
        self.message(ControlMessage::Keyboard(KeyboardMessage::KeyPressed(*keycode_pressed)));
    }

    pub fn set_menu_environment(&mut self, menu_evironment: MenuEnvironment) {
        self.message(ControlMessage::Keyboard(KeyboardMessage::SetEnvironment(menu_evironment)))
    }

    pub fn menu_choice(&mut self, menu_choice: MenuAction) {
        self.message(ControlMessage::Keyboard(KeyboardMessage::SelectMenu(menu_choice)))
    }

    pub fn set_strain_limits(&mut self, strain_limits: (f32, f32)) {
        self.message(ControlMessage::StrainThreshold(StrainThresholdMessage::SetStrainLimits(strain_limits)))
    }

    pub fn action(&mut self, action: Action) {
        self.state.queue_message(ControlMessage::Action(action))
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
