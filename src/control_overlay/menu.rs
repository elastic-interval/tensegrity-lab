use std::fmt::{Display, Formatter};
use crate::control_overlay::menu::MenuContent::{Event, Submenu};

use crate::messages::{LabEvent, SceneAction};

#[derive(Clone)]
pub struct Menu {
    pub root_item: MenuItem,
}

impl Menu {
    pub fn with_fabric_list(list: Vec<String>) -> Self {
        let fabric_items: Vec<MenuItem> = list
            .into_iter()
            .map(fabric_item)
            .collect();
        let fabric_menu = MenuItem {
            label: "Load Fabric".to_string(),
            content: Submenu(fabric_items),
        };
        Self {
            root_item: fabric_menu,
            // menu("Root")
            //     .add(fabric_menu)
            //     .add(menu("one")
            //         .add(menu("one one")
            //             .add(event("Bla", LabEvent::Scene(SceneAction::EscapeHappens)))
            //             .add(event("Escapism", LabEvent::Scene(SceneAction::EscapeHappens))))
            //         .add(menu("one two")))
            //     .add(menu("two"))
        }
    }
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: String,
    pub content: MenuContent,
}

#[derive(Debug, Clone)]
pub enum MenuContent {
    Event(LabEvent),
    Submenu(Vec<MenuItem>),
}

fn menu(label: &'static str) -> MenuItem {
    MenuItem {
        label: label.to_string(),
        content: Submenu(vec![]),
    }
}

fn event(label: &'static str, lab_event: LabEvent) -> MenuItem {
    MenuItem {
        label: label.to_string(),
        content: Event(lab_event),
    }
}

fn fabric_item(fabric_name: String) -> MenuItem {
    MenuItem {
        label: fabric_name.clone(),
        content: Event(LabEvent::LoadFabric(fabric_name)),
    }
}

impl MenuItem {
    fn add(mut self, item: MenuItem) -> Self {
        match &mut self.content {
            Submenu(items) => items.push(item),
            _ => panic!("Illegal add")
        }
        self
    }

    fn _event(self) -> LabEvent {
        if let Event(lab_event) = self.content {
            lab_event
        } else {
            panic!("No event here")
        }
    }
}

impl Display for MenuItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

