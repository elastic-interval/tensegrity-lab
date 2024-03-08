use std::cell::RefCell;
use std::time::SystemTime;

use winit::keyboard::Key;
use winit_input_helper::WinitInputHelper;

use crate::build::tenscript::fabric_library::FabricLibrary;
use crate::build::tenscript::FabricPlan;
use crate::crucible::CrucibleAction;
use crate::scene::SceneAction;
use crate::user_interface::menu::Menu;

mod menu;

#[derive(Debug, Clone, Copy)]
pub enum MenuAction {
    StickAround,
    ReturnToRoot,
    UpOneLevel,
}

#[derive(Debug, Clone)]
pub struct MenuContext {
    pub fabric_menu: Menu,
}

impl MenuContext {
    pub fn new(fabric_menu: Menu) -> Self {
        Self {
            fabric_menu,
        }
    }
}

#[derive(Debug, Clone)]
pub enum GravityMessage {
    NuanceChanged(f32),
    Reset,
}

#[derive(Debug, Clone)]
pub enum MuscleMessage {
    NuanceChanged(f32),
    Reset,
}

#[derive(Debug, Clone)]
pub enum StrainThresholdMessage {
    SetStrainLimits((f32, f32)),
    NuanceChanged(f32),
    Calibrate,
}

#[derive(Debug, Clone)]
pub enum KeyboardMessage {
    KeyPressed(Key),
    SelectSubmenu(Menu),
    SelectMenu(MenuAction),
    SubmitAction { action: Action, menu_action: MenuAction },
    SetEnvironment(MenuContext),
    FreshLibrary(FabricLibrary),
}

#[derive(Debug, Clone)]
pub enum ControlMessage {
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
    UpdatedLibrary(SystemTime),
}

pub struct UserInterface {
    action_queue: RefCell<Vec<Action>>,
}

impl UserInterface {
    pub fn new() -> Self {
        Self {
            action_queue: RefCell::default(),
        }
    }

    pub fn take_actions(&self) -> Vec<Action> {
        self.action_queue.borrow_mut().split_off(0)
    }

    pub fn queue_action(&self, action: Action) {
        self.action_queue.borrow_mut().push(action);
    }

    pub fn message(&mut self, control_message: ControlMessage) {
        match control_message {
            ControlMessage::Action(action) => {
                self.queue_action(action);
            }
            ControlMessage::Reset => {
                // self.gravity.update(GravityMessage::Reset);
                self.queue_action(Action::UpdateMenu);
            }
            ControlMessage::Keyboard(_message) => {
                // queue_action(self.keyboard.update(message));
            }
            ControlMessage::StrainThreshold(_message) => {
                // queue_action(self.strain_threshold.update(message));
            }
            ControlMessage::Gravity(_message) => {
                // queue_action(self.gravity.update(message));
            }
            ControlMessage::Muscle(_message) => {
                // queue_action(self.muscle.update(message));
            }
            ControlMessage::FreshLibrary(_library) => {
                // self.keyboard.update(KeyboardMessage::FreshLibrary(library));
            }
        }
    }

    pub fn handle_input(&mut self, _input: &WinitInputHelper) {
        // self.message(ControlMessage::Keyboard(KeyboardMessage::KeyPressed(*keycode_pressed)));
    }

    pub fn set_menu_context(&mut self, menu_environment: MenuContext) {
        self.message(ControlMessage::Keyboard(KeyboardMessage::SetEnvironment(menu_environment)))
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
