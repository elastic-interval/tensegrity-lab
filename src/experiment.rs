use crate::camera::Camera;
use crate::experiment::Stage::{Pretensing, Pretenst, RunningPlan};
use crate::fabric::Fabric;
use crate::physics::Environment::AirGravity;
use crate::physics::Physics;
use crate::plan_runner::PlanRunner;

enum Stage {
    RunningPlan,
    Pretensing,
    Pretenst,
}

pub struct Experiment {
    pub fabric: Fabric,
    plan_runner: PlanRunner,
    physics: Physics,
    iterations_per_frame: usize,
    stage: Stage,
}

impl Default for Experiment {
    fn default() -> Self {
        Self {
            plan_runner: PlanRunner::default(),
            fabric: Fabric::default(),
            physics: Physics::new(AirGravity),
            iterations_per_frame: 100,
            stage: RunningPlan,
        }
    }
}

impl Experiment {
    pub fn iterate(&mut self, camera: &mut Camera) {
        match &mut self.stage {
            RunningPlan => {
                self.plan_runner.iterate(&mut self.fabric);
                if self.plan_runner.is_done() {
                    self.fabric.install_measures();
                    let up = self.fabric.prepare_for_pretensing(1.03);
                    camera.go_up(up);
                    self.fabric.progress.start(20000);
                    self.stage = Pretensing;
                }
            }
            Pretensing => {
                for _ in 0..self.iterations_per_frame {
                    self.fabric.iterate(&self.physics);
                }
                if !self.fabric.progress.is_busy() {
                    self.stage = Pretenst;
                }
            }
            Pretenst => {}
        }
    }
}
