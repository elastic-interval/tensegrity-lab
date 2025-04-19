use crate::build::evo::evolution::Evolution;
use crate::build::oven::Oven;
use crate::build::tenscript::brick_library::BrickLibrary;
use crate::build::tenscript::plan_runner::PlanRunner;
use crate::build::tenscript::pretenser::Pretenser;
use crate::crucible::Stage::*;
use crate::fabric::physics::Physics;
use crate::fabric::Fabric;
use crate::testing::boxing_test::BoxingTest;
use crate::testing::failure_test::FailureTester;
use crate::testing::physics_test::PhysicsTester;
use crate::{ControlState, CrucibleAction, LabEvent, Radio, StateChange};

#[derive(Debug, Clone)]
pub struct Holder {
    pub fabric: Fabric,
    pub physics: Physics,
}

enum Stage {
    Empty,
    RunningPlan(PlanRunner),
    Pretensing(Pretenser),
    Viewing(Holder),
    Animating(Holder),
    FailureTesting(FailureTester),
    PhysicsTesting(PhysicsTester),
    BoxingTesting(BoxingTest),
    BakingBrick(Oven),
    Evolving(Evolution),
}

pub struct Crucible {
    stage: Stage,
    radio: Radio,
}

impl Crucible {
    pub fn new(radio: Radio) -> Self {
        Self {
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
                    plan_runner.fabric.scale = plan_runner.get_scale();
                    plan_runner.fabric.check_orphan_joints();
                    self.stage = Pretensing(Pretenser::new(
                        plan_runner.pretense_phase(),
                        plan_runner.fabric.clone(),
                    ));
                } else {
                    for _ in plan_runner.physics.iterations() {
                        if let Err(tenscript_error) = plan_runner.iterate(brick_library) {
                            println!("Error:\n{tenscript_error}");
                            plan_runner.disable(tenscript_error);
                            break;
                        }
                    }
                }
            }
            Pretensing(pretenser) => {
                if pretenser.is_done() {
                    let stats = pretenser.fabric.fabric_stats();
                    let holder = pretenser.holder();
                    self.stage = Viewing(holder);
                    LabEvent::FabricBuilt(stats).send(&self.radio);
                } else {
                    for _ in pretenser.physics.iterations() {
                        pretenser.iterate();
                    }
                }
            }
            Viewing(Holder { fabric, physics }) => {
                for _ in physics.iterations() {
                    fabric.iterate(physics);
                }
            }
            Animating(Holder { fabric, physics }) => {
                for _ in physics.iterations() {
                    fabric.iterate(physics);
                }
            }
            FailureTesting(tester) => {
                for _ in tester.physics.iterations() {
                    tester.iterate()
                }
            }
            PhysicsTesting(tester) => {
                for _ in tester.physics.iterations() {
                    tester.iterate()
                }
            }
            BoxingTesting(tester) => {
                for _ in tester.physics.iterations() {
                    tester.iterate()
                }
            }
            BakingBrick(oven) => {
                if let Some(baked) = oven.iterate() {
                    #[cfg(target_arch = "wasm32")]
                    println!("Baked {:?}", baked.into_tenscript());
                    #[cfg(not(target_arch = "wasm32"))]
                    std::fs::write("baked-brick.tenscript", baked.into_tenscript()).unwrap();
                }
            }
            Evolving(evolution) => {
                evolution.iterate();
            }
        }
    }

    pub fn action(&mut self, crucible_action: CrucibleAction) {
        use CrucibleAction::*;
        use StateChange::*;
        match crucible_action {
            BakeBrick(prototype) => {
                self.stage = BakingBrick(Oven::new(prototype, self.radio.clone()));
            }
            BuildFabric(fabric_plan) => {
                self.stage = RunningPlan(PlanRunner::new(fabric_plan));
                ControlState::UnderConstruction.send(&self.radio);
                SetFabricStats(None).send(&self.radio);
            }
            ToViewing => match &mut self.stage {
                Viewing { .. } => ControlState::Viewing.send(&self.radio),
                Animating(holder) => {
                    holder.fabric.activate_muscles(false);
                    self.stage = Viewing(holder.clone());
                    ControlState::Viewing.send(&self.radio);
                }
                _ => {}
            },
            ToAnimating => {
                if let Viewing(holder) = &mut self.stage {
                    holder.fabric.activate_muscles(true);
                    self.stage = Animating(holder.clone());
                    ControlState::Animating.send(&self.radio);
                }
            }
            ToFailureTesting(scenario) => {
                if let Viewing(Holder { fabric, physics }) = &mut self.stage {
                    self.stage = FailureTesting(FailureTester::new(
                        scenario.clone(),
                        &fabric,
                        physics.clone(),
                        self.radio.clone(),
                    ));
                    ControlState::FailureTesting(scenario).send(&self.radio);
                } else {
                    panic!("cannot start experiment");
                }
            }
            ToPhysicsTesting(scenario) => {
                if let Viewing(Holder { fabric, physics }) = &mut self.stage {
                    self.stage = PhysicsTesting(PhysicsTester::new(
                        &fabric,
                        physics.clone(),
                        self.radio.clone(),
                    ));
                    ControlState::PhysicsTesting(scenario).send(&self.radio);
                } else {
                    panic!("cannot start experiment");
                }
            }
            ToBoxingProcess(scenario) => {
                if let Viewing(Holder { fabric, physics }) = &mut self.stage {
                    self.stage = BoxingTesting(BoxingTest::new(&fabric, physics.clone()));
                    ControlState::BoxingTesting(scenario).send(&self.radio);
                } else {
                    panic!("cannot start experiment");
                }
            }
            TesterDo(action) => match &mut self.stage {
                FailureTesting(tester) => {
                    tester.action(action);
                }
                PhysicsTesting(tester) => {
                    tester.action(action);
                }
                _ => {}
            },
            ToEvolving(seed) => {
                self.stage = Evolving(Evolution::new(seed));
            }
        }
    }

    pub fn fabric(&mut self) -> &Fabric {
        match &mut self.stage {
            FailureTesting(tester) => tester.fabric(),
            PhysicsTesting(tester) => &tester.fabric,
            BoxingTesting(tester) => &tester.fabric,
            RunningPlan(plan_runner) => &plan_runner.fabric,
            Pretensing(pretenser) => &pretenser.fabric,
            Viewing(holder) => &holder.fabric,
            Animating(holder) => &holder.fabric,
            BakingBrick(oven) => &oven.fabric,
            Evolving(evolution) => &evolution.fabric,
            Empty => unreachable!(),
        }
    }
}
