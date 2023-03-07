use iced_wgpu::Renderer;
use iced_winit::{Color, Element};
use iced_winit::widget::{Button, Row, Text};
use winit::event::VirtualKeyCode;
use crate::build::tenscript::fabric_library::FabricLibrary;

use crate::user_interface::{Action, ControlMessage, MenuAction, MenuEnvironment};
use crate::user_interface::control_state::{Component, format_row};
use crate::user_interface::menu::Menu;

#[derive(Debug, Clone)]
pub enum KeyboardMessage {
    KeyPressed(VirtualKeyCode),
    SelectSubmenu(Menu),
    SelectMenu(MenuAction),
    SubmitAction { action: Action, menu_action: MenuAction },
    SetEnvironment(MenuEnvironment),
    FreshLibrary(FabricLibrary),
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
    fabric_menu: Menu,
}

impl Component for Keyboard {
    type Message = KeyboardMessage;

    fn update(&mut self, message: Self::Message) -> Option<Action> {
        match message {
            KeyboardMessage::SubmitAction { action, menu_action } => {
                Self::exit(menu_action, &mut self.current, self.fabric_menu.clone());
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
                    MenuAction::StickAround => {}
                    MenuAction::ReturnToRoot => {
                        self.reset_menu(Menu::root_menu(self.fabric_menu.clone()));
                    }
                    MenuAction::TinkerMenu => {
                        self.reset_menu(Menu::tinker_menu());
                    }
                    MenuAction::UpOneLevel => {
                        Self::menu_back(&mut self.current);
                    }
                }
            }
            KeyboardMessage::SetEnvironment(environment) => {
                self.environment = environment;
            }
            KeyboardMessage::FreshLibrary(library) => {
                self.fabric_menu = Menu::fabric_menu(&library.fabric_plans);
                self.current = vec![Menu::root_menu(self.fabric_menu.clone())];
            }
        }
        None
    }

    fn element(&self) -> Element<'_, ControlMessage, Renderer> {
        let mut row = Row::new();
        let current = self.current.last().unwrap();
        match current.menu_action {
            MenuAction::StickAround => {
                row = row.push(Text::new(&self.current.last().unwrap().label));
            }
            MenuAction::ReturnToRoot | MenuAction::TinkerMenu => {
                unimplemented!()
            }
            MenuAction::UpOneLevel => {
                row = row.push(
                    Button::new(
                        Text::new(&self.current.last().unwrap().label)
                            .style(Color::from_rgb(0.0, 1.0, 0.0))
                    ).on_press(KeyboardMessage::SelectMenu(MenuAction::UpOneLevel).into())
                );
            }
        };
        for item in &self.current.last().unwrap().submenu_in(&self.environment) {
            row = row.push(
                Button::new(Text::new(item.label()))
                    .on_press(
                        match &item.action {
                            None => KeyboardMessage::SelectSubmenu(item.clone()),
                            Some(action) => {
                                KeyboardMessage::SubmitAction {
                                    action: action.clone(),
                                    menu_action: item.menu_action,
                                }
                            }
                        }.into()
                    )
            );
        }
        format_row(row)
    }
}

impl Keyboard {

    pub fn new(environment: MenuEnvironment) -> Self {
        let fabric_menu = environment.fabric_menu.clone();
        let current = vec![Menu::root_menu(fabric_menu.clone())];
        Self { current, environment, fabric_menu }

    }

    pub fn reset_menu(&mut self, menu: Menu) {
        self.current.clear();
        self.current.push(menu);
    }

    pub fn current(&self) -> Menu {
        self.current.last().unwrap().clone()
    }

    pub fn key_pressed(&mut self, keycode_pressed: VirtualKeyCode) -> (Vec<Menu>, Option<Action>) {
        let mut current = self.current.clone();
        let action = current
            .last()
            .unwrap()
            .clone()
            .submenu_in(&self.environment)
            .into_iter()
            .find_map(|menu| {
                let Menu { label, keycode, action, menu_action, .. } = &menu;
                let (code, _) = keycode.clone().unwrap_or_else(|| panic!("No keycode for {label}"));
                if code != keycode_pressed {
                    return None;
                }
                if action.is_some() {
                    Self::exit(*menu_action, &mut current, self.fabric_menu.clone());
                    return action.clone();
                }
                current.push(menu);
                None
            });
        (current, action)
    }

    fn menu_back(current: &mut Vec<Menu>) {
        if current.len() > 1 {
            current.pop();
        }
    }

    fn exit(menu_action: MenuAction, current: &mut Vec<Menu>, fabric_menu: Menu) {
        match menu_action {
            MenuAction::StickAround => {}
            MenuAction::UpOneLevel => {
                Self::menu_back(current);
            }
            MenuAction::ReturnToRoot => {
                current.clear();
                current.push(Menu::root_menu(fabric_menu));
            }
            MenuAction::TinkerMenu => {
                current.clear();
                current.push(Menu::tinker_menu());
            }
        }
    }
}
