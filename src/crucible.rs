use crate::build::oven::Oven;
use crate::build::tenscript::{FabricPlan, FaceAlias};
use crate::build::tenscript::plan_runner::PlanRunner;
use crate::build::tinkerer::Tinkerer;
use crate::crucible::Stage::{*};
use crate::fabric::{Fabric, UniqueId};
use crate::fabric::pretenser::Pretenser;
use crate::scene::{SceneAction, SceneVariant};
use crate::user_interface::Action;

const PULL_SHORTENING: f32 = 0.95;
const PRETENST_FACTOR: f32 = 1.03;

enum Stage {
    Empty,
    RunningPlan(PlanRunner),
    Tinkering(Tinkerer),
    Pretensing(Pretenser),
    BakingBrick(Oven),
    Finished,
}

#[derive(Debug, Clone)]
pub enum CrucibleAction {
    BakeBrick(usize),
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
            BakingBrick(oven) => {
                if let Some(baked) = oven.iterate(&mut self.fabric) {
                    println!("{}", baked.into_tenscript());
                    self.stage = Finished;
                }
            }
            Finished => {}
        }
        actions
    }

    pub fn action(&mut self, crucible_action: CrucibleAction) {
        match crucible_action {
            CrucibleAction::BakeBrick(brick_index) => {
                let oven = Oven::new(brick_index);
                self.fabric = oven.prototype_fabric();
                self.stage = BakingBrick(oven);
            }
            CrucibleAction::BuildFabric(fabric_plan) => {
                self.fabric = Fabric::default_bow_tie();
                self.frozen_fabric = None;
                self.stage = RunningPlan(PlanRunner::new(fabric_plan.clone()));
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

    pub fn fabric(&self) -> &Fabric {
        &self.fabric
    }
}
