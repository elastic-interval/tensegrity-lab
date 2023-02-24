use std::collections::HashSet;
use std::fmt::{Display, Formatter};

use winit::event::VirtualKeyCode;
use winit::event::VirtualKeyCode::{*};

use crate::build::tenscript::{FabricPlan, FaceAlias, Library};
use crate::crucible::CrucibleAction;
use crate::fabric::face::FaceRotation;
use crate::scene::SceneAction;
use crate::user_interface::{Action, MenuChoice};
use crate::user_interface::control_state::VisibleControl;

#[derive(Debug, Clone)]
pub struct Menu {
    pub label: String,
    pub keycode: Option<VirtualKeyCode>,
    pub submenu: Vec<Menu>,
    pub action: Option<Action>,
    pub last_action: bool,
}

impl Menu {
    pub fn submenu(label: &str, submenu: Vec<Menu>) -> Self {
        let mut used = HashSet::new();
        let submenu = submenu
            .into_iter()
            .map(|menu| {
                let (keycode, prefix) = label_key_code(menu.label.as_str(), &used);
                used.insert(keycode);
                let keycode = Some(keycode);
                let mut label = prefix;
                label.push_str(menu.label.as_str());
                Menu { keycode, label, ..menu }
            })
            .collect();
        Self { label: label.to_string(), keycode: None, submenu, action: None, last_action: false }
    }

    pub fn action(label: &str, action: Action) -> Self {
        Self { label: label.to_string(), keycode: None, action: Some(action), submenu: vec![], last_action: false }
    }

    pub fn last_action(label: &str, action: Action) -> Self {
        Self { label: label.to_string(), keycode: None, action: Some(action), submenu: vec![], last_action: true }
    }

    pub fn select(menu_choice: MenuChoice) -> Menu {
        match menu_choice {
            MenuChoice::Root => Menu::root_menu(),
            MenuChoice::Tinker => Menu::tinker_menu(),
        }
    }

    fn fabric_menu(fabrics: &[FabricPlan], below: Vec<String>) -> Vec<Menu> {
        let sub_fabrics: Vec<_> = fabrics
            .iter()
            .filter(|fabric| {
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
            unique
                .iter()
                .map(|first| {
                    let mut new_below = below.clone();
                    new_below.push(first.clone());
                    Menu::submenu(first.as_str(), Menu::fabric_menu(fabrics, new_below))
                })
                .collect()
        } else {
            sub_fabrics
                .into_iter()
                .map(|fabric_plan| {
                    let label = fabric_plan.name.last().unwrap();
                    Menu::action(label.as_str(), Action::Crucible(CrucibleAction::BuildFabric(fabric_plan.clone())))
                })
                .collect()
        }
    }

    fn speed_menu() -> Vec<Menu> {
        [(0usize, "Paused"), (5, "Glacial"), (25, "Slow"), (125, "Normal"), (625, "Fast")]
            .into_iter()
            .map(|(speed, label)|
                Menu::action(label, Action::Crucible(CrucibleAction::SetSpeed(speed))))
            .collect()
    }

    fn root_menu() -> Menu {
        Menu::submenu("Tensegrity Lab", vec![
            Menu::submenu("Fabric", Menu::fabric_menu(&Library::standard().fabrics, Vec::new())),
            Menu::action("Tinker", Action::Crucible(CrucibleAction::StartTinkering)),
            Menu::submenu("Speed", Menu::speed_menu()),
            Menu::submenu("Camera", vec![
                Menu::action("Midpoint", Action::Scene(SceneAction::WatchMidpoint)),
                Menu::action("Origin", Action::Scene(SceneAction::WatchOrigin)),
            ]),
            Menu::submenu("Widget", vec![
                Menu::action("Gravity", Action::ShowControl(VisibleControl::Gravity)),
                Menu::action("Strain threshold", Action::ShowControl(VisibleControl::StrainThreshold)),
                Menu::action("Clear", Action::ShowControl(VisibleControl::Nothing)),
            ]),
            Menu::submenu("Etc", vec![
                Menu::action("Debug toggle", Action::ToggleDebug),
            ]),
        ])
    }

    fn tinker_menu() -> Menu {
        Menu::submenu("Tinker", vec![
            Menu::action("Connect", Action::Connect),
            Menu::action("Join", Action::JoinFaces),
            Menu::action("Revert", Action::Revert),
            Menu::submenu("Add", vec![
                Menu::action("Single", Action::ProposeBrick { alias: FaceAlias::single("Single"), face_rotation: FaceRotation::Zero }),
                Menu::action("Omni", Action::ProposeBrick { alias: FaceAlias::single("Omni"), face_rotation: FaceRotation::Zero }),
                Menu::action("Torque", Action::ProposeBrick { alias: FaceAlias::single("Torque"), face_rotation: FaceRotation::Zero }),
                Menu::action("Torque120", Action::ProposeBrick { alias: FaceAlias::single("Torque"), face_rotation: FaceRotation::OneThird }),
                Menu::action("Torque240", Action::ProposeBrick { alias: FaceAlias::single("Torque"), face_rotation: FaceRotation::TwoThirds }),
                Menu::last_action("Connect", Action::Connect),
                Menu::last_action("Revert", Action::Revert),
            ]),
            Menu::last_action("Pretense", Action::Crucible(CrucibleAction::StartPretensing)),
        ])
    }
}

impl Display for Menu {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let Menu { label, submenu, .. } = self;
        let choices = submenu
            .iter()
            .map(|Menu { label, .. }| label.clone())
            .collect::<Vec<String>>()
            .join(" ");
        write!(f, "{label}: {choices}")
    }
}

fn label_key_code(label: &str, used: &HashSet<VirtualKeyCode>) -> (VirtualKeyCode, String) {
    label
        .chars()
        .find_map(|ch| {
            let key_code = to_key_code(ch)?;
            (!used.contains(&key_code))
                .then_some((key_code, format!("{}: ", ch.to_ascii_uppercase())))
        })
        .unwrap()
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