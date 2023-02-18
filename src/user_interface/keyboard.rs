use iced_wgpu::Renderer;
use iced_winit::Element;
use iced_winit::widget::{Button, Row, Text};
use winit::event::VirtualKeyCode;
use winit::event::VirtualKeyCode::{*};

use crate::user_interface::{Action, ControlMessage};
use crate::user_interface::control_state::{Component, format_row};
use crate::user_interface::menu::{action_menu, Menu};

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

impl Default for Keyboard {
    fn default() -> Self {
        let menu = action_menu();
        Self {
            current: vec!(menu.clone()),
            menu,
        }
    }
}

impl Keyboard {
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
