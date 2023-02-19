use iced_wgpu::Renderer;
use iced_winit::Element;
use iced_winit::widget::{Button, Row, Text};
use winit::event::VirtualKeyCode;
use winit::event::VirtualKeyCode::{*};

use crate::user_interface::{Action, ControlMessage, MenuChoice};
use crate::user_interface::control_state::{Component, format_row};
use crate::user_interface::menu::Menu;

#[derive(Debug, Clone)]
pub enum KeyboardMessage {
    KeyPressed(VirtualKeyCode),
    SelectSubmenu(Menu),
    SelectUpperMenu,
    SelectMenu(MenuChoice),
    SubmitAction(Action),
}

impl From<KeyboardMessage> for ControlMessage {
    fn from(value: KeyboardMessage) -> Self {
        ControlMessage::Keyboard(value)
    }
}

#[derive(Debug, Clone)]
pub struct Keyboard {
    current: Vec<Menu>,
}

impl Component for Keyboard {
    type Message = KeyboardMessage;

    fn update(&mut self, message: Self::Message) -> Option<Action> {
        match message {
            KeyboardMessage::SubmitAction(action) => {
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
                if self.current.len() > 1 {
                    self.current.pop();
                }
            }
            KeyboardMessage::SelectMenu(menu_choice) => {
                self.set_menu(Menu::select(menu_choice));
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
        Self { current: vec![Menu::select(MenuChoice::Root)] }
    }
}

impl Keyboard {
    pub fn set_menu(&mut self, menu: Menu) {
        self.current.clear();
        self.current.push(menu);
    }

    pub fn current(&self) -> Menu {
        self.current.last().unwrap().clone()
    }

    pub fn key_pressed(&self, keycode_pressed: &VirtualKeyCode) -> (Vec<Menu>, Option<Action>) {
        let mut current = self.current.clone();
        if keycode_pressed == &Escape || keycode_pressed == &Back {
            if current.len() > 1 {
                current.pop();
            }
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
                    if current.len() > 1 {
                        current.pop();
                    }
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
