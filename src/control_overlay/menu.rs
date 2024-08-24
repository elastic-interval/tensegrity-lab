use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use crate::control_overlay::menu::MenuContent::{Empty, Event, Submenu};
use crate::messages::LabEvent;

pub type EventMap = HashMap<LabEventKey, LabEvent>;

pub struct MenuBuilder {
    event_index: usize,
    root_item: MenuItem,
    events: EventMap,
}

impl Default for MenuBuilder {
    fn default() -> Self {
        Self {
            event_index: 1000,
            root_item: MenuItem {
                label: "Menu".to_string(),
                content: Submenu(vec![]),
            },
            events: EventMap::new(),
        }
    }
}

impl MenuBuilder {
    pub fn add_to_root(&mut self, menu_item: MenuItem) {
        if let Submenu(content) = &mut self.root_item.content {
            content.push(menu_item)
        }
    }

    pub fn event_item(&mut self, label: String, lab_event: LabEvent) -> MenuItem {
        MenuItem {
            label: label.to_string(),
            content: Event(self.insert(lab_event)),
        }
    }

    pub fn load_fabric_item(&mut self, list: Vec<String>) -> MenuItem {
        let items = list
            .iter()
            .map(|name| self.event_item(name.clone(), LabEvent::LoadFabric(name.clone())))
            .collect();
        MenuItem {
            label: "Load structure".to_string(),
            content: Submenu(items)
        }
    }

    fn insert(&mut self, lab_event: LabEvent) -> LabEventKey {
        let index = self.event_index + 1;
        let key = LabEventKey(index);
        self.event_index = index;
        self.events.insert(key.clone(), lab_event);
        key
    }

    pub fn event_map(self) -> EventMap {
        self.events
    }

    pub fn menu(&self) -> Menu {
        Menu {
            root_item: self.root_item.clone(),
        }
    }
}

#[derive(Clone)]
pub struct Menu {
    root_item: MenuItem,
}

impl Menu {
    pub fn root(&self) -> &MenuItem {
        &self.root_item
    }
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: String,
    pub content: MenuContent,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct LabEventKey(usize);

#[derive(Debug, Clone)]
pub enum MenuContent {
    Empty,
    Event(LabEventKey),
    Submenu(Vec<MenuItem>),
}

impl MenuItem {
    pub fn submenu(label: &'static str) -> Self {
        Self{
            label: label.to_string(),
            content: Submenu(vec![]),
        }
    }
    
    pub fn fake_add(self, label: &'static str) -> Self {
        self.add_item(MenuItem{
            label: label.to_string(),
            content: Empty,
        })
    } 
    
    pub fn add_item(mut self, item: MenuItem) -> Self {
        match &mut self.content {
            Submenu(items) => items.push(item),
            _ => panic!("Illegal add")
        }
        self
    }
}

impl Display for MenuItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

