use crate::units::{Percent, Seconds};

/// Waveform shape for actuator control signal
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Waveform {
    /// Smooth sinusoidal oscillation
    Sine,
    /// Square wave pulse with configurable duty cycle (proportion "on")
    Pulse { duty_cycle: Percent },
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

impl Actuator {
    pub fn alpha_between(joint_a: usize, joint_b: usize) -> Self {
        Actuator {
            direction: ActuatorSpec::Alpha,
            attachment: ActuatorAttachment::Between { alpha: joint_a, omega: joint_b },
        }
    }

    pub fn omega_between(joint_a: usize, joint_b: usize) -> Self {
        Actuator {
            direction: ActuatorSpec::Omega,
            attachment: ActuatorAttachment::Between { alpha: joint_a, omega: joint_b },
        }
    }

    pub fn alpha_to_surface(joint: usize, surface: (f32, f32)) -> Self {
        Actuator {
            direction: ActuatorSpec::Alpha,
            attachment: ActuatorAttachment::ToSurface { joint, surface },
        }
    }

    pub fn omega_to_surface(joint: usize, surface: (f32, f32)) -> Self {
        Actuator {
            direction: ActuatorSpec::Omega,
            attachment: ActuatorAttachment::ToSurface { joint, surface },
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnimatePhase {
    pub period: Seconds,
    pub amplitude: Percent,
    pub waveform: Waveform,
    pub stiffness: Percent,
    pub actuators: Vec<Actuator>,
}

impl AnimatePhase {
    pub(crate) fn new() -> Self {
        AnimatePhase {
            period: Seconds(1.0),
            amplitude: Percent(1.0),
            waveform: Waveform::Sine,
            stiffness: Percent(10.0),
            actuators: Vec::new(),
        }
    }
}
