use crate::experiment::Stage::{Pretensing, Pretenst, RunningPlan};
use crate::fabric::Fabric;
use crate::physics::presets::AIR_GRAVITY;
use crate::build::plan_runner::PlanRunner;

enum Stage {
    RunningPlan,
    Pretensing,
    Pretenst,
}

pub struct Experiment {
    fabric: Fabric,
    plan_runner: PlanRunner,
    iterations_per_frame: usize,
    stage: Stage,
}

impl Default for Experiment {
    fn default() -> Self {
        Self {
            plan_runner: PlanRunner::default(),
            fabric: Fabric::default(),
            iterations_per_frame: 100,
            stage: RunningPlan,
        }
    }
}

impl Experiment {
    pub fn iterate(&mut self) -> Option<f32> {
        match &mut self.stage {
            RunningPlan => {
                for _ in 0..self.iterations_per_frame {
                    self.plan_runner.iterate(&mut self.fabric);
                }
                if self.plan_runner.is_done() {
                    self.fabric.install_measures();
                    self.fabric.progress.start(20000);
                    self.stage = Pretensing;
                    let up = self.fabric.prepare_for_pretensing(1.03);
                    return Some(up);
                }
            }
            Pretensing => {
                for _ in 0..self.iterations_per_frame {
                    self.fabric.iterate(&AIR_GRAVITY);
                }
                if !self.fabric.progress.is_busy() {
                    self.stage = Pretenst;
                }
            }
            Pretenst => {}
        }
        None
    }

    pub fn fabric(&self) -> &Fabric {
        &self.fabric
    }
}
