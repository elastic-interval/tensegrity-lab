use std::cmp::Ordering;
use crate::camera::Camera;
use crate::fabric::Fabric;
use crate::growth::Growth;
use crate::interval::Interval;
use crate::interval::Role::Measure;
use crate::interval::Span::{Approaching, Fixed};
use crate::parser::parse;
use crate::plan_runner::Stage::{*};
use crate::world::World;

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
    Evaporating,
}

pub struct PlanRunner {
    pub world: World,
    pub fabric: Fabric,
    pub frozen_fabric: Fabric,
    pub iterations_per_frame: usize,
    pub growth: Growth,
    stage: Stage,
}

impl Default for PlanRunner {
    fn default() -> Self {
        Self {
            world: World::default(),
            fabric: Fabric::default(),
            frozen_fabric: Fabric::default(),
            iterations_per_frame: 100,
            growth: Growth::new(parse(CODE).unwrap()),
            stage: Empty,
        }
    }
}

impl PlanRunner {
    pub fn iterate(&mut self, camera: &mut Camera) {
        let safe = !matches!(self.stage, Pretensing { .. } | Pretenst| Evaporating);
        for _ in 0..self.iterations_per_frame {
            self.fabric.iterate(&self.world, safe);
        }
        if self.fabric.progress.busy() {
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
            GrowApproach => {
                self.finish_approach();
                self.set_stage(GrowCalm);
            }
            GrowCalm => {
                self.set_stage(GrowStep);
            }
            ShapingStart => {
                self.set_stage(ShapingApproach);
            }
            ShapingApproach => {
                self.finish_approach();
                self.set_stage(Shaped);
            }
            Shaped => {
                self.growth.complete_shapers(&mut self.fabric);
                self.set_stage(ShapedApproach);
            }
            ShapedApproach => {
                self.finish_approach();
                self.set_stage(ShapingDone);
            }
            ShapingDone => {
                self.finish_approach();
                self.set_stage(ShapingCalm);
            }
            ShapingCalm => {
                self.frozen_fabric = self.fabric.clone();
                println!("Fabric frozen");
                self.start_pretensing(camera);
            }
            Pretensing => {
                self.finish_approach();
                self.set_stage(Evaporating);
            }
            Pretenst => {
                self.set_stage(Evaporating);
            }
            Evaporating => {
                self.evaporate();
                self.set_stage(Pretenst);
            }
        }
    }

    fn evaporate(&mut self) {
        let min_pull = self.fabric.intervals
            .iter()
            .filter(|(_, Interval { role, .. })| *role == Measure)
            .min_by(|(_, a), (_, b)| if a.strain < b.strain {
                Ordering::Less
            } else if a.strain > b.strain {
                Ordering::Greater
            } else {
                Ordering::Equal
            });
        if let Some((pushiest, _)) = min_pull {
            dbg!(&pushiest);
            self.fabric.remove_interval(*pushiest)
        }
    }

    fn finish_approach(&mut self) {
        for interval in self.fabric.intervals.values_mut() {
            if let Approaching { length, .. } = interval.span {
                interval.span = Fixed { length }
            }
        }
    }

    fn start_pretensing(&mut self, camera: &mut Camera) {
        self.fabric.install_measures();
        let up = self.fabric.prepare_for_pretensing(1.03);
        camera.go_up(up);
        self.set_stage(Pretensing)
    }

    fn finish_pretensing(&mut self) {
        self.set_stage(Evaporating)
    }

    fn set_stage(&mut self, stage: Stage) {
        self.stage = stage;
        let countdown = match stage {
            Empty => 0,
            GrowStep => 0,
            GrowApproach => 1000,
            GrowCalm => 1000,
            ShapingStart => 0,
            ShapingApproach => 20000,
            Shaped => 0,
            ShapedApproach => 5000,
            ShapingDone => 0,
            ShapingCalm => 50000,
            Pretensing => 20000,
            Pretenst => 0,
            Evaporating => 5000,
        };
        if countdown > 0 {
            self.fabric.progress.start(countdown);
        }
    }
}
