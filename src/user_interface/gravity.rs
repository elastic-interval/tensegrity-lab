use crate::user_interface::ControlMessage;

#[derive(Debug, Clone)]
pub enum GravityMessage {
    NuanceChanged(f32),
    Reset,
}

impl From<GravityMessage> for ControlMessage {
    fn from(value: GravityMessage) -> Self {
        ControlMessage::Gravity(value)
    }
}

#[derive(Clone, Debug)]
pub struct Gravity {
    nuance: f32,
    default: f32,
    min_gravity: f32,
    max_gravity: f32,
}

impl Gravity {
    pub fn new(default: f32) -> Self {
        let min_gravity = default * 0.001;
        let max_gravity = default * 3.0;
        let nuance = (default - min_gravity) / (max_gravity - min_gravity);
        Self {
            nuance,
            default,
            min_gravity,
            max_gravity,
        }
    }
}
