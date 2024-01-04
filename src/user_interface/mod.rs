use std::time::SystemTime;

use winit_input_helper::WinitInputHelper;

#[cfg(target_arch = "wasm32")]
use instant::Instant;

use crate::build::tenscript::{FabricPlan, FaceAlias};
use crate::build::tenscript::fabric_library::FabricLibrary;
use crate::build::tinkerer::{BrickOnFace, Frozen};
use crate::camera::Pick;
use crate::crucible::{CrucibleAction, CrucibleState};
use crate::fabric::face::FaceRotation;
use crate::scene::SceneAction;
use crate::user_interface::control_state::ControlState;
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

#[derive(Debug, Clone, Copy)]
pub enum MenuAction {
    StickAround,
    ReturnToRoot,
    TinkerMenu,
    UpOneLevel,
}

#[derive(Debug, Clone)]
pub struct MenuContext {
    pub selection_count: usize,
    pub crucible_state: CrucibleState,
    pub fabric_menu: Menu,
}

impl MenuContext {
    pub fn new(fabric_menu: Menu) -> Self {
        Self {
            selection_count: 0,
            crucible_state: Default::default(),
            fabric_menu,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ControlMessage {
    ToggleDebugMode,
    Reset,
    Keyboard(KeyboardMessage),
    StrainThreshold(StrainThresholdMessage),
    Gravity(GravityMessage),
    Muscle(MuscleMessage),
    Action(Action),
    FreshLibrary(FabricLibrary),
}

#[derive(Clone, Debug)]
pub enum Action {
    Crucible(CrucibleAction),
    UpdateMenu,
    Scene(SceneAction),
    Keyboard(MenuAction),
    CalibrateStrain,
    SelectFace(Option<Pick>),
    ToggleDebug,
    ProposeBrick { alias: FaceAlias, face_rotation: FaceRotation },
    RemoveProposedBrick,
    Connect,
    InitiateJoinFaces,
    Revert,
    RevertToFrozen { frozen: Frozen, brick_on_face: Option<BrickOnFace> },
    UpdatedLibrary(SystemTime),
}

pub struct UserInterface {
    state: ControlState,
}

impl UserInterface {
    pub fn new(fabrics: &[FabricPlan]) -> Self {
        let menu_context = MenuContext::new(Menu::fabric_menu(fabrics));
        let plan = fabrics
            .iter()
            .find(|fp|fp.name.last().unwrap().contains(&"Omni".to_string()))
            .unwrap();
        let state = ControlState::new(menu_context);
        state.queue_action(Action::Crucible(CrucibleAction::BuildFabric(plan.clone())));
        Self { state }
    }

    pub fn controls(&self) -> &ControlState {
        &self.state
    }

    pub fn message(&mut self, _control_message: ControlMessage) {
        // self.state.queue_message(control_message);
    }

    pub fn handle_input(&mut self, _input: &WinitInputHelper) {
        // self.message(ControlMessage::Keyboard(KeyboardMessage::KeyPressed(*keycode_pressed)));
    }

    pub fn set_menu_context(&mut self, menu_evironment: MenuContext) {
        self.message(ControlMessage::Keyboard(KeyboardMessage::SetEnvironment(menu_evironment)))
    }

    pub fn menu_choice(&mut self, menu_choice: MenuAction) {
        self.message(ControlMessage::Keyboard(KeyboardMessage::SelectMenu(menu_choice)))
    }

    pub fn set_strain_limits(&mut self, strain_limits: (f32, f32)) {
        self.message(ControlMessage::StrainThreshold(StrainThresholdMessage::SetStrainLimits(strain_limits)))
    }

    pub fn action(&mut self, _action: Action) {
        // self.state.queue_message(ControlMessage::Action(action))
    }

    pub fn create_fabric_menu(&self, fabrics: &[FabricPlan]) -> Menu {
        Menu::fabric_menu(fabrics)
    }
}
