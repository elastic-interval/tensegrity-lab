use cgmath::Vector3;

use crate::build::brick::{Baked, BrickName};
use crate::build::tenscript::{FabricPlan, FaceName, SurfaceCharacterSpec};
use crate::build::tenscript::plan_runner::PlanRunner;
use crate::controls::Action;
use crate::crucible::Stage::{*};
use crate::fabric::{Fabric, UniqueId};
use crate::fabric::physics::{Physics, SurfaceCharacter};
use crate::fabric::physics::presets::{AIR_GRAVITY, LIQUID, PROTOTYPE_FORMATION};

const PULL_SHORTENING: f32 = 0.95;
const PRETENST_FACTOR: f32 = 1.03;

enum Stage {
    Empty,
    AcceptingPlan(FabricPlan),
    RunningPlan,
    Interactive,
    AddingBrick { brick_name: BrickName, face_id: UniqueId },
    Pretensing,
    Pretenst,
    ShortenPulls { strain_threshold: f32 },
    AcceptingPrototype((Fabric, UniqueId)),
    RunningPrototype(UniqueId),
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
}

impl Default for Crucible {
    fn default() -> Self {
        Self {
            fabric: Fabric::default_bow_tie(),
            physics: AIR_GRAVITY,
            plan_runner: None,
            camera_jump: None,
            frozen_fabric: None,
            iterations_per_frame: 100,
            paused: false,
            stage: Empty,
        }
    }
}

impl Crucible {
    pub fn iterate(&mut self) -> Option<Action> {
        if self.paused {
            return None;
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
                            self.stage = if self.fabric.faces.is_empty() {
                                self.start_pretensing()
                            } else {
                                Interactive
                            }
                        }
                    }
                }
            }
            Interactive => {
                self.iterate_frame(Some(&LIQUID));
            }
            AddingBrick { brick_name, face_id } => {
                let faces = self.fabric.attach_brick(*brick_name, 1.0, Some(*face_id));
                self.stage = Interactive;
                self.fabric.progress.start(1000);
                let (_, new_face_id) = faces.into_iter().find(|&(face_name, _)|face_name == FaceName(1))?;
                return Some(Action::SelectFace(new_face_id));
            }
            Pretensing => {
                self.iterate_frame(None);
                if !self.fabric.progress.is_busy() {
                    self.stage = Pretenst;
                }
            }
            Pretenst => {
                self.iterate_frame(None);
            }
            ShortenPulls { strain_threshold } => {
                self.fabric.shorten_pulls(*strain_threshold, PULL_SHORTENING);
                self.stage = self.start_pretensing();
            }
            AcceptingPrototype((fabric, face_id)) => {
                self.fabric = fabric.clone();
                self.stage = RunningPrototype(*face_id);
            }
            RunningPrototype(face_id) => {
                let mut speed_squared = 1.0;
                for _ in 0..self.iterations_per_frame {
                    speed_squared = self.fabric.iterate(&PROTOTYPE_FORMATION);
                }
                let age = self.fabric.age;
                if age > 1000 && speed_squared < 1e-12 {
                    println!("Fabric settled in iteration {age} at speed squared {speed_squared}");
                    match Baked::try_from((self.fabric.clone(), *face_id)) {
                        Ok(brick) => {
                            println!("{}", brick.into_code());
                        }
                        Err(problem) => {
                            println!("Cannot create brick: {problem}");
                        }
                    }

                    self.stage = Empty
                }
            }
        }
        None
    }

    fn iterate_frame(&mut self, special_physics: Option<&Physics>) {
        let physics = special_physics.unwrap_or(&self.physics);
        for _ in 0..self.iterations_per_frame {
            self.fabric.iterate(physics);
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
        self.stage = ShortenPulls { strain_threshold };
    }

    pub fn add_brick(&mut self, brick_name: BrickName, face_id: UniqueId) {
        self.stage = AddingBrick { brick_name, face_id };
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

    pub fn capture_prototype(&mut self, brick_name: BrickName) {
        println!("Settling and capturing prototype {brick_name:?}");
        self.stage = AcceptingPrototype(Baked::prototype(brick_name));
    }

    fn start_pretensing(&mut self) -> Stage {
        self.frozen_fabric = Some(self.fabric.clone());
        let old_midpoint = self.fabric.midpoint();
        self.fabric.prepare_for_pretensing(PRETENST_FACTOR);
        self.camera_jump = Some(self.fabric.midpoint() - old_midpoint);
        self.fabric.progress.start(20000);
        Pretensing
    }
}
