use std::collections::HashSet;
use CrucibleAction::{*};

use crate::build::oven::Oven;
use crate::build::tenscript::FabricPlan;
use crate::build::tenscript::plan_runner::PlanRunner;
use crate::build::tinkerer::{BrickOnFace, Tinkerer};
use crate::crucible::Stage::{*};
use crate::fabric::{Fabric, UniqueId};
use crate::fabric::physics::SurfaceCharacter;
use crate::fabric::pretenser::Pretenser;
use crate::scene::{SceneAction, SceneVariant};
use crate::user_interface::{Action, MenuAction};

const PULL_SHORTENING: f32 = 0.95;
const PRETENST_FACTOR: f32 = 1.03;

enum Stage {
    Empty,
    RunningPlan(PlanRunner),
    TinkeringLaunch,
    Tinkering(Tinkerer),
    PretensingLaunch(SurfaceCharacter),
    Pretensing(Pretenser),
    BakingBrick(Oven),
    Finished,
}

#[derive(Debug, Clone)]
pub enum TinkererAction {
    Propose(BrickOnFace),
    Commit,
    JoinIfPair(HashSet<UniqueId>),
    InitiateRevert,
}

#[derive(Debug, Clone)]
pub enum CrucibleAction {
    BakeBrick(usize),
    BuildFabric(FabricPlan),
    SetSpeed(usize),
    RevertTo(Fabric),
    StartPretensing(SurfaceCharacter),
    StartTinkering,
    Tinkerer(TinkererAction)
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
                if plan_runner.is_done() {
                    self.stage = if self.fabric.faces.is_empty() {
                        PretensingLaunch(plan_runner.surface_character())
                    } else {
                        TinkeringLaunch
                    }
                } else {
                    for _ in 0..self.iterations_per_frame {
                        plan_runner.iterate(&mut self.fabric);
                    }
                }
            }
            TinkeringLaunch => {
                actions.push(Action::Keyboard(MenuAction::TinkerMenu));
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
            PretensingLaunch(surface_character) => {
                actions.push(Action::Scene(SceneAction::Variant(SceneVariant::Pretensing)));
                self.stage = Pretensing(Pretenser::new(PRETENST_FACTOR, *surface_character))
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
            Tinkerer(tinkerer_action) => {
                let Tinkering(tinkerer) = &mut self.stage else {
                    panic!("must be tinkering");
                };
                tinkerer.action(tinkerer_action);
            }
            SetSpeed(iterations_per_frame) => {
                self.iterations_per_frame = iterations_per_frame;
            }
            RevertTo(frozen) => {
                self.fabric = frozen;
            }
            StartPretensing(surface_character) => {
                self.stage = PretensingLaunch(surface_character);
            }
            StartTinkering => {
                self.stage = TinkeringLaunch;
            }
        }
    }

    pub fn fabric(&self) -> &Fabric {
        &self.fabric
    }

    pub fn is_tinkering(&self) -> bool {
        matches!(&self.stage, Tinkering(_))
    }

    pub fn is_brick_proposed(&self) -> bool {
        match &self.stage {
            Tinkering(tinkerer) => tinkerer.is_brick_proposed(),
            _ => false
        }
    }

    pub fn is_history_available(&self) -> bool {
        match &self.stage {
            Tinkering(tinkerer) => tinkerer.is_history_available(),
            _ => false
        }
    }

    pub fn is_pretenst_complete(&self) -> bool {
        match &self.stage {
            Pretensing(pretenser) => pretenser.is_done(),
            _ => false
        }
    }
}
