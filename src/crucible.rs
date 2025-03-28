use crate::animator::Animator;
use crate::application::AppStateChange;
use crate::build::evo::evolution::Evolution;
use crate::build::oven::Oven;
use crate::build::tenscript::brick::Prototype;
use crate::build::tenscript::brick_library::BrickLibrary;
use crate::build::tenscript::plan_runner::PlanRunner;
use crate::build::tenscript::pretense_phase::PretensePhase;
use crate::build::tenscript::pretenser::Pretenser;
use crate::build::tenscript::FabricPlan;
use crate::build::tester::Tester;
use crate::crucible::Stage::*;
use crate::fabric::physics::Physics;
use crate::fabric::Fabric;
use crate::messages::{ControlState, LabEvent};
use crate::ITERATIONS_PER_FRAME;
use winit::event_loop::EventLoopProxy;

enum Stage {
    Empty,
    RunningPlan(PlanRunner),
    PretensingLaunch(PretensePhase),
    Pretensing(Pretenser),
    Viewing(Physics),
    Animating(Animator),
    Testing(Tester),
    BakingBrick(Oven),
    Evolving(Evolution),
}

#[derive(Debug, Clone)]
pub enum TesterAction {
    GravityChanged(f32),
    NextExperiment(bool),
}

#[derive(Debug, Clone)]
pub enum CrucibleAction {
    BakeBrick(Prototype),
    BuildFabric {
        fabric_plan: FabricPlan,
        after_build: Option<CrucibleAction>,
    },
    SetSpeed(f32),
    StartExperiment(bool),
    TesterDo(TesterAction),
    Evolve(u64),
    StartAnimating,
    StopAnimating,
}

pub struct Crucible {
    fabric: Fabric,
    iterations_per_frame: usize,
    stage: Stage,
    after_build: Option<CrucibleAction>,
    event_loop_proxy: EventLoopProxy<LabEvent>,
}

impl Crucible {
    pub(crate) fn new(event_loop_proxy: EventLoopProxy<LabEvent>) -> Self {
        Self {
            fabric: Fabric::default(),
            iterations_per_frame: ITERATIONS_PER_FRAME,
            stage: Empty,
            after_build: None,
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
                    self.stage = Viewing(pretenser.physics.clone());
                    send(LabEvent::FabricBuilt(self.fabric.fabric_stats()));
                }
            }
            Viewing(physics) => {
                for _ in 0..self.iterations_per_frame {
                    self.fabric.iterate(physics);
                }
            }
            Animating(animator) => {
                for _ in 0..self.iterations_per_frame {
                    animator.iterate(&mut self.fabric);
                }
            }
            Testing(tester) => {
                for _ in 0..self.iterations_per_frame {
                    tester.iterate()
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

    pub fn action(&mut self, crucible_action: CrucibleAction) {
        use CrucibleAction::*;
        let send = |lab_event: LabEvent| self.event_loop_proxy.send_event(lab_event).unwrap();
        match crucible_action {
            BakeBrick(prototype) => {
                let oven = Oven::new(prototype);
                self.fabric = oven.prototype_fabric();
                self.stage = BakingBrick(oven);
            }
            BuildFabric {
                fabric_plan,
                after_build,
            } => {
                self.fabric = Fabric::default();
                self.stage = RunningPlan(PlanRunner::new(fabric_plan));
            }
            TesterDo(lab_action) => {
                if let Testing(lab) = &mut self.stage {
                    lab.action(lab_action)
                };
            }
            SetSpeed(change) => {
                let iterations = (self.iterations_per_frame as f32 * change) as usize;
                self.iterations_per_frame = iterations.clamp(1, 5000);
                send(LabEvent::AppStateChanged(
                    AppStateChange::SetIterationsPerFrame(self.iterations_per_frame),
                ));
            }
            StopAnimating => {
                if let Animating(animator) = &mut self.stage {
                    self.stage = Viewing(animator.physics.clone());
                    send(LabEvent::AppStateChanged(AppStateChange::SetControlState(
                        ControlState::Viewing,
                    )));
                }
            }
            StartAnimating => {
                if let Viewing(physics) = &mut self.stage {
                    self.stage = Animating(Animator::new(physics.clone()));
                    send(LabEvent::AppStateChanged(AppStateChange::SetControlState(
                        ControlState::Animating,
                    )));
                }
            }
            StartExperiment(tension) => {
                if let Viewing(physics) = &mut self.stage {
                    self.stage = Testing(Tester::new(
                        &self.fabric,
                        physics.clone(),
                        tension,
                        self.event_loop_proxy.clone(),
                    ));
                    send(LabEvent::AppStateChanged(AppStateChange::SetControlState(
                        ControlState::Testing(tension),
                    )));
                } else {
                    panic!("cannot start experiment");
                }
            }
            Evolve(seed) => {
                self.stage = Evolving(Evolution::new(seed));
            }
        }
    }

    pub fn fabric(&mut self) -> &Fabric {
        if let Testing(experiment) = &mut self.stage {
            experiment.fabric()
        } else {
            &self.fabric
        }
    }
}
