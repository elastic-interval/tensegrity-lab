use std::cmp::Ordering;
use crate::camera::Camera;
use crate::fabric::Fabric;
use crate::fabric::Stage::{*};
use crate::growth::Growth;
use crate::interval::Interval;
use crate::interval::Role::Measure;
use crate::interval::Span::{Approaching, Fixed};
use crate::parser::parse;
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

pub struct PlanRunner {
    pub world: World,
    pub fabric: Fabric,
    pub iterations_per_frame: usize,
    pub growth: Growth,
}

impl Default for PlanRunner {
    fn default() -> Self {
        Self {
            world: World::default(),
            fabric: Fabric::default(),
            iterations_per_frame: 100,
            growth: Growth::new(parse(CODE).unwrap()),
        }
    }
}

impl PlanRunner {
    pub fn iterate(&mut self, camera: &mut Camera) {
        for _ in 0..self.iterations_per_frame {
            self.fabric.iterate(&self.world);
        }
        if self.fabric.progress.busy() {
            return;
        }
        match self.fabric.stage() {
            Empty => {
                self.growth.init(&mut self.fabric);
                self.fabric.set_stage(GrowStep);
            }
            GrowStep => {
                if self.growth.is_growing() {
                    self.growth.growth_step(&mut self.fabric);
                    self.fabric.set_stage(GrowApproach);
                } else if self.growth.needs_shaping() {
                    self.growth.create_shapers(&mut self.fabric);
                    self.fabric.set_stage(ShapingStart);
                }
            }
            GrowApproach => {
                self.finish_approach();
                self.fabric.set_stage(GrowCalm);
            }
            GrowCalm => {
                self.fabric.set_stage(GrowStep);
            }
            ShapingStart => {
                self.fabric.set_stage(ShapingApproach);
            }
            ShapingApproach => {
                self.finish_approach();
                self.fabric.set_stage(Shaped);
            }
            Shaped => {
                self.growth.complete_shapers(&mut self.fabric);
                self.fabric.set_stage(ShapedApproach);
            }
            ShapedApproach => {
                self.finish_approach();
                self.fabric.set_stage(ShapingDone);
            }
            ShapingDone => {
                self.finish_approach();
                self.fabric.set_stage(ShapingCalm);
            }
            ShapingCalm => {
                self.start_pretensing(camera);
            }
            Pretensing => {
                self.finish_approach();
                self.fabric.set_stage(Evaporating);
            }
            Pretenst => {
                self.fabric.set_stage(Evaporating);
            }
            Evaporating => {
                self.evaporate();
                self.fabric.set_stage(Pretenst);
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
        self.fabric.set_stage(Pretensing)
    }

    fn finish_pretensing(&mut self) {
        self.fabric.set_stage(Evaporating)
    }
}
