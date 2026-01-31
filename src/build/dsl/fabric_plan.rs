#![allow(clippy::result_large_err)]

use crate::build::dsl::animate_phase::{Actuator, AnimatePhase, Waveform};
use crate::build::dsl::build_phase::BuildPhase;
use crate::build::dsl::fabric_library::FabricName;
use crate::build::dsl::fall_phase::FallPhase;
use crate::build::dsl::grav_pretense_phase::GravPretensePhase;
use crate::build::dsl::pretense_phase::PretensePhase;
use crate::build::dsl::settle_phase::SettlePhase;
use crate::build::dsl::shape_phase::ShapePhase;
use crate::fabric::FabricDimensions;

use crate::units::{Percent, Seconds};

#[derive(Debug, Clone)]
pub struct FabricPlan {
    pub name: FabricName,
    pub build_phase: BuildPhase,
    pub shape_phase: ShapePhase,
    pub pretense_phase: PretensePhase,
    pub fall_phase: FallPhase,
    pub settle_phase: Option<SettlePhase>,
    pub grav_pretense_phase: Option<GravPretensePhase>,
    pub animate_phase: Option<AnimatePhase>,
    pub dimensions: FabricDimensions,
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

    pub fn grav_pretense(self, seconds: Seconds) -> GravPretenseBuilder {
        GravPretenseBuilder {
            plan: self,
            phase: GravPretensePhase {
                seconds: Some(seconds),
                min_push_strain: None,
                max_push_strain: None,
            },
        }
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
    pub fn actuators<const N: usize>(mut self, actuators: [Actuator; N]) -> FabricPlan {
        self.phase.actuators = actuators.to_vec();
        self.plan.animate_phase = Some(self.phase);
        self.plan
    }
}

/// Builder for configuring gravitational pretensing with chained methods
pub struct GravPretenseBuilder {
    plan: FabricPlan,
    phase: GravPretensePhase,
}

impl GravPretenseBuilder {
    /// Set the target minimum compression for push intervals (default 1%)
    pub fn min_push_strain(mut self, strain: Percent) -> Self {
        self.phase.min_push_strain = Some(strain.as_factor());
        self
    }

    /// Set the maximum compression per extension round (default 3%)
    pub fn max_push_strain(mut self, strain: Percent) -> Self {
        self.phase.max_push_strain = Some(strain.as_factor());
        self
    }

    /// Terminal method: finalize gravitational pretensing configuration
    pub fn done(mut self) -> FabricPlan {
        self.plan.grav_pretense_phase = Some(self.phase);
        self.plan
    }

    /// Continue to animation configuration
    pub fn animate(mut self) -> AnimateBuilder {
        self.plan.grav_pretense_phase = Some(self.phase);
        self.plan.animate()
    }
}
