use crate::build::evo::evolution::Evolution;
use crate::build::experiment::Experiment;
use crate::build::oven::Oven;
use crate::build::tenscript::brick::Prototype;
use crate::build::tenscript::brick_library::BrickLibrary;
use crate::build::tenscript::plan_runner::PlanRunner;
use crate::build::tenscript::pretense_phase::PretensePhase;
use crate::build::tenscript::pretenser::Pretenser;
use crate::build::tenscript::FabricPlan;
use crate::crucible::Stage::*;
use crate::fabric::Fabric;
use crate::messages::LabEvent;
use winit::event_loop::EventLoopProxy;

enum Stage {
    Empty,
    RunningPlan(PlanRunner),
    PretensingLaunch(PretensePhase),
    Pretensing(Pretenser),
    Experimenting(Experiment),
    BakingBrick(Oven),
    Evolving(Evolution),
}

#[derive(Debug, Clone)]
pub enum LabAction {
    ToggleMusclesActive,
    GravityChanged(f32),
    MusclesActive(bool),
    MuscleChanged(f32),
    NextExperiment(bool),
}

#[derive(Debug, Clone)]
pub enum CrucibleAction {
    BakeBrick(Prototype),
    BuildFabric(Option<FabricPlan>),
    SetSpeed(f32),
    RevertTo(Fabric),
    StartPretensing(PretensePhase),
    Experiment(LabAction),
    Evolve(u64),
}

pub struct Crucible {
    fabric: Fabric,
    iterations_per_frame: usize,
    stage: Stage,
    event_loop_proxy: EventLoopProxy<LabEvent>,
}

impl Crucible {
    pub(crate) fn new(event_loop_proxy: EventLoopProxy<LabEvent>) -> Self {
        Self {
            fabric: Fabric::default(),
            iterations_per_frame: 160,
            stage: Empty,
            event_loop_proxy,
        }
    }
}

impl Crucible {
    pub fn iterate(&mut self, brick_library: &BrickLibrary) {
        let send = |lab_event: LabEvent| self.event_loop_proxy.send_event(lab_event).unwrap();
        match &mut self.stage {
            Empty => {}
            RunningPlan(plan_runner) => {
                if plan_runner.is_done() {
                    self.fabric.scale = plan_runner.get_scale();
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
                    self.stage = Experimenting(Experiment::new(
                        pretenser.clone(),
                        &self.fabric,
                        true,
                        self.event_loop_proxy.clone(),
                    ));
                    send(LabEvent::FabricBuilt(self.fabric.fabric_stats()));
                }
            }
            Experimenting(lab) => {
                for _ in 0..self.iterations_per_frame {
                    lab.iterate()
                }
            }
            BakingBrick(oven) => {
                if let Some(baked) = oven.iterate(&mut self.fabric) {
                    #[cfg(target_arch = "wasm32")]
                    println!("Baked {:?}", baked.into_tenscript());
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        std::fs::write("baked-brick.tenscript", baked.into_tenscript()).unwrap();
                        std::process::exit(0);
                    }
                }
            }
            Evolving(evolution) => {
                evolution.iterate(&mut self.fabric);
            }
        }
    }

    pub fn action(&mut self, crucible_action: CrucibleAction)  {
        use CrucibleAction::*;
        match crucible_action {
            BakeBrick(prototype) => {
                let oven = Oven::new(prototype);
                self.fabric = oven.prototype_fabric();
                self.stage = BakingBrick(oven);
            }
            BuildFabric(fabric_plan) => {
                self.fabric = Fabric::default();
                if let Some(fabric_plan) = fabric_plan {
                    self.stage = RunningPlan(PlanRunner::new(fabric_plan));
                }
            }
            Experiment(lab_action) => {
                if let Experimenting(lab) = &mut self.stage {
                    lab.action(lab_action)
                };
            }
            SetSpeed(change) => {
                self.iterations_per_frame = (self.iterations_per_frame as f32 * change) as usize;
                if self.iterations_per_frame <= 0 {
                    self.iterations_per_frame = 0;
                }
            }
            RevertTo(frozen) => {
                self.fabric = frozen;
            }
            StartPretensing(pretenst_phase) => {
                self.stage = PretensingLaunch(pretenst_phase);
            }
            Evolve(seed) => {
                self.stage = Evolving(Evolution::new(seed));
            }
        }
    }

    pub fn fabric(&mut self) -> &Fabric {
        if let Experimenting(experiment) = &mut self.stage {
            experiment.fabric()
        } else {
            &self.fabric
        }
    }
}
