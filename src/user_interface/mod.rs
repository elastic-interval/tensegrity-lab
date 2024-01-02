use std::time::SystemTime;

use winit::event::VirtualKeyCode;

#[cfg(target_arch = "wasm32")]
use instant::Instant;

use crate::build::tenscript::{FabricPlan, FaceAlias};
use crate::build::tenscript::fabric_library::FabricLibrary;
use crate::build::tinkerer::{BrickOnFace, Frozen};
use crate::camera::Pick;
use crate::crucible::CrucibleAction;
use crate::fabric::face::FaceRotation;
use crate::scene::SceneAction;
use crate::user_interface::control_state::{ControlState, VisibleControl};
use crate::user_interface::gravity::GravityMessage;
use crate::user_interface::keyboard::KeyboardMessage;
use crate::user_interface::menu::Menu;
use crate::user_interface::muscle::MuscleMessage;
use crate::user_interface::strain_threshold::StrainThresholdMessage;

mod strain_threshold;
mod gravity;
mod keyboard;
mod control_state;
mod menu;
mod muscle;

const FRAME_RATE_MEASURE_INTERVAL_SECS: f64 = 0.5;

#[derive(Debug, Clone, Copy)]
pub enum MenuAction {
    StickAround,
    ReturnToRoot,
    TinkerMenu,
    UpOneLevel,
}

#[derive(Debug, Clone)]
pub struct MenuEnvironment {
    pub face_count: usize,
    pub selection_count: usize,
    pub tinkering: bool,
    pub brick_proposed: bool,
    pub experimenting: bool,
    pub history_available: bool,
    pub visible_control: VisibleControl,
    pub fabric_menu: Menu,
}

impl MenuEnvironment {
    pub fn new(fabric_menu: Menu) -> Self {
        Self {
            face_count: 0,
            selection_count: 0,
            tinkering: false,
            brick_proposed: false,
            experimenting: false,
            history_available: false,
            visible_control: Default::default(),
            fabric_menu,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ControlMessage {
    ToggleDebugMode,
    Reset,
    ShowControl(VisibleControl),
    Keyboard(KeyboardMessage),
    StrainThreshold(StrainThresholdMessage),
    Gravity(GravityMessage),
    Muscle(MuscleMessage),
    Action(Action),
    FrameRateUpdated(f64),
    FreshLibrary(FabricLibrary),
}

#[derive(Clone, Debug)]
pub enum FaceChoice {
    Left,
    Right,
}

#[derive(Clone, Debug)]
pub enum Action {
    Crucible(CrucibleAction),
    UpdateMenu,
    Scene(SceneAction),
    Keyboard(MenuAction),
    CalibrateStrain,
    SelectFace(Option<Pick>),
    ShowControl(VisibleControl),
    SelectAFace,
    ToggleDebug,
    ProposeBrick { alias: FaceAlias, face_rotation: FaceRotation },
    RemoveProposedBrick,
    Connect,
    InitiateJoinFaces,
    Revert,
    RevertToFrozen { frozen: Frozen, brick_on_face: Option<BrickOnFace> },
    UpdatedLibrary(SystemTime),
}

/// Largely adapted from https://github.com/iced-rs/iced/blob/master/examples/integration_wgpu/src/main.rs
pub struct UserInterface {
    state: ControlState,
}

impl UserInterface {
    pub fn new(fabrics: &[FabricPlan]) -> Self {
        let menu_environment = MenuEnvironment::new(Menu::fabric_menu(fabrics));
        let plan = fabrics
            .iter()
            .find(|fp|fp.name.last().unwrap().contains(&"Tommy".to_string()))
            .unwrap();
        let state = ControlState::new(menu_environment, plan.clone());
        Self { state }
    }

    pub fn controls(&self) -> &ControlState {
        &self.state
    }

    pub fn message(&mut self, _control_message: ControlMessage) {}

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

    pub fn action(&mut self, _action: Action) {}

    pub fn create_fabric_menu(&self, fabrics: &[FabricPlan]) -> Menu {
        Menu::fabric_menu(fabrics)
    }
}
