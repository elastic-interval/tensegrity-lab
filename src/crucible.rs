use winit::event::VirtualKeyCode;

use crate::build::brick::{Baked};
use crate::build::tenscript::{FabricPlan, FaceAlias, Library, SurfaceCharacterSpec};
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
    AddingBrick { face_alias: FaceAlias, face_id: UniqueId },
    Pretensing,
    AcceptingPrototype(Fabric),
    RunningPrototype,
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
            iterations_per_frame: 25,
            stage: Empty,
        }
    }
}

impl Crucible {
    pub fn iterate(&mut self) {
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
            AddingBrick { face_alias, face_id } => {
                let faces = self.fabric.attach_brick(face_alias, 1.0, Some(*face_id));
                self.stage = Interactive;
                self.fabric.progress.start(1000);
                self.action = faces.first().map(|&face_id| Action::SelectFace(face_id));
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
            Finished => {}
        }
    }

    fn iterate_frame(&mut self, special_physics: Option<&Physics>) {
        let physics = special_physics.unwrap_or(&self.physics);
        for _ in 0..self.iterations_per_frame {
            self.fabric.iterate(physics);
        }
    }

    pub fn strain_limits(&self) -> (f32, f32) {
        self.fabric.strain_limits(Fabric::BOW_TIE_MATERIAL_INDEX)
    }

    pub fn set_speed(&mut self, key: &VirtualKeyCode) {
        self.iterations_per_frame = match key {
            VirtualKeyCode::Key0 => 0,
            VirtualKeyCode::Key1 => 1,
            VirtualKeyCode::Key2 => 5,
            VirtualKeyCode::Key3 => 25,
            VirtualKeyCode::Key4 => 125,
            VirtualKeyCode::Key5 => 625,
            _ => unreachable!()
        };
    }

    pub fn add_brick(&mut self, face_alias: FaceAlias, face_id: UniqueId) {
        self.stage = AddingBrick { face_alias, face_id };
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

    pub fn capture_prototype(&mut self, brick_index: usize) {
        println!("Settling and capturing prototype number {brick_index}");
        let fabric = Library::standard()
            .bricks
            .get(brick_index)
            .expect("no such brick")
            .proto
            .clone()
            .into();
        self.stage = AcceptingPrototype(fabric);
    }
}
