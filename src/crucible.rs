use crate::animator::Animator;
use crate::application::AppStateChange;
use crate::build::evo::evolution::Evolution;
use crate::build::failure_test::{FailureTester, FailureTesterAction};
use crate::build::oven::Oven;
use crate::build::physics_test::{PhysicsTester, PhysicsTesterAction};
use crate::build::tenscript::brick::Prototype;
use crate::build::tenscript::brick_library::BrickLibrary;
use crate::build::tenscript::plan_runner::PlanRunner;
use crate::build::tenscript::pretense_phase::PretensePhase;
use crate::build::tenscript::pretenser::Pretenser;
use crate::build::tenscript::FabricPlan;
use crate::crucible::Stage::*;
use crate::fabric::physics::Physics;
use crate::fabric::Fabric;
use crate::messages::{ControlState, LabEvent, TestScenario};
use crate::ITERATIONS_PER_FRAME;
use winit::event_loop::EventLoopProxy;

enum Stage {
    Empty,
    RunningPlan(PlanRunner),
    PretensingLaunch(PretensePhase),
    Pretensing(Pretenser),
    Viewing(Physics),
    Animating(Animator),
    FailureTesting(FailureTester),
    PhysicsTesting(PhysicsTester),
    BakingBrick(Oven),
    Evolving(Evolution),
}

#[derive(Debug, Clone)]
pub enum CrucibleAction {
    BakeBrick(Prototype),
    BuildFabric(FabricPlan),
    ToFailureTesting(TestScenario),
    ToPhysicsTesting(TestScenario),
    FailureTesterDo(FailureTesterAction),
    PhysicsTesterDo(PhysicsTesterAction),
    StartEvolving(u64),
    AdjustSpeed(f32),
    ViewingToAnimating,
    ToViewing,
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
            iterations_per_frame: ITERATIONS_PER_FRAME,
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
            FailureTesting(tester) => {
                for _ in 0..self.iterations_per_frame {
                    tester.iterate()
                }
            }
            PhysicsTesting(tester) => {
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
        use AppStateChange::*;
        use CrucibleAction::*;
        let send = |change: AppStateChange| {
            self.event_loop_proxy
                .send_event(LabEvent::AppStateChanged(change))
                .unwrap()
        };
        match crucible_action {
            BakeBrick(prototype) => {
                let oven = Oven::new(prototype);
                self.fabric = oven.prototype_fabric();
                self.stage = BakingBrick(oven);
            }
            BuildFabric(fabric_plan) => {
                self.fabric = Fabric::default();
                self.stage = RunningPlan(PlanRunner::new(fabric_plan));
                send(SetControlState(ControlState::UnderConstruction));
                send(SetFabricStats(None))
            }
            AdjustSpeed(change) => {
                let mut iterations = (self.iterations_per_frame as f32 * change) as usize;
                if iterations == self.iterations_per_frame && change > 1.0 {
                    iterations += 1;
                }
                self.iterations_per_frame = iterations.clamp(1, 5000);
                send(SetIterationsPerFrame(self.iterations_per_frame));
            }
            ToViewing => {
                match &mut self.stage {
                    Viewing(_) => {
                        send(SetControlState(ControlState::Viewing))
                    }
                    Animating(animator) => {
                        self.stage = Viewing(animator.physics.clone());
                        send(SetControlState(ControlState::Viewing));
                    }
                    _ => {}
                }
            }
            ViewingToAnimating => {
                if let Viewing(physics) = &mut self.stage {
                    self.stage = Animating(Animator::new(physics.clone()));
                    send(SetControlState(ControlState::Animating));
                }
            }
            ToFailureTesting(scenario) => {
                if let Viewing(physics) = &mut self.stage {
                    self.stage = FailureTesting(FailureTester::new(
                        scenario.clone(),
                        &self.fabric,
                        physics.clone(),
                        self.event_loop_proxy.clone(),
                    ));
                    send(SetControlState(ControlState::FailureTesting(scenario)));
                } else {
                    panic!("cannot start experiment");
                }
            }
            ToPhysicsTesting(scenario) => {
                if let Viewing(physics) = &mut self.stage {
                    self.stage = PhysicsTesting(PhysicsTester::new(
                        &self.fabric,
                        physics.clone(),
                        self.event_loop_proxy.clone(),
                    ));
                    send(SetControlState(ControlState::PhysicsTesting(scenario)));
                } else {
                    panic!("cannot start experiment");
                }
            }
            FailureTesterDo(action) => {
                if let FailureTesting(tester) = &mut self.stage {
                    tester.action(action);
                }
            }
            PhysicsTesterDo(action) => {
                if let PhysicsTesting(tester) = &mut self.stage {
                    tester.action(action);
                }
            }
            StartEvolving(seed) => {
                self.stage = Evolving(Evolution::new(seed));
            }
        }
    }

    pub fn fabric(&mut self) -> &Fabric {
        match &mut self.stage {
            FailureTesting(tester) => {tester.fabric()}
            PhysicsTesting(tester) => {tester.fabric()}
            _ => {&self.fabric}
        }
    }
}
