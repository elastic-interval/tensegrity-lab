use crate::units::{Amplitude, Percent, Seconds};

/// Waveform shape for actuator control signal
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Waveform {
    /// Smooth sinusoidal oscillation
    Sine,
    /// Square wave pulse with configurable duty cycle (0.0 to 1.0 proportion "on")
    Pulse { duty_cycle: f32 },
}

impl Default for Waveform {
    fn default() -> Self {
        Waveform::Sine
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActuatorSpec {
    Alpha,
    Omega,
}

impl ActuatorSpec {
    pub fn to_surface(self, joint: usize, surface: (f32, f32)) -> Actuator {
        Actuator {
            direction: self,
            attachment: ActuatorAttachment::ToSurface { joint, surface },
        }
    }

    pub fn between(self, alpha: usize, omega: usize) -> Actuator {
        Actuator {
            direction: self,
            attachment: ActuatorAttachment::Between { alpha, omega },
        }
    }
}

#[derive(Debug, Clone)]
pub enum ActuatorAttachment {
    ToSurface { joint: usize, surface: (f32, f32) },
    Between { alpha: usize, omega: usize },
}

#[derive(Debug, Clone)]
pub struct Actuator {
    pub direction: ActuatorSpec,
    pub attachment: ActuatorAttachment,
}

#[derive(Debug, Clone)]
pub struct AnimatePhase {
    pub period: Seconds,
    pub amplitude: Amplitude,
    pub waveform: Waveform,
    pub stiffness: Percent,
    pub actuators: Vec<Actuator>,
}
