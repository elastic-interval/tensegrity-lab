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
    ShapingStart,
    ShapingApproach,
    Shaped,
    ShapedApproach,
    ShapingDone,
    ShapingCalm,
    VulcanizeCalm,
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
        let next_stage = match self.stage {
            Initialize => {
                self.growth.init(fabric);
                GrowApproach
            }
            GrowStep => {
                if self.growth.is_growing() {
                    self.growth.growth_step(fabric);
                    GrowApproach
                } else if self.growth.needs_shaping() {
                    self.growth.create_shapers(fabric);
                    ShapingStart
                } else {
                    ShapingDone
                }
            }
            GrowApproach => GrowCalm,
            GrowCalm => GrowStep,
            ShapingStart => ShapingApproach,
            ShapingApproach => Shaped,
            Shaped => {
                self.growth.complete_shapers(fabric);
                ShapedApproach
            }
            ShapedApproach => ShapingDone,
            ShapingDone => ShapingCalm,
            ShapingCalm => {
                self.growth.post_shaping(fabric);
                VulcanizeCalm
            },
            VulcanizeCalm => Completed,
            Completed => Completed,
        };
        let countdown = match next_stage {
            GrowApproach => 1500,
            GrowCalm => 1500,
            ShapingApproach => 25000,
            ShapedApproach => 5000,
            ShapingCalm => 500,
            VulcanizeCalm => 5000,
            Initialize | GrowStep | ShapingStart | Shaped | ShapingDone | Completed => 0,
        };
        fabric.progress.start(countdown);
        self.stage = next_stage;
    }

    pub fn is_done(&self) -> bool {
        self.stage == Completed
    }
}
