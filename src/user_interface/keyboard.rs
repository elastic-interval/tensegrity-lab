use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use iced_wgpu::Renderer;
use iced_winit::Element;
use iced_winit::widget::{Button, Row, Text};
use winit::event::VirtualKeyCode;
use winit::event::VirtualKeyCode::{*};

use crate::user_interface::{Action, ControlMessage};
use crate::user_interface::control_state::{Component, format_row};

#[derive(Debug, Clone)]
pub struct Menu {
    pub label: String,
    pub keycode: Option<VirtualKeyCode>,
    pub submenu: Vec<Menu>,
    pub action: Option<Action>,
}

impl Menu {
    pub fn new(label: &str, submenu: Vec<Menu>) -> Self {
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
        Self { label: label.to_string(), keycode: None, submenu, action: None }
    }

    pub fn action(label: &str, action: Action) -> Self {
        Self { label: label.to_string(), keycode: None, action: Some(action), submenu: vec![] }
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

#[derive(Debug, Clone)]
pub enum KeyboardMessage {
    KeyPressed(VirtualKeyCode),
    SelectSubmenu(Menu),
    SelectUpperMenu,
    SelectRootMenu,
    SubmitAction(Action),
}

impl From<KeyboardMessage> for ControlMessage {
    fn from(value: KeyboardMessage) -> Self {
        ControlMessage::Keyboard(value)
    }
}

#[derive(Debug, Clone)]
pub struct Keyboard {
    menu: Menu,
    current: Vec<Menu>,
}

impl Component for Keyboard {
    type Message = KeyboardMessage;

    fn update(&mut self, message: Self::Message) -> Option<Action> {
        match message {
            KeyboardMessage::SubmitAction(action) => {
                self.current.clear();
                self.current.push(self.menu.clone());
                return Some(action);
            }
            KeyboardMessage::KeyPressed(key_code) => {
                let (current, action) = self.key_pressed(&key_code);
                self.current = current;
                return action;
            }
            KeyboardMessage::SelectSubmenu(menu) => {
                self.current.push(menu);
            }
            KeyboardMessage::SelectUpperMenu => {
                self.current.pop();
            }
            KeyboardMessage::SelectRootMenu => {
                self.current.clear();
                self.current.push(self.menu.clone());
            }
        }
        None
    }

    fn element(&self) -> Element<'_, ControlMessage, Renderer> {
        let mut row = Row::new();
        row = row.push(Text::new(&self.current.last().unwrap().label));
        for item in &self.current.last().unwrap().submenu {
            row = row.push(
                Button::new(Text::new(item.label.clone()))
                    .on_press(
                        match &item.action {
                            None => KeyboardMessage::SelectSubmenu(item.clone()),
                            Some(action) => KeyboardMessage::SubmitAction(action.clone()),
                        }.into()
                    )
            );
        }
        format_row(row)
    }
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

    pub fn key_pressed(&self, keycode_pressed: &VirtualKeyCode) -> (Vec<Menu>, Option<Action>) {
        let mut current = self.current.clone();
        if keycode_pressed == &Escape {
            current.clear();
            current.push(self.menu.clone());
            return (current, None);
        };
        let action = current
            .last()
            .unwrap()
            .clone()
            .submenu
            .iter()
            .find_map(|menu| {
                let Menu { keycode, action, submenu, .. } = menu;
                if keycode.unwrap() != *keycode_pressed {
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
        (current, action)
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