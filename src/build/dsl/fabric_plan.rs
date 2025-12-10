#![allow(clippy::result_large_err)]

use crate::build::dsl::animate_phase::{AnimatePhase, Waveform, Actuator};
use crate::build::dsl::build_phase::BuildPhase;
use crate::build::dsl::fall_phase::FallPhase;
use crate::build::dsl::settle_phase::SettlePhase;
use crate::build::dsl::pretense_phase::PretensePhase;
use crate::build::dsl::shape_phase::ShapePhase;

use crate::units::{Millimeters, Percent, Seconds};

#[derive(Debug, Clone)]
pub struct FabricPlan {
    pub name: String,
    pub build_phase: BuildPhase,
    pub shape_phase: ShapePhase,
    pub pretense_phase: PretensePhase,
    pub fall_phase: FallPhase,
    pub settle_phase: Option<SettlePhase>,
    pub animate_phase: Option<AnimatePhase>,
    pub scale: f32,
    pub altitude: Millimeters,
}

impl FabricPlan {
    pub fn fall(mut self, seconds: Seconds) -> Self {
        self.fall_phase = FallPhase { seconds };
        self
    }

    pub fn settle(mut self, seconds: Seconds) -> Self {
        self.settle_phase = Some(SettlePhase { seconds });
        self
    }

    pub fn animate_sine(
        mut self,
        period: Seconds,
        amplitude: Percent,
        stiffness: Percent,
        actuators: Vec<Actuator>,
    ) -> Self {
        self.animate_phase = Some(AnimatePhase {
            period,
            amplitude,
            waveform: Waveform::Sine,
            stiffness,
            actuators,
        });
        self
    }

    pub fn animate_pulse(
        mut self,
        period: Seconds,
        amplitude: Percent,
        duty_cycle: f32,
        stiffness: Percent,
        actuators: Vec<Actuator>,
    ) -> Self {
        self.animate_phase = Some(AnimatePhase {
            period,
            amplitude,
            waveform: Waveform::Pulse { duty_cycle },
            stiffness,
            actuators,
        });
        self
    }
}
