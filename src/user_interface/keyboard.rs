use std::fmt::{Display, Formatter, Pointer};
use winit::event::VirtualKeyCode;

use crate::user_interface::Action;

#[derive(Debug, Clone)]
pub struct Menu {
    pub keycode: VirtualKeyCode,
    pub label: String,
    pub submenu: Vec<Menu>,
    pub action: Option<Action>,
}

impl Menu {
    pub fn new(label: &str, keycode: VirtualKeyCode, submenu: Vec<Menu>) -> Self {
        Self { keycode, label: label.to_string(), submenu, action: None }
    }

    pub fn action(label: &str, keycode: VirtualKeyCode, action: Action) -> Self {
        Self { keycode, label: label.to_string(), action: Some(action), submenu: vec![] }
    }
}

impl Display for Menu {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let Menu{label, submenu,..} = self;
        let choices = submenu
            .iter()
            .map(|Menu{label,..}|label.clone())
            .collect::<Vec<String>>()
            .join(" ");
        write!(f, "{label}: {choices}")
    }
}

#[derive(Debug)]
pub struct Keyboard {
    menu: Menu,
    current: Vec<Menu>,
}

impl Keyboard {
    pub fn new(menu: Menu) -> Self {
        Self {
            current: vec!(menu.clone()),
            menu,
        }
    }

    pub fn current(&self) -> Menu {
        self.current.last().unwrap().clone()
    }

    pub fn enum_action(&mut self, keycode_pressed: &VirtualKeyCode) -> Option<Action> {
        if keycode_pressed == &VirtualKeyCode::Escape {
            self.current.clear();
            self.current.push(self.menu.clone());
        };
        self.current
            .last()
            .unwrap()
            .clone()
            .submenu
            .iter()
            .find_map(|menu| {
                let Menu { keycode, action, submenu,.. } = menu;
                if keycode != keycode_pressed {
                    return None;
                }
                if action.is_some() {
                    self.current.clear();
                    self.current.push(self.menu.clone());
                    return action.clone();
                }
                if submenu.is_empty() {
                    panic!("expected submenu");
                }
                self.current.push(menu.clone());
                None
            })
    }
}