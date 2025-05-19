use crate::build::evo::evolution::Evolution;
use crate::build::oven::Oven;
use crate::build::tenscript::brick_library::BrickLibrary;
use crate::build::tenscript::plan_runner::PlanRunner;
use crate::build::tenscript::pretenser::Pretenser;
use crate::crucible::Stage::*;
use crate::crucible_context::CrucibleContext;
use crate::fabric::physics::presets::AIR_GRAVITY;
use crate::fabric::physics::Physics;
use crate::fabric::Fabric;
use crate::testing::boxing_test::BoxingTest;
use crate::testing::failure_test::FailureTester;
use crate::testing::physics_test::PhysicsTester;
use crate::{ControlState, CrucibleAction, LabEvent, Radio, StateChange};

pub enum Stage {
    Empty,
    RunningPlan(PlanRunner),
    Pretensing(Pretenser),
    Viewing,
    Animating,
    FailureTesting(FailureTester),
    PhysicsTesting(PhysicsTester),
    BoxingTesting(BoxingTest),
    BakingBrick(Oven),
    Evolving(Evolution),
}

pub struct Crucible {
    stage: Stage,
    radio: Radio,
    pub fabric: Fabric,
    pub physics: Physics,
}

impl Crucible {
    pub fn new(radio: Radio) -> Self {
        Self {
            stage: Empty,
            radio,
            fabric: Fabric::new("Empty".to_string()),
            physics: AIR_GRAVITY,
        }
    }
}

impl Crucible {
    pub fn iterate(&mut self, brick_library: &BrickLibrary) {
        // Create a context for this iteration
        let mut context = CrucibleContext::new(
            &mut self.fabric,
            &mut self.physics,
            &self.radio,
            brick_library,
        );

        match &mut self.stage {
            Empty => {}
            RunningPlan(plan_runner) => {
                if plan_runner.is_done() {
                    // Get a copy of the fabric from the plan runner
                    let mut new_fabric = plan_runner.fabric.clone();
                    new_fabric.scale = plan_runner.get_scale();
                    new_fabric.check_orphan_joints();

                    // Replace the current fabric
                    context.replace_fabric(new_fabric);

                    // Create a new pretenser and transition to it
                    let pretenser = Pretenser::new(plan_runner.pretense_phase());
                    context.transition_to(Pretensing(pretenser));
                } else {
                    // Pass the context to the plan runner's iterate method
                    if let Err(tenscript_error) = plan_runner.iterate(&mut context) {
                        println!("Error:\n{tenscript_error}");
                        plan_runner.disable(tenscript_error);
                    }
                }
            }
            Pretensing(pretenser) => {
                if pretenser.is_done() {
                    // Get stats before transitioning
                    let stats = context.fabric.fabric_stats();

                    // Update the context's physics directly
                    context.replace_physics(pretenser.physics().clone());

                    // Transition to viewing stage
                    context.transition_to(Viewing);

                    // Queue the FabricBuilt event
                    context.queue_event(LabEvent::FabricBuilt(stats));
                } else {
                    // Pass the context to the pretenser's iterate method
                    pretenser.iterate(&mut context);
                }
            }
            Viewing => {
                // Handle viewing (not animating)
                // Use the context's physics and fabric

                // Use the physics-defined number of iterations
                for _ in context.physics.iterations() {
                    // Iterate the context's fabric directly
                    context.fabric.iterate(context.physics);
                }
            }
            Animating => {
                // Handle animating
                // Muscles are already activated when transitioning to this state
                // Calling activate_muscles(true) here would reset muscle_nuance to 0.5
                // and interfere with the natural oscillation

                // Use the physics-defined number of iterations
                for _ in context.physics.iterations() {
                    // Iterate the context's fabric directly
                    context.fabric.iterate(context.physics);
                }
            }
            FailureTesting(tester) => {
                // Pass the context to the tester's iterate method
                tester.iterate(&mut context);
            }
            PhysicsTesting(tester) => {
                // Pass the context to the tester's iterate method
                tester.iterate(&mut context);
            }
            BoxingTesting(tester) => {
                // Pass the context to the tester's iterate method
                tester.iterate(&mut context);
            }
            BakingBrick(oven) => {
                // Pass the context to the oven's iterate method
                if let Some(baked) = oven.iterate(&mut context) {
                    #[cfg(target_arch = "wasm32")]
                    println!("Baked {:?}", baked.into_tenscript());
                    #[cfg(not(target_arch = "wasm32"))]
                    std::fs::write("baked-brick.tenscript", baked.into_tenscript()).unwrap();
                }
            }
            Evolving(evolution) => {
                // Pass the context to the evolution's iterate method
                evolution.iterate(&mut context);
            }
        }

        // Apply any stage transition requested by the context
        if let Some(new_stage) = context.apply_changes() {
            self.stage = new_stage;
        }
    }

