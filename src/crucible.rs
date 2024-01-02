use std::collections::HashSet;

use CrucibleAction::{*};

use crate::build::oven::Oven;
use crate::build::tenscript::brick::Prototype;
use crate::build::tenscript::brick_library::BrickLibrary;
use crate::build::tenscript::FabricPlan;
use crate::build::tenscript::plan_runner::PlanRunner;
use crate::build::tenscript::pretense_phase::PretensePhase;
use crate::build::tenscript::pretenser::Pretenser;
use crate::build::tinkerer::{BrickOnFace, Tinkerer};
use crate::crucible::Stage::{*};
use crate::fabric::{Fabric, UniqueId};
use crate::fabric::lab::Lab;
use crate::scene::{SceneAction, SceneVariant};
use crate::user_interface::{Action, MenuAction};

const PULL_SHORTENING: f32 = 0.95;

enum Stage {
    Empty,
    RunningPlan(PlanRunner),
    TinkeringLaunch,
    Tinkering(Tinkerer),
    PretensingLaunch(PretensePhase),
    Pretensing(Pretenser),
    Experimenting(Lab),
    BakingBrick(Oven),
    Finished,
}

#[derive(Debug, Clone)]
pub enum TinkererAction {
    Propose(BrickOnFace),
    Clear,
    Commit,
    JoinIfPair(HashSet<UniqueId>),
    InitiateRevert,
}

#[derive(Debug, Clone)]
pub enum LabAction {
    GravityChanged(f32),
    MuscleTest,
    MuscleChanged(f32),
}

#[derive(Debug, Clone)]
pub enum CrucibleAction {
    BakeBrick(Prototype),
    BuildFabric(FabricPlan),
    SetSpeed(usize),
    RevertTo(Fabric),
    StartPretensing(PretensePhase),
    StartTinkering,
    Tinkerer(TinkererAction),
    Experiment(LabAction),
    ActivateMuscles,
}

pub struct Crucible {
    fabric: Fabric,
    iterations_per_frame: usize,
    stage: Stage,
}

#[derive(Default, Debug, Clone)]
pub struct CrucibleState {
    pub tinkering: bool,
    pub brick_proposed: bool,
    pub experimenting: bool,
    pub history_available: bool,
}

impl Default for Crucible {
    fn default() -> Self {
        Self {
            fabric: Fabric::default(),
            iterations_per_frame: 125,
            stage: Empty,
        }
    }
}

impl Crucible {
    pub fn iterate(&mut self, paused: bool, brick_library: &BrickLibrary) -> Vec<Action> {
        let mut actions = Vec::new();
        match &mut self.stage {
            Empty => {}
            RunningPlan(plan_runner) => {
                if plan_runner.is_done() {
                    self.stage = if self.fabric.faces.is_empty() {
                        PretensingLaunch(plan_runner.pretense_phase())
                    } else {
                        TinkeringLaunch
                    }
                } else {
                    for _ in 0..self.iterations_per_frame {
                        if let Err(tenscript_error) = plan_runner.iterate(&mut self.fabric, brick_library) {
                            println!("Error:\n{tenscript_error}");
                            plan_runner.disable(tenscript_error);
                            break;
                        }
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
                    if let Some(tinker_action) = tinkerer.iterate(&mut self.fabric, brick_library) {
                        actions.push(tinker_action);
                    }
                }
            }
            PretensingLaunch(pretense_phase) => {
                actions.push(Action::Scene(SceneAction::Variant(SceneVariant::Pretensing)));
                self.stage = Pretensing(Pretenser::new(pretense_phase.clone()))
            }
            Pretensing(pretenser) => {
                for _ in 0..self.iterations_per_frame {
                    pretenser.iterate(&mut self.fabric);
                }
                if pretenser.is_done() {
                    actions.push(Action::UpdateMenu);
                    self.stage = Experimenting(Lab::new(pretenser.clone()));
                }
            }
            Experimenting(lab) => {
                for _ in 0..self.iterations_per_frame {
                    lab.iterate(&mut self.fabric);
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
            BakeBrick(prototype) => {
                let oven = Oven::new(prototype);
                self.fabric = oven.prototype_fabric();
                self.stage = BakingBrick(oven);
            }
            BuildFabric(fabric_plan) => {
                self.fabric = Fabric::default();
                self.stage = RunningPlan(PlanRunner::new(fabric_plan));
            }
            Tinkerer(tinkerer_action) => {
                let Tinkering(tinkerer) = &mut self.stage else {
                    panic!("must be tinkering");
                };
                tinkerer.action(tinkerer_action);
            }
            Experiment(lab_action) => {
                let Experimenting(lab) = &mut self.stage else {
                    panic!("must be experimenting");
                };
                lab.action(lab_action, &mut self.fabric);
            }
            SetSpeed(iterations_per_frame) => {
                self.iterations_per_frame = iterations_per_frame;
            }
            RevertTo(frozen) => {
                self.fabric = frozen;
            }
            StartPretensing(pretenst_phase) => {
                self.stage = PretensingLaunch(pretenst_phase);
            }
            StartTinkering => {
                self.stage = TinkeringLaunch;
            }
            ActivateMuscles => {
                let Experimenting(lab) = &mut self.stage else {
                    panic!("must be experimenting");
                };
                lab.action(LabAction::MuscleTest, &mut self.fabric);
            }
        }
    }

    pub fn fabric(&self) -> &Fabric {
        &self.fabric
    }

    pub fn state(&self) -> CrucibleState {
        CrucibleState {
            tinkering: matches!(&self.stage, Tinkering(_)),
            brick_proposed: match &self.stage {
                Tinkering(tinkerer) => tinkerer.is_brick_proposed(),
                _ => false
            },
            experimenting: matches!(self.stage, Experimenting(_)),
            history_available: match &self.stage {
                Tinkering(tinkerer) => tinkerer.is_history_available(),
                _ => false
            },
        }
    }
}
