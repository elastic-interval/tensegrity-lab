use std::cell::RefCell;

#[cfg(target_arch = "wasm32")]
use instant::Instant;
use crate::build::tenscript::FabricPlan;
use crate::crucible::CrucibleAction;

use crate::fabric::physics::presets::AIR_GRAVITY;
use crate::user_interface::{Action, ControlMessage, MenuEnvironment};
use crate::user_interface::gravity::Gravity;
use crate::user_interface::keyboard::Keyboard;
use crate::user_interface::muscle::Muscle;
use crate::user_interface::strain_threshold::StrainThreshold;
use crate::user_interface::strain_threshold::StrainThresholdMessage::SetStrainLimits;

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub enum VisibleControl {
    #[default]
    Nothing,
    Gravity,
    Muscle,
    StrainThreshold,
}

#[derive(Clone)]
pub struct ControlState {
    debug_mode: bool,
    keyboard: Keyboard,
    visible_control: VisibleControl,
    strain_threshold: StrainThreshold,
    gravity: Gravity,
    muscle: Muscle,
    show_strain: bool,
    frame_rate: f64,
    action_queue: RefCell<Vec<Action>>,
}

impl ControlState {
    pub fn new(environment: MenuEnvironment, bootstrap_fabric: FabricPlan) -> Self {
        let action_queue = RefCell::new(Vec::<Action>::new());
        action_queue.borrow_mut().push(Action::Crucible(CrucibleAction::BuildFabric(bootstrap_fabric)));
        Self {
            keyboard: Keyboard::new(environment),
            debug_mode: false,
            visible_control: VisibleControl::Nothing,
            strain_threshold: StrainThreshold {
                nuance: 0.0,
                strain_limits: (0.0, 1.0),
            },
            gravity: Gravity::new(AIR_GRAVITY.gravity),
            muscle: Muscle::new(),
            show_strain: false,
            frame_rate: 0.0,
            action_queue,
        }
    }

    pub fn take_actions(&self) -> Vec<Action> {
        self.action_queue.borrow_mut().split_off(0)
    }

    pub fn queue_action(&self, action: Action) {
        self.action_queue.borrow_mut().push(action);
    }

    pub fn show_strain(&self) -> bool {
        self.show_strain
    }

    pub fn show_controls(&self) -> VisibleControl {
        self.visible_control
    }

    pub fn strain_limits_changed(&self, limits: (f32, f32)) -> ControlMessage {
        SetStrainLimits(limits).into()
    }
}

impl ControlState {
    fn update(&mut self, message: ControlMessage) {
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
                self.visible_control = VisibleControl::Nothing;
                // self.gravity.update(GravityMessage::Reset);
                queue_action(Some(Action::UpdateMenu))
            }
            ControlMessage::ShowControl(visible_control) => {
                self.visible_control = visible_control;
                match visible_control {
                    VisibleControl::StrainThreshold => {
                        queue_action(Some(Action::CalibrateStrain));
                        self.show_strain = true;
                    }
                    _ => {
                        self.show_strain = false;
                    }
                }
                queue_action(Some(Action::UpdateMenu));
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
            ControlMessage::FrameRateUpdated(frame_rate) => {
                self.frame_rate = frame_rate;
            }
            ControlMessage::FreshLibrary(_library) => {
                // self.keyboard.update(KeyboardMessage::FreshLibrary(library));
            }
        }
    }
}
