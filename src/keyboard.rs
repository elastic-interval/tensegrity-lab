use winit::event::VirtualKeyCode;
use winit::event::VirtualKeyCode::{*};
use crate::keyboard::KeyAct::{*};

pub enum KeyAct {
    Idle,
    MainMenu,
    ToggleDebug,
    SetSpeed(usize),
    CreateBrick,
    SelectNextFace,
    WatchMidpoint,
    WatchOrigin,
}

#[derive( Debug, Default)]
pub struct Keyboard {
    pub gumby: Option<usize>,
}

impl Keyboard {
    pub fn act(&mut self, keycode: &VirtualKeyCode) -> KeyAct {
        match keycode {
            Escape => MainMenu,
            D => ToggleDebug,
            Key0 => SetSpeed(0),
            Key1 => SetSpeed(1),
            Key2 => SetSpeed(5),
            Key3 => SetSpeed(25),
            Key4 => SetSpeed(125),
            Key5 => SetSpeed(625),
            B => CreateBrick,
            F => SelectNextFace,
            M => WatchMidpoint,
            O => WatchOrigin,
            _ => Idle
        }
    }
}