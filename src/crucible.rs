use crate::build::brick::{Baked, Prototype};
use crate::build::tenscript::{FabricPlan, FaceAlias, Library};
use crate::build::tenscript::plan_runner::PlanRunner;
use crate::crucible::Stage::{*};
use crate::fabric::{Fabric, UniqueId};
use crate::fabric::face::FaceRotation;
use crate::fabric::physics::presets::{LIQUID, PROTOTYPE_FORMATION};
use crate::fabric::pretenser::Pretenser;
use crate::user_interface::Action;

const PULL_SHORTENING: f32 = 0.95;
const PRETENST_FACTOR: f32 = 1.03;

enum Stage {
    Empty,
    AcceptingPlan(FabricPlan),
    RunningPlan(PlanRunner),
    Interactive,
    AddingBrick { alias: FaceAlias, face_id: UniqueId },
    Pretensing(Pretenser),
    AcceptingPrototype(Prototype),
    RunningPrototype(FaceAlias),
    Finished,
}

pub struct Crucible {
    fabric: Fabric,
    frozen_fabric: Option<Fabric>,
    action: Option<Action>,
    iterations_per_frame: usize,
    stage: Stage,
}

impl Default for Crucible {
    fn default() -> Self {
        Self {
            fabric: Fabric::default_bow_tie(),
            frozen_fabric: None,
            action: None,
            iterations_per_frame: 125,
            stage: Empty,
        }
    }
}

impl Crucible {
    pub fn iterate(&mut self) {
        match &mut self.stage {
            Empty => {}
            AcceptingPlan(fabric_plan) => {
                self.fabric = Fabric::default_bow_tie();
                self.frozen_fabric = None;
                self.stage = RunningPlan(PlanRunner::new(fabric_plan.clone()));
            }
            RunningPlan(plan_runner) => {
                for _ in 0..self.iterations_per_frame {
                    plan_runner.iterate(&mut self.fabric);
                }
                if plan_runner.is_done() {
                    self.stage =
                        if self.fabric.faces.is_empty() {
                            self.action = Some(Action::ShowSurface);
                            Pretensing(Pretenser::new(PRETENST_FACTOR))
                        } else {
                            Interactive
                        }
                }
            }
            Interactive => {
                for _ in 0..self.iterations_per_frame {
                    self.fabric.iterate(&LIQUID);
                }
            }
            AddingBrick { alias, face_id } => {
                let faces = self.fabric.attach_brick(alias, FaceRotation::Zero, 1.0, Some(*face_id));
                self.stage = Interactive;
                self.fabric.progress.start(1000);
                self.action = faces.first().map(|&face_id| Action::SelectFace(face_id));
            }
            Pretensing(pretenser) => {
                for _ in 0..self.iterations_per_frame {
                    pretenser.iterate(&mut self.fabric);
                }
                if pretenser.is_done() {
                    self.stage = Finished;
                }
            }
            AcceptingPrototype(prototype) => {
                let alias = prototype.alias.clone();
                self.fabric = Fabric::from(prototype.clone());
                self.stage = RunningPrototype(alias);
            }
            RunningPrototype(alias) => {
                let mut speed_squared = 1.0;
                for _ in 0..self.iterations_per_frame {
                    speed_squared = self.fabric.iterate(&PROTOTYPE_FORMATION);
                }
                let age = self.fabric.age;
                if age > 1000 && speed_squared < 1e-12 {
                    println!("Fabric settled in iteration {age} at speed squared {speed_squared}");
                    match Baked::try_from((self.fabric.clone(), alias.clone())) {
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

    pub fn strain_limits(&self) -> (f32, f32) {
        self.fabric.strain_limits(Fabric::BOW_TIE_MATERIAL_INDEX)
    }

    pub fn set_speed(&mut self, iterations_per_frame: usize) {
        self.iterations_per_frame = iterations_per_frame;
    }

    pub fn add_brick(&mut self, alias: FaceAlias, face_id: UniqueId) {
        self.stage = AddingBrick { alias, face_id };
    }

    pub fn build_fabric(&mut self, fabric_plan: FabricPlan) {
        self.stage = AcceptingPlan(fabric_plan);
    }

    pub fn action(&mut self) -> Option<Action> {
        self.action.take()
    }

    pub fn fabric(&self) -> &Fabric {
        &self.fabric
    }

    pub fn capture_prototype(&mut self, brick_index: usize) {
        println!("Settling and capturing prototype number {brick_index}");
        let proto = Library::standard()
            .bricks
            .get(brick_index)
            .expect("no such brick")
            .proto
            .clone();
        self.stage = AcceptingPrototype(proto);
    }
}
