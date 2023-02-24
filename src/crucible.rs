use std::collections::HashSet;
use CrucibleAction::{*};

use crate::build::oven::Oven;
use crate::build::tenscript::FabricPlan;
use crate::build::tenscript::plan_runner::PlanRunner;
use crate::build::tinkerer::{BrickOnFace, Tinkerer};
use crate::crucible::Stage::{*};
use crate::fabric::{Fabric, UniqueId};
use crate::fabric::pretenser::Pretenser;
use crate::scene::{SceneAction, SceneVariant};
use crate::user_interface::{Action, MenuChoice};

const PULL_SHORTENING: f32 = 0.95;
const PRETENST_FACTOR: f32 = 1.03;

enum Stage {
    Empty,
    RunningPlan(PlanRunner),
    TinkeringLaunch,
    Tinkering(Tinkerer),
    PretensingLaunch,
    Pretensing(Pretenser),
    BakingBrick(Oven),
    Finished,
}

#[derive(Debug, Clone)]
pub enum CrucibleAction {
    BakeBrick(usize),
    BuildFabric(FabricPlan),
    ProposeBrick(BrickOnFace),
    ConnectBrick,
    JoinFaces(HashSet<UniqueId>),
    SetSpeed(usize),
    InitiateRevert,
    RevertTo(Fabric),
    StartPretensing,
    StartTinkering,
}

pub struct Crucible {
    fabric: Fabric,
    iterations_per_frame: usize,
    stage: Stage,
}

impl Default for Crucible {
    fn default() -> Self {
        Self {
            fabric: Fabric::default_bow_tie(),
            iterations_per_frame: 125,
            stage: Empty,
        }
    }
}

impl Crucible {
    pub fn iterate(&mut self, paused: bool) -> Vec<Action> {
        let mut actions = Vec::new();
        match &mut self.stage {
            Empty => {}
            RunningPlan(plan_runner) => {
                for _ in 0..self.iterations_per_frame {
                    plan_runner.iterate(&mut self.fabric);
                }
                if plan_runner.is_done() {
                    self.stage = if self.fabric.faces.is_empty() {
                        PretensingLaunch
                    } else {
                        TinkeringLaunch
                    }
                }
            }
            TinkeringLaunch => {
                actions.push(Action::Keyboard(MenuChoice::Tinker));
                actions.push(Action::SelectFace(None));
                self.stage = Tinkering(Tinkerer::default())
            }
            Tinkering(tinkerer) => {
                let iterations = if paused { 1 } else { self.iterations_per_frame };
                for _ in 0..iterations {
                    if let Some(tinker_action) = tinkerer.iterate(&mut self.fabric) {
                        actions.push(tinker_action);
                    }
                }
            }
            PretensingLaunch => {
                actions.push(Action::Scene(SceneAction::Variant(SceneVariant::Pretensing)));
                self.stage = Pretensing(Pretenser::new(PRETENST_FACTOR))
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
            BakeBrick(brick_index) => {
                let oven = Oven::new(brick_index);
                self.fabric = oven.prototype_fabric();
                self.stage = BakingBrick(oven);
            }
            BuildFabric(fabric_plan) => {
                self.fabric = Fabric::default_bow_tie();
                self.stage = RunningPlan(PlanRunner::new(fabric_plan));
            }
            ProposeBrick(brick_on_face) => {
                let Tinkering(tinkerer) = &mut self.stage else {
                    panic!("cannot add brick unless tinkering");
                };
                tinkerer.propose_brick(brick_on_face);
            }
            ConnectBrick => {
                let Tinkering(tinkerer) = &mut self.stage else {
                    panic!("cannot add brick unless tinkering");
                };
                tinkerer.connect();
            }
            JoinFaces(face_set) => {
                let Tinkering(tinkerer) = &mut self.stage else {
                    panic!("cannot add brick unless tinkering");
                };
                if let Ok([a, b]) = face_set.into_iter().next_chunk() {
                    tinkerer.join_faces(a, b);
                }
            }
            SetSpeed(iterations_per_frame) => {
                self.iterations_per_frame = iterations_per_frame;
            }
            InitiateRevert => {
                let Tinkering(tinkerer) = &mut self.stage else {
                    panic!("cannot revert unless tinkering");
                };
                tinkerer.revert();
            }
            RevertTo(frozen) => {
                self.fabric = frozen;
            }
            StartPretensing => {
                self.stage = PretensingLaunch;
            }
            StartTinkering => {
                self.stage = TinkeringLaunch;
            }
        }
    }

    pub fn fabric(&self) -> &Fabric {
        &self.fabric
    }
}
