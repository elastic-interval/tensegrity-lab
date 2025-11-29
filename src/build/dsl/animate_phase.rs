use crate::units::{Amplitude, Seconds};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MuscleSpec {
    Alpha,
    Omega,
}

impl MuscleSpec {
    pub fn to_surface(self, joint: usize, surface: (f32, f32)) -> Muscle {
        Muscle {
            direction: self,
            attachment: MuscleAttachment::ToSurface { joint, surface },
        }
    }

    pub fn between(self, alpha: usize, omega: usize) -> Muscle {
        Muscle {
            direction: self,
            attachment: MuscleAttachment::Between { alpha: alpha, omega: omega },
        }
    }
}

#[derive(Debug, Clone)]
pub enum MuscleAttachment {
    ToSurface { joint: usize, surface: (f32, f32) },
    Between { alpha: usize, omega: usize },
}

#[derive(Debug, Clone)]
pub struct Muscle {
    pub direction: MuscleSpec,
    pub attachment: MuscleAttachment,
}

#[derive(Debug, Clone)]
pub struct AnimatePhase {
    pub period: Seconds,
    pub amplitude: Amplitude,
    pub muscles: Vec<Muscle>,
}
