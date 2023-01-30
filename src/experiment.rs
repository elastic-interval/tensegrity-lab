use cgmath::Vector3;
use crate::experiment::Stage::{*};
use crate::fabric::{Fabric, Link};
use crate::fabric::physics::presets::AIR_GRAVITY;
use crate::build::plan_runner::PlanRunner;
use crate::build::tenscript::FabricPlan;
use crate::fabric::physics::Physics;

enum Stage {
    Empty,
    AcceptingPlan(FabricPlan),
    RunningPlan,
    Pretensing,
    Pretenst,
    AddPulls { strain_threshold: f32 },
}

pub struct Experiment {
    fabric: Fabric,
    physics: Physics,
    plan_runner: Option<PlanRunner>,
    camera_jump: Option<Vector3<f32>>,
    frozen_fabric: Option<Fabric>,
    iterations_per_frame: usize,
    paused: bool,
    stage: Stage,
    add_pulls: Option<f32>,
}

impl Experiment {
}

impl Default for Experiment {
    fn default() -> Self {
        Self {
            fabric: Fabric::default(),
            physics: AIR_GRAVITY.clone(),
            plan_runner: None,
            camera_jump: None,
            frozen_fabric: None,
            iterations_per_frame: 100,
            paused: false,
            stage: Empty,
            add_pulls: None,
        }
    }
}

impl Experiment {
    pub fn iterate(&mut self) {
        if self.paused {
            return;
        }
        match &self.stage {
            Empty => {}
            AcceptingPlan(fabric_plan) => {
                self.fabric = Fabric::default();
                self.frozen_fabric = None;
                self.plan_runner = Some(PlanRunner::new(fabric_plan.clone()));
                self.stage = RunningPlan;
            }
            RunningPlan => {
                match &mut self.plan_runner {
                    None => {
                        self.stage = Empty;
                    }
                    Some(plan_runner) => {
                        for _ in 0..self.iterations_per_frame {
                            plan_runner.iterate(&mut self.fabric);
                        }
                        if plan_runner.is_done() {
                            let old_midpoint = self.fabric.midpoint();
                            self.fabric.prepare_for_pretensing(1.03);
                            self.start_pretensing();
                            self.camera_jump = Some(self.fabric.midpoint() - old_midpoint);
                        }
                    }
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
            Pretenst => {
                for _ in 0..self.iterations_per_frame {
                    self.fabric.iterate(&self.physics);
                }
                match self.add_pulls {
                    None => {}
                    Some(strain_threshold) => {
                        self.stage = AddPulls { strain_threshold }
                    }
                }
            }
            AddPulls { strain_threshold } => {
                self.add_pulls = None;
                let new_pulls = self.fabric.measures_to_pulls(*strain_threshold);
                self.fabric = self.frozen_fabric.take().unwrap();
                for (alpha_index, omega_index, ideal) in new_pulls {
                    self.fabric.create_interval(alpha_index, omega_index, Link::Pull { ideal });
                }
                self.start_pretensing()
            }
        }
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn camera_jump(&mut self) -> Option<Vector3<f32>> {
        self.camera_jump.take()
    }

    pub fn add_pulls(&mut self, strain_threshold: f32) {
        self.add_pulls = Some(strain_threshold);
    }

    pub fn build_fabric(&mut self, fabric_plan: FabricPlan) {
        self.stage = AcceptingPlan(fabric_plan);
    }

    pub fn set_gravity(&mut self, gravity: f32) {
        self.physics.gravity = gravity;
    }

    pub fn fabric(&self) -> &Fabric {
        &self.fabric
    }

    fn start_pretensing(&mut self) {
        self.frozen_fabric = Some(self.fabric.clone());
        self.fabric.progress.start(20000);
        self.stage = Pretensing;
    }
}
