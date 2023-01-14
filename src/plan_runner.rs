use crate::fabric::Fabric;
use crate::growth::Growth;
use crate::parser::parse;
use crate::physics::Environment::Liquid;
use crate::physics::Physics;
use crate::plan_runner::Stage::{*};

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
    pub physics: Physics,
    pub iterations_per_frame: usize,
    pub growth: Growth,
    stage: Stage,
}

impl Default for PlanRunner {
    fn default() -> Self {
        Self {
            physics: Physics::new(Liquid),
            iterations_per_frame: 100,
            growth: Growth::new(parse(CODE).unwrap()),
            stage: Empty,
        }
    }
}

impl PlanRunner {
    pub fn iterate(&mut self, fabric: &mut Fabric) {
        for _ in 0..self.iterations_per_frame {
            fabric.iterate(&self.physics);
        }
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
