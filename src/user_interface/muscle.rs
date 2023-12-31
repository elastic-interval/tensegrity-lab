use crate::user_interface::ControlMessage;

#[derive(Debug, Clone)]
pub enum MuscleMessage {
    NuanceChanged(f32),
    Reset,
}

impl From<MuscleMessage> for ControlMessage {
    fn from(value: MuscleMessage) -> Self {
        ControlMessage::Muscle(value)
    }
}

#[derive(Clone, Debug)]
pub struct Muscle {
    nuance: f32,
}

impl Muscle {
    pub fn new() -> Self {
        Self { nuance: 0.5 }
    }
}
