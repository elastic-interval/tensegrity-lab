use crate::build::dsl::joint_names::Link;
use crate::fabric::joint_path::JointPath;
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

/// Builder for creating actuators at a specific phase offset
pub struct PhaseBuilder {
    phase_offset: Percent,
}

impl PhaseBuilder {
    /// New actuators along a link, one for each of its images, all at this
    /// phase.
    pub fn between(self, link: Link) -> Actuator {
        Actuator { phase_offset: self.phase_offset, attachment: ActuatorAttachment::Between { link } }
    }

    /// Drive the intervals already along a link (e.g. guy lines), one for
    /// each of its images, all at this phase; they are restored when the
    /// animation stops.
    pub fn interval(self, link: Link) -> Actuator {
        Actuator { phase_offset: self.phase_offset, attachment: ActuatorAttachment::Existing { link } }
    }

    /// Create an actuator from a joint to a surface point (accepts &str or JointPath)
    pub fn surface(self, joint: impl Into<JointPath>, point: (f32, f32)) -> Actuator {
        Actuator {
            phase_offset: self.phase_offset,
            attachment: ActuatorAttachment::ToSurface {
                joint: joint.into(),
                point,
            },
        }
    }
}

/// Create an actuator at the specified phase offset (0% = start of cycle, 50% = half cycle)
pub fn phase(offset: Percent) -> PhaseBuilder {
    PhaseBuilder {
        phase_offset: offset,
    }
}

#[derive(Debug, Clone)]
pub enum ActuatorAttachment {
    ToSurface {
        joint: JointPath,
        point: (f32, f32),
    },
    Between {
        link: Link,
    },
    Existing {
        link: Link,
    },
}

#[derive(Debug, Clone)]
pub struct Actuator {
    pub phase_offset: Percent,
    pub attachment: ActuatorAttachment,
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
