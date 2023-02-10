use crate::build::brick::{Baked, BrickName};
use crate::build::tenscript::{FabricPlan, FaceName, SurfaceCharacterSpec};
use crate::build::tenscript::plan_runner::PlanRunner;
use crate::controls::Action;
use crate::crucible::Stage::{*};
use crate::fabric::{Fabric, UniqueId};
use crate::fabric::physics::{Physics, SurfaceCharacter};
use crate::fabric::physics::presets::{AIR_GRAVITY, LIQUID, PROTOTYPE_FORMATION};
use crate::fabric::pretenser::Pretenser;

const PULL_SHORTENING: f32 = 0.95;
const PRETENST_FACTOR: f32 = 1.03;

enum Stage {
    Empty,
    AcceptingPlan(FabricPlan),
    RunningPlan,
    Interactive,
    AddingBrick { brick_name: BrickName, face_id: UniqueId },
    Pretensing,
    AcceptingPrototype((Fabric, UniqueId)),
    RunningPrototype(UniqueId),
    Finished,
}

pub struct Crucible {
    fabric: Fabric,
    physics: Physics,
    plan_runner: Option<PlanRunner>,
    pretenser: Option<Pretenser>,
    frozen_fabric: Option<Fabric>,
    action: Option<Action>,
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
            pretenser: None,
            frozen_fabric: None,
            action: None,
            iterations_per_frame: 100,
            paused: false,
            stage: Empty,
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
                            self.stage = if self.fabric.faces.is_empty() {
                                self.pretenser = Some(Pretenser::new(PRETENST_FACTOR));
                                self.action = Some(Action::ShowSurface);
                                Pretensing
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
                let (_, new_face_id) = faces.into_iter().find(|&(face_name, _)| face_name == FaceName(1)).unwrap();
                self.action = Some(Action::SelectFace(new_face_id));
            }
            Pretensing => {
                match &mut self.pretenser {
                    None => {
                        self.stage = Empty;
                    }
                    Some(pretenser) => {
                        for _ in 0..self.iterations_per_frame {
                            pretenser.iterate(&mut self.fabric);
                        }
                        if pretenser.is_done() {
                            self.stage = Finished;
                        }
                    }
                }
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
            Finished => {}
        }
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

    pub fn strain_limits(&self) -> (f32, f32) {
        self.fabric.strain_limits(Fabric::BOW_TIE_MATERIAL_INDEX)
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

    pub fn action(&mut self) -> Option<Action> {
        self.action.take()
    }

    pub fn fabric(&self) -> &Fabric {
        &self.fabric
    }

    pub fn capture_prototype(&mut self, brick_name: BrickName) {
        println!("Settling and capturing prototype {brick_name:?}");
        self.stage = AcceptingPrototype(Baked::prototype(brick_name));
    }
}
