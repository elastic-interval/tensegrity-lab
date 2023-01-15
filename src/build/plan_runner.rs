use crate::build::growth::Growth;
use crate::build::parser::parse;
use crate::build::plan_runner::Stage::{*};
use crate::fabric::Fabric;
use crate::physics::presets::LIQUID;

const CODE: &str = "
(fabric
      (name \"Halo by Crane\")
      (build
            (seed :left)
            (grow A+ 5 (scale 92%)
                (branch
                        (grow B- 12 (scale 92%)
                             (branch (mark A+ :halo-end))
                        )
                        (grow D- 11 (scale 92%)
                            (branch (mark A+ :halo-end))
                        )
                 )
            )
      )
      (shape
        (pull-together :halo-end)
      )
)
";

#[derive(Clone, Debug, Copy, PartialEq)]
enum Stage {
    Empty,
    GrowStep,
    GrowApproach,
    GrowCalm,
    ShapingStart,
    ShapingApproach,
    Shaped,
    ShapedApproach,
    ShapingDone,
    ShapingCalm,
    Completed,
}

pub struct PlanRunner {
    pub growth: Growth,
    stage: Stage,
}

impl Default for PlanRunner {
    fn default() -> Self {
        Self {
            growth: Growth::new(parse(CODE).unwrap()),
            stage: Empty,
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
            Empty => {
                self.growth.init(fabric);
                GrowStep
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
            ShapingCalm => Completed,
            Completed => Completed,
        };
        let countdown = match next_stage {
            GrowApproach => 1000,
            GrowCalm => 1000,
            ShapingApproach => 20000,
            ShapedApproach => 5000,
            ShapingCalm => 50000,
            Empty | GrowStep | ShapingStart | Shaped | ShapingDone | Completed => 0,
        };
        fabric.progress.start(countdown);
        self.stage = next_stage;
    }

    pub fn is_done(&self) -> bool {
        self.stage == Completed
    }
}
