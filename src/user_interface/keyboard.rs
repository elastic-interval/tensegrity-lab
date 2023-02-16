use std::cell::RefCell;
use std::fmt::{Display, Formatter};
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

#[derive(Debug, Clone)]
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

    pub fn action(&self, keycode_pressed: &VirtualKeyCode, action_queue: &RefCell<Vec<Action>>) -> Option<Self>  {
        let mut current = self.current.clone();
        if keycode_pressed == &VirtualKeyCode::Escape {
            current.clear();
            current.push(self.menu.clone());
            return Some(self.clone());
        };
        let action = current
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
                    current.clear();
                    current.push(self.menu.clone());
                    return action.clone();
                }
                if submenu.is_empty() {
                    panic!("expected submenu");
                }
                current.push(menu.clone());
                None
            });
        if let Some(action) = action  {
            action_queue.borrow_mut().push(action);
        };
        Some(Self {
            menu: self.menu.clone(),
            current,
        })
    }
}