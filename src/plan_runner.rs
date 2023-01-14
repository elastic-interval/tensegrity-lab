use crate::camera::Camera;
use crate::fabric::Fabric;
use crate::growth::Growth;
use crate::interval::Interval;
use crate::interval::Role::Measure;
use crate::parser::parse;
use crate::physics::Environment::{AirGravity, Liquid};
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
    Pretensing,
    Pretenst,
}

pub struct PlanRunner {
    pub physics: Physics,
    pub fabric: Fabric,
    pub frozen: Option<Fabric>,
    pub iterations_per_frame: usize,
    pub growth: Growth,
    stage: Stage,
}

impl Default for PlanRunner {
    fn default() -> Self {
        Self {
            physics: Physics::new(Liquid),
            fabric: Fabric::default(),
            frozen: None,
            iterations_per_frame: 100,
            growth: Growth::new(parse(CODE).unwrap()),
            stage: Empty,
        }
    }
}

impl PlanRunner {
    pub fn iterate(&mut self, camera: &mut Camera) {
        for _ in 0..self.iterations_per_frame {
            self.fabric.iterate(&self.physics);
        }
        if self.fabric.progress.is_busy() {
            return;
        }
        match self.stage {
            Empty => {
                self.growth.init(&mut self.fabric);
                self.set_stage(GrowStep);
            }
            GrowStep => {
                if self.growth.is_growing() {
                    self.growth.growth_step(&mut self.fabric);
                    self.set_stage(GrowApproach);
                } else if self.growth.needs_shaping() {
                    self.growth.create_shapers(&mut self.fabric);
                    self.set_stage(ShapingStart);
                }
            }
            GrowApproach => self.set_stage(GrowCalm),
            GrowCalm => self.set_stage(GrowStep),
            ShapingStart => self.set_stage(ShapingApproach),
            ShapingApproach => self.set_stage(Shaped),
            Shaped => {
                self.growth.complete_shapers(&mut self.fabric);
                self.set_stage(ShapedApproach);
            }
            ShapedApproach => self.set_stage(ShapingDone),
            ShapingDone => self.set_stage(ShapingCalm),
            ShapingCalm => {
                self.frozen = Some(self.fabric.clone());
                self.physics = Physics::new(AirGravity);
                self.fabric.install_measures();
                let up = self.fabric.prepare_for_pretensing(1.03);
                camera.go_up(up);
                self.set_stage(Pretensing);
            }
            Pretensing => self.set_stage(Pretenst),
            Pretenst => {}
        }
    }

    fn set_stage(&mut self, stage: Stage) {
        self.stage = stage;
        let countdown = match stage {
            GrowApproach => 1000,
            GrowCalm => 1000,
            ShapingApproach => 20000,
            ShapedApproach => 5000,
            ShapingCalm => 50000,
            Pretensing => 20000,
            Empty | GrowStep | ShapingStart | Shaped | ShapingDone | Pretenst => 0,
        };
        self.fabric.progress.start(countdown);
    }
}
