use crate::build::brick::{Baked, Prototype};
use crate::build::tenscript::{FabricPlan, FaceAlias, Library};
use crate::build::tenscript::plan_runner::PlanRunner;
use crate::build::tinkerer::Tinkerer;
use crate::crucible::Stage::{*};
use crate::fabric::{Fabric, UniqueId};
use crate::fabric::physics::presets::PROTOTYPE_FORMATION;
use crate::fabric::pretenser::Pretenser;
use crate::scene::{SceneAction, SceneVariant};
use crate::user_interface::Action;

const PULL_SHORTENING: f32 = 0.95;
const PRETENST_FACTOR: f32 = 1.03;

enum Stage {
    Empty,
    AcceptingPlan(FabricPlan),
    RunningPlan(PlanRunner),
    Tinkering(Tinkerer),
    Pretensing(Pretenser),
    AcceptingPrototype(Prototype),
    RunningPrototype(FaceAlias),
    Finished,
}

#[derive(Debug, Clone)]
pub enum CrucibleAction {
    BuildFabric(FabricPlan),
    CreateBrickOnFace(UniqueId),
    SetSpeed(usize),
}

pub struct Crucible {
    fabric: Fabric,
    frozen_fabric: Option<Fabric>,
    iterations_per_frame: usize,
    stage: Stage,
}

impl Default for Crucible {
    fn default() -> Self {
        Self {
            fabric: Fabric::default_bow_tie(),
            frozen_fabric: None,
            iterations_per_frame: 125,
            stage: Empty,
        }
    }
}

impl Crucible {
    pub fn iterate(&mut self) -> Vec<Action> {
        let mut actions = Vec::new();
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
                            actions.push(Action::Scene(SceneAction::Variant(SceneVariant::Pretensing)));
                            Pretensing(Pretenser::new(PRETENST_FACTOR))
                        } else {
                            actions.push(Action::Scene(SceneAction::Variant(SceneVariant::Tinkering)));
                            Tinkering(Tinkerer::new())
                        }
                }
            }
            Tinkering(tinkerer) => {
                for _ in 0..self.iterations_per_frame {
                    if let Some(tinker_action) = tinkerer.iterate(&mut self.fabric) {
                        actions.push(tinker_action);
                    }
                }
                if tinkerer.is_done() {
                    self.stage = Finished;
                }
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
        actions
    }

    pub fn action(&mut self, crucible_action: CrucibleAction) {
        match crucible_action {
            CrucibleAction::BuildFabric(fabric_plan) => {
                self.stage = AcceptingPlan(fabric_plan);
            }
            CrucibleAction::CreateBrickOnFace(face_id) => {
                let Tinkering(tinkerer) = &mut self.stage else {
                    panic!("cannot add brick unless tinkering");
                };
                let spin = self.fabric.face(face_id).spin.opposite();
                let face_alias = FaceAlias::single("Single") + &spin.into_alias();
                tinkerer.add_brick(face_alias, face_id);
            }
            CrucibleAction::SetSpeed(iterations_per_frame) => {
                self.iterations_per_frame = iterations_per_frame;
            }
        }
    }

    pub fn strain_limits(&self) -> (f32, f32) {
        self.fabric.strain_limits(Fabric::BOW_TIE_MATERIAL_INDEX)
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
