use std::ops::ControlFlow;

use crate::build::growth::Growth;
use crate::build::plan_runner::Stage::{*};
use crate::build::tenscript::FabricPlan;
use crate::fabric::Fabric;
use crate::fabric::physics::presets::LIQUID;

#[derive(Clone, Debug, Copy, PartialEq)]
enum Stage {
    Initialize,
    GrowStep,
    GrowApproach,
    GrowCalm,
    Shaping,
    Completed,
}

pub struct PlanRunner {
    stage: Stage,
    pub growth: Growth,
}

impl PlanRunner {
    pub fn new(fabric_plan: FabricPlan) -> Self {
        Self {
            stage: Initialize,
            growth: Growth::new(fabric_plan),
        }
    }
}

impl PlanRunner {
    pub fn iterate(&mut self, fabric: &mut Fabric) {
        fabric.iterate(&LIQUID);
        if fabric.progress.is_busy() {
            return;
        }
        let (next_stage, countdown) = match self.stage {
            Initialize => {
                self.growth.init(fabric);
                (GrowApproach, 0)
            }
            GrowStep => {
                if self.growth.is_growing() {
                    self.growth.growth_step(fabric);
                    (GrowApproach, 0)
                } else if self.growth.needs_shaping() {
                    (Shaping, 0)
                } else {
                    (Completed, 0)
                }
            }
            GrowApproach =>
                (GrowCalm, 1500),
            GrowCalm =>
                (GrowStep, 1500),
            Shaping =>
                match self.growth.shaping_step(fabric) {
                    ControlFlow::Continue(count) =>
                        (Shaping, count),
                    ControlFlow::Break(()) =>
                        (Completed, 500),
                }
            Completed =>
                (Completed, 0),
        };
        fabric.progress.start(countdown);
        self.stage = next_stage;
    }

    pub fn is_done(&self) -> bool {
        self.stage == Completed
    }
}
