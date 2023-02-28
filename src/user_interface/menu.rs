use std::collections::HashSet;

use winit::event::VirtualKeyCode;
use winit::event::VirtualKeyCode::{*};

use crate::build::tenscript::{FabricPlan, FaceAlias, Library};
use crate::crucible::{CrucibleAction, TinkererAction};
use crate::fabric::face::FaceRotation;
use crate::fabric::physics::SurfaceCharacter;
use crate::scene::SceneAction;
use crate::user_interface::{Action, MenuAction, MenuEnvironment};
use crate::user_interface::control_state::VisibleControl;

#[derive(Debug, Clone)]
pub struct MaybeMenu {
    exists_in: fn(MenuEnvironment) -> bool,
    menu: Menu,
}

impl MaybeMenu {
    pub fn menu_in(&self, environment: MenuEnvironment) -> Option<Menu> {
        (self.exists_in)(environment).then_some(self.menu.clone())
    }
}

#[derive(Debug, Clone)]
pub struct Menu {
    pub label: String,
    pub keycode: Option<(VirtualKeyCode, String)>,
    pub submenu: Vec<MaybeMenu>,
    pub action: Option<Action>,
    pub exit_action: bool,
}

impl Menu {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            keycode: None,
            submenu: Vec::new(),
            action: None,
            exit_action: false,
        }
    }

    pub fn label(&self) -> String {
        match &self.keycode {
            None => self.label.clone(),
            Some((_, prefix)) => format!("{}{}", prefix, self.label)
        }
    }

    pub fn submenu(self, exists_in: fn(MenuEnvironment) -> bool, menu: Menu) -> Self {
        let mut new = self;
        new.submenu.push(
            MaybeMenu {
                exists_in,
                menu: Self {
                    label: menu.label,
                    keycode: None,
                    submenu: menu.submenu,
                    action: None,
                    exit_action: false,
                },
            }
        );
        new
    }

    pub fn action(self, label: &str, exit_action: bool, exists_in: fn(MenuEnvironment) -> bool, action: Action) -> Self {
        let maybe = MaybeMenu {
            exists_in,
            menu: Menu {
                label: label.to_string(),
                keycode: None,
                action: Some(action),
                submenu: Vec::new(),
                exit_action,
            },
        };
        let mut new = self;
        new.submenu.push(maybe);
        new
    }

    pub fn submenu_in(&self, environment: MenuEnvironment) -> Vec<Menu> {
        let mut used = HashSet::new();
        let sub: Vec<_> = self.submenu
            .clone()
            .into_iter()
            .flat_map(|maybe| {
                let menu = maybe.menu.assign_key(&used);
                let (code, _) = menu.keycode.clone().unwrap();
                used.insert(code);
                (maybe.exists_in)(environment).then_some(menu)
            })
            .collect();
        sub
    }

    fn fabric_menu_recurse(menu: Menu, fabrics: &[FabricPlan], below: Vec<String>) -> Menu {
        let sub_fabrics: Vec<_> = fabrics
            .iter()
            .filter(|&fabric| {
                let mut compare = below.clone();
                compare.push(fabric.name.last().unwrap().clone());
                compare == fabric.name
            })
            .collect();
        if sub_fabrics.is_empty() {
            let mut unique: Vec<String> = Vec::new();
            for plan in fabrics {
                let next_name = plan.name.get(below.len()).unwrap();
                match unique.last() {
                    None => unique.push(next_name.clone()),
                    Some(last_next_name) if next_name != last_next_name => unique.push(next_name.clone()),
                    _ => {}
                }
            }
            let mut menu = menu;
            for first in unique {
                let mut new_below = below.clone();
                new_below.push(first.clone());
                menu = menu.submenu(ALWAYS, Menu::fabric_menu_recurse(Menu::new(first.as_str()), fabrics, new_below));
            }
            menu
        } else {
            let mut menu = Menu::new(below.last().unwrap());
            for fabric_plan in sub_fabrics {
                let label = fabric_plan.name.last().unwrap();
                menu = menu.action(
                    label.as_str(), false, ALWAYS,
                    Action::Crucible(CrucibleAction::BuildFabric(fabric_plan.clone())),
                );
            }
            menu
        }
    }

    fn fabric_menu(fabrics: &[FabricPlan]) -> Menu {
        Self::fabric_menu_recurse(Menu::new("Fabrics"), fabrics, Vec::new())
    }

    fn speed_menu() -> Menu {
        let mut menu = Menu::new("Speed");
        for (speed, label) in [(0usize, "Paused"), (5, "Glacial"), (25, "Slow"), (125, "Normal"), (625, "Fast")] {
            menu = menu.action(label, true, ALWAYS, Action::Crucible(CrucibleAction::SetSpeed(speed)));
        }
        menu
    }

    pub fn root_menu() -> Menu {
        Menu::new("Tensegrity Lab")
            .action("Tinker", false, |env| env.face_count > 0,
                    Action::SelectAFace)
            .submenu(ALWAYS, Menu::fabric_menu(&Library::standard().fabrics))
            .submenu(ALWAYS, Menu::speed_menu())
            .submenu(
                ALWAYS,
                Menu::new("Camera")
                    .action("Midpoint", true, ALWAYS, Action::Scene(SceneAction::WatchMidpoint))
                    .action("Origin", true, ALWAYS, Action::Scene(SceneAction::WatchOrigin)),
            )
            .action("Gravity control", true,
                    |env| env.crucible_finished && env.visible_control != VisibleControl::Gravity,
                    Action::ShowControl(VisibleControl::Gravity))
            .action("Strain control", true,
                    |env| env.crucible_finished && env.visible_control != VisibleControl::StrainThreshold,
                    Action::ShowControl(VisibleControl::StrainThreshold))
            .action("Hide controls", true,
                    |env| env.crucible_finished && env.visible_control != VisibleControl::Nothing,
                    Action::ShowControl(VisibleControl::Nothing))
            .submenu(
                ALWAYS,
                Menu::new("Etc")
                    .action("Debug toggle", true, ALWAYS, Action::ToggleDebug),
            )
    }

    pub fn tinker_menu() -> Menu {
        Menu::new("Tinker")
            .action("Pick a face with <Shift-click>", false, |env| env.selection_count == 0,
                    Action::SelectAFace)
            .action("Connect the new brick", false, |env| env.brick_proposed,
                    Action::Connect)
            .action("Join the selected faces", false, |env| env.selection_count == 2,
                    Action::InitiateJoinFaces)
            .action("Revert to previous", false, |env| env.history_available,
                    Action::Revert)
            .submenu(
                |env| env.selection_count == 1,
                Menu::new("Add a brick at the green face")
                    .action("Single", false, ALWAYS,
                            Action::ProposeBrick { alias: FaceAlias::single("Single"), face_rotation: FaceRotation::Zero })
                    .action("Omni", false, ALWAYS,
                            Action::ProposeBrick { alias: FaceAlias::single("Omni"), face_rotation: FaceRotation::Zero })
                    .action("Torque-000", false, ALWAYS,
                            Action::ProposeBrick { alias: FaceAlias::single("Torque"), face_rotation: FaceRotation::Zero })
                    .action("Torque-120", false, ALWAYS,
                            Action::ProposeBrick { alias: FaceAlias::single("Torque"), face_rotation: FaceRotation::OneThird })
                    .action("Torque-240", false, ALWAYS,
                            Action::ProposeBrick { alias: FaceAlias::single("Torque"), face_rotation: FaceRotation::TwoThirds })
                    .action("Skip it", true, |env| env.brick_proposed,
                            Action::Crucible(CrucibleAction::Tinkerer(TinkererAction::Clear)))
                    .action("Connect", true, |env| env.brick_proposed,
                            Action::Connect))
            .submenu(
                ALWAYS, Menu::new("Finish")
                    .action("Sticky surface", true, |_| true,
                            Action::Crucible(CrucibleAction::StartPretensing(SurfaceCharacter::Frozen)))
                    .action("Bouncy surface", true, |_| true,
                            Action::Crucible(CrucibleAction::StartPretensing(SurfaceCharacter::Bouncy)))
                    .action("Not yet", true, |_| true,
                            Action::Keyboard(MenuAction::UpOneLevel)),
            )
    }

    fn assign_key(self, used: &HashSet<VirtualKeyCode>) -> Menu {
        let keycode = self.label
            .chars()
            .find_map(|ch| {
                let key_code = to_key_code(ch)?;
                (!used.contains(&key_code))
                    .then_some((key_code, format!("{}: ", ch.to_ascii_uppercase())))
            })
            .unwrap();
        let mut new = self;
        new.keycode = Some(keycode);
        new
    }
}

fn to_key_code(ch: char) -> Option<VirtualKeyCode> {
    Some(match ch.to_ascii_uppercase() {
        'A' => A,
        'B' => B,
        'C' => C,
        'D' => D,
        'E' => E,
        'F' => F,
        'G' => G,
        'H' => H,
        'I' => I,
        'J' => J,
        'K' => K,
        'L' => L,
        'M' => M,
        'N' => N,
        'O' => O,
        'P' => P,
        'Q' => Q,
        'R' => R,
        'S' => S,
        'T' => T,
        'U' => U,
        'V' => V,
        'W' => W,
        'X' => X,
        'Y' => Y,
        'Z' => Z,
        _ => return None
    })
}

const ALWAYS: fn(MenuEnvironment) -> bool = |_| true;
