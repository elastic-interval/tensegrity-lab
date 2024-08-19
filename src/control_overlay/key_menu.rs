use std::fmt::{Display, Formatter};

use winit::event::KeyEvent;
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::messages::{LabEvent, SceneAction};

pub struct KeyMenu {
    root_item: MenuItem,
}

impl Default for KeyMenu {
    fn default() -> Self {
        Self {
            root_item: MenuItem {
                active: false,
                label: "root".to_string(),
                key_code: KeyCode::Escape,
                lab_event: None,
                submenu: vec![],
            },
        }
    }
}

impl KeyMenu {
    pub fn handle_key_event(&mut self, event: KeyEvent) -> Option<LabEvent> {
        if !event.state.is_pressed() {
            return None;
        }
        match event {
            KeyEvent { physical_key: PhysicalKey::Code(code), .. } => {
                if code == KeyCode::Escape {
                    return Some(LabEvent::Scene(SceneAction::EscapeHappens));
                }
                if let Some(item) = self.root_item.submenu
                    .iter()
                    .filter(|item| item.active)
                    .find(|MenuItem { key_code, .. }| *key_code == code) {
                    if item.lab_event.is_some() {
                        return item.lab_event.clone();
                    }
                }
                None
            }
            _ => None
        }
    }
}

pub struct MenuItem {
    active: bool,
    label: String,
    key_code: KeyCode,
    lab_event: Option<LabEvent>,
    submenu: Vec<MenuItem>,
}

impl Display for MenuItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

