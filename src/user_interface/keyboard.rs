use iced_wgpu::Renderer;
use iced_winit::{Color, Element};
use iced_winit::widget::{Button, Row, Text};
use winit::event::VirtualKeyCode;
use winit::event::VirtualKeyCode::{*};

use crate::user_interface::{Action, ControlMessage, MenuAction, MenuEnvironment};
use crate::user_interface::control_state::{Component, format_row};
use crate::user_interface::menu::Menu;

#[derive(Debug, Clone)]
pub enum KeyboardMessage {
    KeyPressed(VirtualKeyCode),
    SelectSubmenu(Menu),
    SelectMenu(MenuAction),
    SubmitAction(Action),
    SubmitExitAction(Action),
    SetEnvironment(MenuEnvironment),
}

impl From<KeyboardMessage> for ControlMessage {
    fn from(value: KeyboardMessage) -> Self {
        ControlMessage::Keyboard(value)
    }
}

#[derive(Debug, Clone)]
pub struct Keyboard {
    current: Vec<Menu>,
    environment: MenuEnvironment,
}

impl Component for Keyboard {
    type Message = KeyboardMessage;

    fn update(&mut self, message: Self::Message) -> Option<Action> {
        match message {
            KeyboardMessage::SubmitAction(action) => {
                return Some(action);
            }
            KeyboardMessage::SubmitExitAction(action) => {
                self.menu_back();
                return Some(action);
            }
            KeyboardMessage::KeyPressed(key_code) => {
                let (current, action) = self.key_pressed(key_code);
                self.current = current;
                return action;
            }
            KeyboardMessage::SelectSubmenu(menu) => {
                self.current.push(menu);
            }
            KeyboardMessage::SelectMenu(menu_action) => {
                match menu_action {
                    MenuAction::ReturnToRoot => {
                        self.reset_menu(Menu::root_menu());
                    }
                    MenuAction::TinkerMenu => {
                        self.reset_menu(Menu::tinker_menu());
                    }
                    MenuAction::UpOneLevel => {
                        self.menu_back();
                    }
                }
            }
            KeyboardMessage::SetEnvironment(environment) => {
                self.environment = environment;
            }
        }
        None
    }

    fn element(&self) -> Element<'_, ControlMessage, Renderer> {
        let mut row = Row::new();
        row = row.push(
            Button::new(
                Text::new(&self.current.last().unwrap().label)
                    .style(Color::from_rgb(0.0, 1.0, 0.0)))
                .on_press(KeyboardMessage::SelectMenu(MenuAction::UpOneLevel).into())
        );
        for item in &self.current.last().unwrap().submenu_in(self.environment) {
            row = row.push(
                Button::new(Text::new(item.label()))
                    .on_press(
                        match &item.action {
                            None => KeyboardMessage::SelectSubmenu(item.clone()),
                            Some(action) => {
                                if item.exit_action {
                                    KeyboardMessage::SubmitExitAction(action.clone())
                                } else {
                                    KeyboardMessage::SubmitAction(action.clone())
                                }
                            }
                        }.into()
                    )
            );
        }
        format_row(row)
    }
}

impl Default for Keyboard {
    fn default() -> Self {
        let environment = MenuEnvironment::default();
        let current = vec![Menu::root_menu()];
        Self { current, environment }
    }
}

impl Keyboard {
    pub fn reset_menu(&mut self, menu: Menu) {
        self.current.clear();
        self.current.push(menu);
    }

    pub fn current(&self) -> Menu {
        self.current.last().unwrap().clone()
    }

    pub fn key_pressed(&self, keycode_pressed: VirtualKeyCode) -> (Vec<Menu>, Option<Action>) {
        let mut current = self.current.clone();
        if matches!(keycode_pressed, Escape | Back) {
            if current.len() > 1 {
                current.pop();
            }
            return (current, None);
        };
        let action = current
            .last()
            .unwrap()
            .clone()
            .submenu_in(self.environment)
            .into_iter()
            .find_map(|menu| {
                let Menu { label, keycode, action, submenu, exit_action } = &menu;
                let (code, _) = keycode.clone().unwrap_or_else(|| panic!("No keycode for {label}"));
                if code != keycode_pressed {
                    return None;
                }
                if action.is_some() {
                    if *exit_action {
                        if current.len() > 1 {
                            current.pop();
                        } else {
                            current = vec![Menu::root_menu()];
                        }
                    }
                    return action.clone();
                }
                if submenu.is_empty() {
                    panic!("expected submenu");
                }
                current.push(menu);
                None
            });
        (current, action)
    }

    fn menu_back(&mut self) {
        if self.current.len() > 1 {
            self.current.pop();
        } else {
            self.current = vec![Menu::root_menu()];
        }
    }
}
