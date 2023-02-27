use std::collections::HashSet;

use winit::event::VirtualKeyCode;
use winit::event::VirtualKeyCode::{*};

use crate::build::tenscript::{FabricPlan, FaceAlias, Library};
use crate::crucible::CrucibleAction;
use crate::fabric::face::FaceRotation;
use crate::fabric::physics::SurfaceCharacter;
use crate::scene::SceneAction;
use crate::user_interface::{Action, MenuChoice, MenuEnvironment};
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
    pub keycode: Option<VirtualKeyCode>,
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
                used.insert(menu.keycode.unwrap());
                (maybe.exists_in)(environment).then_some(menu)
            })
            .collect();
        sub
    }

    pub fn select(menu_choice: MenuChoice) -> Menu {
        match menu_choice {
            MenuChoice::Root => Menu::root_menu(),
            MenuChoice::Tinker => Menu::tinker_menu(),
        }
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

    fn root_menu() -> Menu {
        Menu::new("Tensegrity Lab")
            .submenu(ALWAYS, Menu::fabric_menu(&Library::standard().fabrics))
            .submenu(ALWAYS, Menu::speed_menu())
            .submenu(
                ALWAYS,
                Menu::new("Camera")
                    .action("Midpoint", true, ALWAYS, Action::Scene(SceneAction::WatchMidpoint))
                    .action("Origin", true, ALWAYS, Action::Scene(SceneAction::WatchOrigin)),
            )
            .submenu(
                ALWAYS,
                Menu::new("Widget")
                    .action("Gravity", true, |env| env.pretenst_complete, Action::ShowControl(VisibleControl::Gravity))
                    .action("Strain threshold", true, |env| env.pretenst_complete, Action::ShowControl(VisibleControl::StrainThreshold))
                    .action("Clear", true, ALWAYS, Action::ShowControl(VisibleControl::Nothing)),
            )
            .submenu(
                ALWAYS,
                Menu::new("Etc")
                    .action("Debug toggle", true, ALWAYS, Action::ToggleDebug),
            )
    }


    fn tinker_menu() -> Menu {
        Menu::new("Tinker")
            .action("Connect", false, |env| { env.brick_proposed },
                    Action::Connect)
            .action("Join", false, |env| env.selection_count == 2,
                    Action::InitiateJoinFaces)
            .action("Revert", false, |env| env.face_count > 0,
                    Action::Revert)
            .submenu(
                |env| env.selection_count == 1,
                Menu::new("Add")
                    .action("Single", false, ALWAYS,
                            Action::ProposeBrick { alias: FaceAlias::single("Single"), face_rotation: FaceRotation::Zero })
                    .action("Omni", false, ALWAYS,
                            Action::ProposeBrick { alias: FaceAlias::single("Omni"), face_rotation: FaceRotation::Zero })
                    .action("Torque", false, ALWAYS,
                            Action::ProposeBrick { alias: FaceAlias::single("Torque"), face_rotation: FaceRotation::Zero })
                    .action("Torque120", false, ALWAYS,
                            Action::ProposeBrick { alias: FaceAlias::single("Torque"), face_rotation: FaceRotation::OneThird })
                    .action("Torque240", false, ALWAYS,
                            Action::ProposeBrick { alias: FaceAlias::single("Torque"), face_rotation: FaceRotation::TwoThirds })
                    .action("Connect", true, ALWAYS,
                            Action::Connect)
                    .action("Revert", true, ALWAYS,
                            Action::Revert))
            .action("Frozen", false, |_| true,
                    Action::Crucible(CrucibleAction::StartPretensing(SurfaceCharacter::Frozen)))
            .action("Bouncy", false, |_| true,
                    Action::Crucible(CrucibleAction::StartPretensing(SurfaceCharacter::Bouncy)))
    }

    fn assign_key(self, used: &HashSet<VirtualKeyCode>) -> Menu {
        let label = self.label.clone();
        let (keycode, prefix) = self.label
            .chars()
            .find_map(|ch| {
                let key_code = to_key_code(ch)?;
                (!used.contains(&key_code))
                    .then_some((key_code, format!("{}: ", ch.to_ascii_uppercase())))
            })
            .unwrap();
        let mut new = self;
        new.keycode = Some(keycode);
        new.label = format!("{prefix}{label}");
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
