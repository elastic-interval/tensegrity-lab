use winit::event::VirtualKeyCode;
use winit::event::VirtualKeyCode::{*};
use crate::user_interface::Action;
use crate::user_interface::Action::{*};

#[derive( Debug, Default)]
pub struct Keyboard {
    pub gumby: Option<usize>,
}

impl Keyboard {
    pub fn action(&mut self, keycode: &VirtualKeyCode) -> Option<Action> {
        Some(match keycode {
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
            _ => return None,
        })
    }
}