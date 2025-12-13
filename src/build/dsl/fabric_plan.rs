#![allow(clippy::result_large_err)]

use crate::build::dsl::animate_phase::{AnimatePhase, Waveform, Actuator};
use crate::build::dsl::build_phase::BuildPhase;
use crate::build::dsl::fabric_library::FabricName;
use crate::build::dsl::fall_phase::FallPhase;
use crate::build::dsl::settle_phase::SettlePhase;
use crate::build::dsl::pretense_phase::PretensePhase;
use crate::build::dsl::shape_phase::ShapePhase;

use crate::units::{Meters, Percent, Seconds};

#[derive(Debug, Clone)]
pub struct FabricPlan {
    pub name: FabricName,
    pub build_phase: BuildPhase,
    pub shape_phase: ShapePhase,
    pub pretense_phase: PretensePhase,
    pub fall_phase: FallPhase,
    pub settle_phase: Option<SettlePhase>,
    pub animate_phase: Option<AnimatePhase>,
    pub scale: Meters,
    pub altitude: Meters,
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

    pub fn animate(self) -> AnimateBuilder {
        AnimateBuilder {
            plan: self,
            phase: AnimatePhase::new(),
        }
    }
}

/// Builder for configuring animation with chained methods
pub struct AnimateBuilder {
    plan: FabricPlan,
    phase: AnimatePhase,
}

impl AnimateBuilder {
    pub fn period(mut self, period: Seconds) -> Self {
        self.phase.period = period;
        self
    }

    pub fn amplitude(mut self, amplitude: Percent) -> Self {
        self.phase.amplitude = amplitude;
        self
    }

    pub fn stiffness(mut self, stiffness: Percent) -> Self {
        self.phase.stiffness = stiffness;
        self
    }

    pub fn sine(mut self) -> Self {
        self.phase.waveform = Waveform::Sine;
        self
    }

    pub fn pulse(mut self, duty_cycle: Percent) -> Self {
        self.phase.waveform = Waveform::Pulse { duty_cycle };
        self
    }

    /// Terminal method: add actuators and return the completed FabricPlan
    pub fn actuators(mut self, actuators: &[Actuator]) -> FabricPlan {
        self.phase.actuators = actuators.to_vec();
        self.plan.animate_phase = Some(self.phase);
        self.plan
    }
}
