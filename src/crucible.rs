use CrucibleAction::*;

use crate::build::oven::Oven;
use crate::build::tenscript::brick::Prototype;
use crate::build::tenscript::brick_library::BrickLibrary;
use crate::build::tenscript::plan_runner::PlanRunner;
use crate::build::tenscript::pretense_phase::PretensePhase;
use crate::build::tenscript::pretenser::Pretenser;
use crate::build::tenscript::FabricPlan;
use crate::crucible::Stage::*;
use crate::build::experiment::Experiment;
use crate::build::tenscript::fabric_plan::DEFAULT_BUILD_ALTITUDE;
use crate::fabric::Fabric;
use crate::messages::LabEvent;

enum Stage {
    Empty,
    RunningPlan(PlanRunner),
    PretensingLaunch(PretensePhase),
    Pretensing(Pretenser),
    Experimenting(Experiment),
    BakingBrick(Oven),
}

#[derive(Debug, Clone)]
pub enum LabAction {
    GravityChanged(f32),
    MuscleTest(bool),
    MuscleChanged(f32),
}

#[derive(Debug, Clone)]
pub enum CrucibleAction {
    BakeBrick(Prototype),
    BuildFabric(Option<FabricPlan>),
    SetSpeed(usize),
    RevertTo(Fabric),
    StartPretensing(PretensePhase),
    Experiment(LabAction),
}

pub struct Crucible {
    fabric: Fabric,
    iterations_per_frame: usize,
    stage: Stage,
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
    pub fn iterate(&mut self, brick_library: &BrickLibrary) -> Option<LabEvent> {
        match &mut self.stage {
            Empty => {}
            RunningPlan(plan_runner) => {
                if plan_runner.is_done() {
                    self.stage = PretensingLaunch(plan_runner.pretense_phase())
                } else {
                    for _ in 0..self.iterations_per_frame {
                        if let Err(tenscript_error) =
                            plan_runner.iterate(&mut self.fabric, brick_library)
                        {
                            println!("Error:\n{tenscript_error}");
                            plan_runner.disable(tenscript_error);
                            break;
                        }
                    }
                }
            }
            PretensingLaunch(pretense_phase) => {
                self.fabric.check_orphan_joints();
                self.stage = Pretensing(Pretenser::new(pretense_phase.clone()));
            }
            Pretensing(pretenser) => {
                for _ in 0..self.iterations_per_frame {
                    pretenser.iterate(&mut self.fabric);
                }
                if pretenser.is_done() {
                    self.stage = Experimenting(Experiment::new(pretenser.clone()));
                    return Some(LabEvent::FabricBuilt(self.fabric.fabric_stats()));
                }
            }
            Experimenting(lab) => {
                for _ in 0..self.iterations_per_frame {
                    lab.iterate(&mut self.fabric);
                }
            }
            BakingBrick(oven) => {
                if let Some(baked) = oven.iterate(&mut self.fabric) {
                    #[cfg(target_arch = "wasm32")]
                    println!("Baked {:?}", baked.into_tenscript());
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        use std::fs;
                        use std::process;
                        fs::write("baked-brick.tenscript", baked.into_tenscript()).unwrap();
                        process::exit(0);
                    }
                }
            }
        }
        return None
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
                if let Some(fabric_plan) = fabric_plan {
                    self.fabric.altitude = Some(DEFAULT_BUILD_ALTITUDE);
                    self.stage = RunningPlan(PlanRunner::new(fabric_plan));
                }
            }
            Experiment(lab_action) => {
                if let Experimenting(lab) = &mut self.stage {
                    lab.action(lab_action, &mut self.fabric);
                };
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
        }
    }

    pub fn fabric(&self) -> &Fabric {
        &self.fabric
    }
}
