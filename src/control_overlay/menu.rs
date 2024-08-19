use std::fmt::{Display, Formatter};

use crate::messages::{LabEvent, SceneAction};

pub struct Menu {
    root_item: MenuItem,
}

impl Default for Menu {
    fn default() -> Self {
        let root_item =
            menu("Root")
                .add(menu("one")
                    .add(menu("one one")
                        .add(event("Escapism", LabEvent::Scene(SceneAction::EscapeHappens))))
                    .add(menu("one two")))
                .add(menu("two"));
        Self {
            root_item
        }
    }
}

impl Menu {
    fn _root(self) -> MenuItem {
        self.root_item
    }
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    label: String,
    lab_event: Option<LabEvent>,
    submenu: Vec<MenuItem>,
}

fn menu(label: &'static str) -> MenuItem {
    MenuItem {
        label: label.to_string(),
        lab_event: None,
        submenu: vec![],
    }
}

fn event(label: &'static str, lab_event: LabEvent) -> MenuItem {
    MenuItem {
        label: label.to_string(),
        lab_event: Some(lab_event),
        submenu: vec![],
    }
}

impl MenuItem {
    fn add(mut self, item: MenuItem) -> Self {
        self.submenu.push(item);
        self
    }
    
    fn _event(self) -> LabEvent {
        self.lab_event.unwrap()
    }
}

impl Display for MenuItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