    pub fn action(&mut self, crucible_action: CrucibleAction) {
        use CrucibleAction::*;
        use StateChange::*;

        // Create a dummy brick library for the context
        // We don't actually use it in actions, but the context requires it
        let dummy_brick_library = BrickLibrary::from_source().unwrap_or_else(|_| {
            // If we can't load the brick library, create an empty one
            // This is just a placeholder for the context and won't be used
            BrickLibrary {
                brick_definitions: Vec::new(),
                baked_bricks: Vec::new(),
            }
        });

        // Clone physics for use in testers to avoid borrow checker issues
        let physics_clone = self.physics.clone();

        // Create a context for this action
        let mut context = CrucibleContext::new(
            &mut self.fabric,
            &mut self.physics,
            &self.radio,
            &dummy_brick_library,
        );

        match crucible_action {
            BakeBrick(prototype) => {
                // Create a new Oven with a fresh fabric
                let oven = Oven::new(prototype, self.radio.clone());

                // Update our fabric with the oven's fabric
                context.replace_fabric(oven.fabric.clone());

                // Transition to BakingBrick stage
                context.transition_to(BakingBrick(oven));
            }
            BuildFabric(fabric_plan) => {
                // Create a new PlanRunner
                let plan_runner = PlanRunner::new(fabric_plan);

                // Reset the fabric to an empty one
                context.replace_fabric(Fabric::new("Building".to_string()));

                // Transition to RunningPlan stage
                context.transition_to(RunningPlan(plan_runner));

                // Send events
                context.send_event(LabEvent::UpdateState(SetControlState(
                    ControlState::UnderConstruction,
                )));
                context.send_event(LabEvent::UpdateState(SetFabricStats(None)));
            }
            ToViewing => match &mut self.stage {
                Viewing => {
                    context.send_event(LabEvent::UpdateState(SetControlState(
                        ControlState::Viewing,
                    )));
                }
                Animating => {
                    // Deactivate muscles when transitioning back to Viewing
                    context.fabric.activate_muscles(false);
                    
                    self.stage = Viewing;

                    context.send_event(LabEvent::UpdateState(SetControlState(
                        ControlState::Viewing,
                    )));
                }
                _ => {}
            },
            ToAnimating => {
                if let Viewing = &mut self.stage {
                    self.stage = Animating;

                    // Activate muscles through the context
                    context.fabric.activate_muscles(true);

                    context.send_event(LabEvent::UpdateState(SetControlState(
                        ControlState::Animating,
                    )));
                }
            }
            ToFailureTesting(scenario) => {
                if let Viewing = &mut self.stage {
                    // Create a new FailureTester using the cloned physics
                    let tester = FailureTester::new(
                        scenario.clone(),
                        context.fabric,
                        physics_clone.clone(),
                        self.radio.clone(),
                    );

                    // Update our fabric with the tester's fabric
                    context.replace_fabric(tester.fabric().clone());

                    // Transition to FailureTesting stage
                    context.transition_to(FailureTesting(tester));

                    // Send event
                    context.send_event(LabEvent::UpdateState(SetControlState(
                        ControlState::FailureTesting(scenario),
                    )));
                } else {
                    panic!("cannot start experiment");
                }
            }
            ToPhysicsTesting(scenario) => {
                if let Viewing = &mut self.stage {
                    // Create a new PhysicsTester using the cloned physics
                    let tester = PhysicsTester::new(
                        context.fabric,
                        physics_clone.clone(),
                        self.radio.clone(),
                    );

                    // Update our fabric with the tester's fabric
                    context.replace_fabric(tester.fabric.clone());

                    // Transition to PhysicsTesting stage
                    context.transition_to(PhysicsTesting(tester));

                    // Send event
                    context.send_event(LabEvent::UpdateState(SetControlState(
                        ControlState::PhysicsTesting(scenario),
                    )));
                } else {
                    panic!("cannot start experiment");
                }
            }
            ToBoxingProcess(scenario) => {
                if let Viewing = &mut self.stage {
                    // Create a new BoxingTest using the cloned physics
                    let test = BoxingTest::new(context.fabric, physics_clone.clone());

                    // Update our fabric with the tester's fabric
                    context.replace_fabric(test.fabric.clone());

                    // Transition to BoxingTesting stage
                    context.transition_to(BoxingTesting(test));

                    // Send event
                    context.send_event(LabEvent::UpdateState(SetControlState(
                        ControlState::BoxingTesting(scenario),
                    )));
                } else {
                    panic!("cannot start experiment");
                }
            }
            TesterDo(action) => match &mut self.stage {
                FailureTesting(tester) => {
                    tester.action(action);

                    // Update our fabric with the tester's fabric
                    context.replace_fabric(tester.fabric().clone());
                }
                PhysicsTesting(tester) => {
                    tester.action(action);

                    // Update our fabric with the tester's fabric
                    context.replace_fabric(tester.fabric.clone());
                }
                BoxingTesting(tester) => {
                    tester.action(action);

                    // Update our fabric with the tester's fabric
                    context.replace_fabric(tester.fabric.clone());
                }
                _ => {}
            },
            ToEvolving(seed) => {
                // Create a new Evolution
                let evolution = Evolution::new(seed);

                // Update our fabric with the evolution's fabric
                context.replace_fabric(evolution.fabric.clone());

                // Transition to Evolving stage
                context.transition_to(Evolving(evolution));
            }
        }

        // Apply any stage transition requested by the context
        if let Some(new_stage) = context.apply_changes() {
            self.stage = new_stage;
        }
    }

    pub fn update_attachment_connections(&mut self) {
        // Directly update the attachment connections on the main fabric
        self.fabric.update_all_attachment_connections();
    }

    // fabric() and fabric_mut() methods removed - access fabric directly
}
