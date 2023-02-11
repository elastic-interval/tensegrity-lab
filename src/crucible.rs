use cgmath::Vector3;

use crate::build::brick::Baked;
use crate::build::tenscript::{FabricPlan, Library, SurfaceCharacterSpec};
use crate::build::tenscript::plan_runner::PlanRunner;
use crate::crucible::Stage::{*};
use crate::fabric::Fabric;
use crate::fabric::physics::{Physics, SurfaceCharacter};
use crate::fabric::physics::presets::{AIR_GRAVITY, PROTOTYPE_FORMATION};

const PULL_SHORTENING: f32 = 0.95;

enum Stage {
    Empty,
    AcceptingPlan(FabricPlan),
    RunningPlan,
    Pretensing,
    Pretenst,
    ShortenPulls(f32),
    AcceptingPrototype(Fabric),
    RunningPrototype,
}

pub struct Crucible {
    fabric: Fabric,
    physics: Physics,
    plan_runner: Option<PlanRunner>,
    camera_jump: Option<Vector3<f32>>,
    frozen_fabric: Option<Fabric>,
    iterations_per_frame: usize,
    paused: bool,
    stage: Stage,
    shorten_pulls: Option<f32>,
}

impl Default for Crucible {
    fn default() -> Self {
        Self {
            fabric: Fabric::default_bow_tie(),
            physics: AIR_GRAVITY,
            plan_runner: None,
            camera_jump: None,
            frozen_fabric: None,
            iterations_per_frame: 5,
            paused: false,
            stage: Empty,
            shorten_pulls: None,
        }
    }
}

impl Crucible {
    pub fn iterate(&mut self) {
        if self.paused {
            return;
        }
        match &self.stage {
            Empty => {}
            AcceptingPlan(fabric_plan) => {
                self.fabric = Fabric::default_bow_tie();
                match fabric_plan.surface {
                    None => {}
                    Some(surface_character) => {
                        self.physics.surface_character = match surface_character {
                            SurfaceCharacterSpec::Bouncy => SurfaceCharacter::Bouncy,
                            SurfaceCharacterSpec::Sticky => SurfaceCharacter::Sticky,
                            _ => SurfaceCharacter::Frozen,
                        }
                    }
                }
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
                match self.shorten_pulls {
                    None => {}
                    Some(strain_threshold) => {
                        self.stage = ShortenPulls(strain_threshold)
                    }
                }
            }
            ShortenPulls(strain_threshold) => {
                self.shorten_pulls = None;
                self.fabric.shorten_pulls(*strain_threshold, PULL_SHORTENING);
                self.start_pretensing()
            }
            AcceptingPrototype(fabric) => {
                self.fabric = fabric.clone();
                self.stage = RunningPrototype;
            }
            RunningPrototype => {
                let mut speed_squared = 1.0;
                for _ in 0..self.iterations_per_frame {
                    speed_squared = self.fabric.iterate(&PROTOTYPE_FORMATION);
                }
                let age = self.fabric.age;
                if age > 1000 && speed_squared < 1e-12 {
                    println!("Fabric settled in iteration {age} at speed squared {speed_squared}");
                    match Baked::try_from(self.fabric.clone()) {
                        Ok(brick) => {
                            println!("{}", brick.into_tenscript());
                        }
                        Err(problem) => {
                            println!("Cannot create brick: {problem}");
                        }
                    }

                    self.stage = Empty
                }
            }
        }
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn camera_jump(&mut self) -> Option<Vector3<f32>> {
        self.camera_jump.take()
    }

    pub fn strain_limits(&self) -> (f32, f32) {
        self.fabric.strain_limits(Fabric::BOW_TIE_MATERIAL_INDEX)
    }

    pub fn shorten_pulls(&mut self, strain_threshold: f32) {
        self.shorten_pulls = Some(strain_threshold);
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

    pub fn capture_prototype(&mut self, brick_index: usize) {
        println!("Settling and capturing prototype number {brick_index}");
        let fabric = Library::standard().bricks
            .get(brick_index)
            .expect("no such brick")
            .proto
            .clone()
            .into();
        self.stage = AcceptingPrototype(fabric);
    }

    fn start_pretensing(&mut self) {
        self.frozen_fabric = Some(self.fabric.clone());
        self.fabric.progress.start(20000);
        self.stage = Pretensing;
    }
}
