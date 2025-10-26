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
                    // Update the scale and check for orphan joints
                    context.fabric.scale = plan_runner.get_scale();
                    context.fabric.check_orphan_joints();
                    let pretenser = Pretenser::new(plan_runner.pretense_phase(), &self.radio);
                    pretenser.copy_physics_into(&mut context);
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
                    *context.physics = pretenser.physics().clone();

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
                let oven = Oven::new(prototype, self.radio.clone());

                context.replace_fabric(oven.fabric.clone());

                // Initialize the physics for baking
                oven.copy_physics_into(&mut context);

                context.transition_to(BakingBrick(oven));
            }
            BuildFabric(fabric_plan) => {
                // Get the name from the fabric plan
                let name = fabric_plan.name.clone();

                let plan_runner = PlanRunner::new(fabric_plan);

                // Reset the fabric to an empty one with the plan's name
                context.replace_fabric(Fabric::new(name));

                plan_runner.copy_physics_into(&mut context);
                context.transition_to(RunningPlan(plan_runner));

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
                    let fabric_clone = context.fabric.clone();
                    let tester = FailureTester::new(
                        scenario.clone(),
                        &fabric_clone,
                        physics_clone.clone(),
                        self.radio.clone(),
                    );

                    context.replace_fabric(tester.fabric().clone());

                    tester.adopt_physica(&mut context);

                    context.transition_to(FailureTesting(tester));

                    context.send_event(LabEvent::UpdateState(SetControlState(
                        ControlState::FailureTesting(scenario),
                    )));
                } else {
                    panic!("cannot start experiment");
                }
            }
            ToPhysicsTesting(scenario) => {
                if let Viewing = &mut self.stage {
                    let tester = PhysicsTester::new(
                        context.fabric.clone(),
                        physics_clone.clone(),
                        self.radio.clone(),
                    );

                    context.replace_fabric(tester.fabric.clone());

                    tester.copy_physics_into(&mut context);

                    context.transition_to(PhysicsTesting(tester));

                    context.send_event(LabEvent::UpdateState(SetControlState(
                        ControlState::PhysicsTesting(scenario),
                    )));
                } else {
                    panic!("cannot start experiment");
                }
            }
            TesterDo(action) => match &mut self.stage {
                FailureTesting(tester) => {
                    tester.action(action);

                    context.replace_fabric(tester.fabric().clone());
                }
                PhysicsTesting(tester) => {
                    tester.action(action);

                    context.replace_fabric(tester.fabric.clone());
                }
                _ => {}
            },
            ToEvolving(seed) => {
                let evolution = Evolution::new(seed);

                context.replace_fabric(evolution.fabric.clone());

                // Initialize the physics for evolution
                evolution.adopt_physica(&mut context);

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
