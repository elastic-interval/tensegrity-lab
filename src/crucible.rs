use crate::animator::Animator;
use crate::build::evo::evolution::Evolution;
use crate::build::failure_test::FailureTester;
use crate::build::oven::Oven;
use crate::build::physics_test::PhysicsTester;
use crate::build::tenscript::brick_library::BrickLibrary;
use crate::build::tenscript::plan_runner::PlanRunner;
use crate::build::tenscript::pretense_phase::PretensePhase;
use crate::build::tenscript::pretenser::Pretenser;
use crate::crucible::Stage::*;
use crate::fabric::physics::Physics;
use crate::fabric::Fabric;
use crate::messages::{ControlState, CrucibleAction, LabEvent, Radio, StateChange};
use crate::ITERATIONS_PER_FRAME;

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

pub struct Crucible {
    fabric: Fabric,
    iterations_per_frame: usize,
    stage: Stage,
    radio: Radio,
}

impl Crucible {
    pub(crate) fn new(radio: Radio) -> Self {
        Self {
            fabric: Fabric::default(),
            iterations_per_frame: ITERATIONS_PER_FRAME,
            stage: Empty,
            radio,
        }
    }
}

impl Crucible {
    pub fn iterate(&mut self, brick_library: &BrickLibrary) {
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
                    LabEvent::FabricBuilt(self.fabric.fabric_stats()).send(&self.radio);
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
        use StateChange::*;
        use CrucibleAction::*;
        match crucible_action {
            BakeBrick(prototype) => {
                let oven = Oven::new(prototype);
                self.fabric = oven.prototype_fabric();
                self.stage = BakingBrick(oven);
            }
            BuildFabric(fabric_plan) => {
                self.fabric = Fabric::default();
                self.stage = RunningPlan(PlanRunner::new(fabric_plan));
                ControlState::UnderConstruction.send(&self.radio);
                SetFabricStats(None).send(&self.radio);
            }
            AdjustSpeed(change) => {
                let mut iterations = (self.iterations_per_frame as f32 * change) as usize;
                if iterations == self.iterations_per_frame && change > 1.0 {
                    iterations += 1;
                }
                self.iterations_per_frame = iterations.clamp(1, 5000);
                SetIterationsPerFrame(self.iterations_per_frame).send(&self.radio);
            }
            ToViewing => match &mut self.stage {
                Viewing(_) => ControlState::Viewing.send(&self.radio),
                Animating(animator) => {
                    self.stage = Viewing(animator.physics.clone());
                    ControlState::Viewing.send(&self.radio);
                }
                _ => {}
            },
            ToAnimating => {
                if let Viewing(physics) = &mut self.stage {
                    self.stage = Animating(Animator::new(physics.clone()));
                    ControlState::Animating.send(&self.radio);
                }
            }
            ToFailureTesting(scenario) => {
                if let Viewing(physics) = &mut self.stage {
                    self.stage = FailureTesting(FailureTester::new(
                        scenario.clone(),
                        &self.fabric,
                        physics.clone(),
                        self.radio.clone(),
                    ));
                    ControlState::FailureTesting(scenario).send(&self.radio);
                } else {
                    panic!("cannot start experiment");
                }
            }
            ToPhysicsTesting(scenario) => {
                if let Viewing(physics) = &mut self.stage {
                    self.stage = PhysicsTesting(PhysicsTester::new(
                        &self.fabric,
                        physics.clone(),
                        self.radio.clone(),
                    ));
                    ControlState::PhysicsTesting(scenario).send(&self.radio);
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
            ToEvolving(seed) => {
                self.stage = Evolving(Evolution::new(seed));
            }
        }
    }

    pub fn fabric(&mut self) -> &Fabric {
        match &mut self.stage {
            FailureTesting(tester) => tester.fabric(),
            PhysicsTesting(tester) => tester.fabric(),
            _ => &self.fabric,
        }
    }
}
