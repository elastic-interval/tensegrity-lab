#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use iced::mouse;
use iced_wgpu::{Backend, Renderer, Settings};
use iced_winit::{Clipboard, Color, conversion, Debug, program, renderer, Size, Viewport};
use wgpu::{CommandEncoder, Device, TextureView};
use winit::dpi::PhysicalPosition;
use winit::event::{ModifiersState, VirtualKeyCode, WindowEvent};
use VirtualKeyCode::{*};
use winit::window::{CursorIcon, Window};

#[cfg(target_arch = "wasm32")]
use instant::Instant;
use crate::build::tenscript::{FabricPlan, FaceAlias, Library};
use crate::fabric::UniqueId;

use crate::graphics::GraphicsWindow;
use crate::user_interface::control_state::{ControlMessage, ControlState};
use crate::user_interface::keyboard::{KeyboardMessage, Menu};
use crate::user_interface::strain_threshold::StrainThresholdMessage;

mod strain_threshold;
mod gravity;
mod control_state;
pub mod keyboard;

const FRAME_RATE_MEASURE_INTERVAL_SECS: f64 = 0.5;

#[derive(Clone, Debug)]
pub enum Action {
    BuildFabric(FabricPlan),
    SelectFace(UniqueId),
    AddBrick { face_alias: FaceAlias, face_id: UniqueId },
    GravityChanged(f32),
    ShowSurface,
    CalibrateStrain,
    ToggleDebug,
    SetSpeed(usize),
    CreateBrick,
    SelectNextFace,
    WatchMidpoint,
    WatchOrigin,
}

const NUMBER_KEYS: [VirtualKeyCode; 9] = [Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9];

fn fabric_menu(fabrics: &Vec<FabricPlan>, below: Vec<String>) -> Vec<Menu> {
    let sub_fabrics: Vec<_> = fabrics
        .iter()
        .filter(|fabric|{
            let mut compare = below.clone();
            compare.push(fabric.name.last().unwrap().clone());
            compare == fabric.name
        })
        .collect();
    if sub_fabrics.is_empty() {
        let mut unique: Vec<String> = Vec::new();
        for plan in fabrics {
            let next_name = plan.name.iter().nth(below.len()).unwrap();
            match unique.last() {
                None => unique.push(next_name.clone()),
                Some(last_next_name) if next_name != last_next_name => unique.push(next_name.clone()),
                _ => {}
            }
        }
        unique
            .iter()
            .zip(NUMBER_KEYS.into_iter().enumerate())
            .map(|(first, (index,key))| {
                let mut new_below = below.clone();
                new_below.push(first.clone());
                let label = format!("{}: {}", index + 1, first);
                Menu::new(label.as_str(), key, fabric_menu(fabrics, new_below))
            })
            .collect()
    } else {
        sub_fabrics
            .into_iter()
            .zip(NUMBER_KEYS.into_iter().enumerate())
            .map(|(fabric_plan, (index, key))|{
                let label = format!("{}: {}", index + 1, fabric_plan.name.last().unwrap());
                Menu::action(label.as_str(), key, Action::BuildFabric(fabric_plan.clone()))
            })
            .collect()
    }
    // fabrics
    //     .into_iter()
    //     .zip(NUMBER_KEYS.iter().enumerate())
    //     .map(|(plan, (index, key))|
    //         (format!("{}: {}", index + 1, plan.name.last().unwrap()), key, plan))
    //     .map(|(label, key, plan)|
    //         Menu::action(label.as_str(), *key, Action::BuildFabric(plan)))
    //     .collect()
}

fn speed_menu() -> Vec<Menu> {
    [(0usize, "Paused"), (5, "Glacial"), (25, "Slow"), (125, "Normal"), (625, "Fast")]
        .into_iter()
        .zip(NUMBER_KEYS.iter().enumerate())
        .map(|((speed, label), (index, key))|
            (format!("{}: {label} ({speed})", index + 1), key, speed))
        .map(|(label, key, speed)|
            Menu::action(label.as_str(), *key, Action::SetSpeed(speed)))
        .collect()
}

fn action_menu() -> Menu {
    Menu::new("Lab", Space, vec![
        Menu::new("Fabric", F, fabric_menu(&Library::standard().fabrics, Vec::new())),
        Menu::new("Speed", S, speed_menu()),
        Menu::new("Camera", C, vec![
            Menu::action("Midpoint", M, Action::WatchMidpoint),
            Menu::action("Origin", O, Action::WatchOrigin),
        ]),
        Menu::action("Debug toggle", D, Action::ToggleDebug),
        Menu::action("Brick create", B, Action::CreateBrick),
        Menu::action("Next face", N, Action::SelectNextFace),
    ])
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

    pub fn key_pressed(&mut self, keycode_pressed: &VirtualKeyCode) {
        self.state.queue_message(ControlMessage::Keyboard(KeyboardMessage::KeyPressed(*keycode_pressed)));
    }

    pub fn set_strain_limits(&mut self, strain_limits: (f32, f32)) {
        self.state.queue_message(ControlMessage::StrainThreshold(StrainThresholdMessage::SetStrainLimits(strain_limits)))
    }

    pub fn reset(&mut self) {
        self.state.queue_message(ControlMessage::Reset);
    }

    pub fn action(&mut self, action: Action) {
        self.state.queue_message(ControlMessage::Action(action))
    }

    pub fn toggle_debug_mode(&mut self) {
        self.state.queue_message(ControlMessage::ToggleDebugMode)
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
