use std::fmt::{Debug, Formatter};

use winit::keyboard::Key;

use crate::build::tenscript::FabricPlan;
use crate::crucible::CrucibleAction;
use crate::scene::SceneAction;
use crate::user_interface::{Action, MenuAction, MenuContext};
use crate::user_interface::MenuAction::*;

#[derive(Clone)]
pub struct MaybeMenu {
    exists_in: fn(&MenuContext) -> bool,
    menu: Menu,
}

impl Debug for MaybeMenu {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Maybe{:?}", self.menu)
    }
}

impl MaybeMenu {
    pub fn menu_in(&self, environment: &MenuContext) -> Option<Menu> {
        (self.exists_in)(environment).then_some(self.menu.clone())
    }
}

#[derive(Debug, Clone)]
pub struct Menu {
    pub label: String,
    pub keycode: Option<(Key, String)>,
    pub submenu: Vec<MaybeMenu>,
    pub action: Option<Action>,
    pub menu_action: MenuAction,
}

impl Menu {
    pub fn new(label: &str, menu_action: MenuAction) -> Self {
        Self {
            label: label.to_string(),
            keycode: None,
            submenu: Vec::new(),
            action: None,
            menu_action,
        }
    }

    pub fn label(&self) -> String {
        match &self.keycode {
            None => self.label.clone(),
            Some((_, prefix)) => format!("{}{}", prefix, self.label)
        }
    }

    pub fn submenu(self, exists_in: fn(&MenuContext) -> bool, menu: Menu) -> Self {
        let mut new = self;
        new.submenu.push(
            MaybeMenu {
                exists_in,
                menu: Self {
                    label: menu.label,
                    keycode: None,
                    submenu: menu.submenu,
                    action: None,
                    menu_action: menu.menu_action,
                },
            }
        );
        new
    }

    pub fn action(self, label: &str, menu_action: MenuAction, exists_in: fn(&MenuContext) -> bool, action: Action) -> Self {
        let maybe = MaybeMenu {
            exists_in,
            menu: Menu {
                label: label.to_string(),
                keycode: None,
                action: Some(action),
                submenu: Vec::new(),
                menu_action,
            },
        };
        let mut new = self;
        new.submenu.push(maybe);
        new
    }

    pub fn submenu_in(&self, _context: &MenuContext) -> Vec<Menu> {
        vec![]
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
                menu = menu.submenu(ALWAYS, Menu::fabric_menu_recurse(Menu::new(first.as_str(), UpOneLevel), fabrics, new_below));
            }
            menu
        } else {
            let mut menu = Menu::new(below.last().unwrap(), UpOneLevel);
            for fabric_plan in sub_fabrics {
                let label = fabric_plan.name.last().unwrap();
                menu = menu.action(
                    label.as_str(), ReturnToRoot, ALWAYS,
                    Action::Crucible(CrucibleAction::BuildFabric(fabric_plan.clone())),
                );
            }
            menu
        }
    }

    pub fn fabric_menu(fabrics: &[FabricPlan]) -> Menu {
        Self::fabric_menu_recurse(Menu::new("Tensegrity menu", UpOneLevel), fabrics, Vec::new())
    }

    fn speed_menu() -> Menu {
        let mut menu = Menu::new("Speed", StickAround);
        for (speed, label) in [(0usize, "Paused"), (5, "Glacial"), (25, "Slow"), (125, "Normal"), (625, "Fast")] {
            menu = menu.action(label, ReturnToRoot, ALWAYS, Action::Crucible(CrucibleAction::SetSpeed(speed)));
        }
        menu
    }

    pub fn root_menu(fabric_menu: Menu) -> Menu {
        Menu::new("Welcome", StickAround)
            .submenu(ALWAYS, fabric_menu)
            .action("Muscle test", StickAround,
                    |env| env.crucible_state.experimenting,
                    Action::Crucible(CrucibleAction::ActivateMuscles))
            .submenu(ALWAYS, Menu::new("Settings", StickAround)
                .submenu(ALWAYS, Menu::speed_menu())
                .submenu(ALWAYS, Menu::new("Camera", StickAround)
                    .action("Midpoint", ReturnToRoot, ALWAYS, Action::Scene(SceneAction::WatchMidpoint))
                    .action("Origin", ReturnToRoot, ALWAYS, Action::Scene(SceneAction::WatchOrigin)),
                ),
            )
    }
}

const ALWAYS: fn(&MenuContext) -> bool = |_| true;
