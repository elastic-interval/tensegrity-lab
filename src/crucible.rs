use crate::build::animator::Animator;
use crate::build::converger::Converger;
use crate::build::evo::evolution::Evolution;
use crate::build::oven::Oven;
use crate::build::tenscript::brick_library::BrickLibrary;
use crate::build::tenscript::plan_runner::PlanRunner;
use crate::build::tenscript::pretenser::Pretenser;
use crate::build::tenscript::FabricPlan;
use crate::crucible::Stage::*;
use crate::crucible_context::CrucibleContext;
use crate::fabric::physics::presets::AIR_GRAVITY;
use crate::fabric::physics::Physics;
use crate::fabric::Fabric;
use crate::testing::failure_test::FailureTester;
use crate::testing::physics_test::PhysicsTester;
use crate::{ControlState, CrucibleAction, LabEvent, PhysicsFeature, PhysicsParameter, Radio, StateChange};
use StateChange::*;

pub enum Stage {
    Empty,
    RunningPlan(PlanRunner),
    Pretensing(Pretenser),
    Converging(Converger),
    Viewing,
    Animating(Animator),
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
    pending_camera_translation: Option<cgmath::Vector3<f32>>,
    fabric_plan: Option<FabricPlan>,
}

impl Crucible {
    pub fn new(radio: Radio) -> Self {
        Self {
            stage: Empty,
            radio,
            fabric: Fabric::new("Empty".to_string()),
            physics: AIR_GRAVITY,
            pending_camera_translation: None,
            fabric_plan: None,
        }
    }

    /// Take and clear any pending camera translation
    pub fn take_camera_translation(&mut self) -> Option<cgmath::Vector3<f32>> {
        self.pending_camera_translation.take()
    }

    /// Set a pending camera translation
    pub fn set_camera_translation(&mut self, translation: cgmath::Vector3<f32>) {
        self.pending_camera_translation = Some(translation);
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
                    // Preserve user's base scales
                    let base_mass_scale = context.physics.base_mass_scale;
                    let base_rigidity_scale = context.physics.base_rigidity_scale;
                    
                    context.fabric.scale = plan_runner.get_scale();
                    context.fabric.check_orphan_joints();
                    let pretenser = Pretenser::new(plan_runner.pretense_phase(), &self.radio);
                    pretenser.copy_physics_into(&mut context);
                    
                    // Restore user's base scales after pretenser overwrites physics
                    context.physics.base_mass_scale = base_mass_scale;
                    context.physics.base_rigidity_scale = base_rigidity_scale;
                    context.physics.update_effective_scales();
                    
                    context.transition_to(Pretensing(pretenser));
                    
                    // Notify that we're pretensing
                    context.send_event(LabEvent::UpdateState(SetStageLabel("Pretensing".to_string())));
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
                    let base_mass_scale = context.physics.base_mass_scale;
                    let base_rigidity_scale = context.physics.base_rigidity_scale;
                    
                    let stats = context.fabric.fabric_stats();
                    *context.physics = pretenser.physics();
                    
                    context.physics.base_mass_scale = base_mass_scale;
                    context.physics.base_rigidity_scale = base_rigidity_scale;
                    context.physics.update_effective_scales();

                    // Check if converge phase is specified
                    if let Some(converge_phase) = self.fabric_plan.as_ref().and_then(|p| p.converge_phase.as_ref()) {
                        // Transition to Converging stage with specified time
                        context.transition_to(Converging(Converger::new(converge_phase)));
                        context.queue_event(LabEvent::FabricBuilt(stats));
                        context.send_event(LabEvent::UpdateState(SetStageLabel("Converging".to_string())));
                    } else {
                        // No converge phase - go directly to Viewing
                        context.fabric.zero_velocities();
                        context.fabric.frozen = true;
                        context.transition_to(Viewing);
                        context.queue_event(LabEvent::FabricBuilt(stats));
                        context.send_event(LabEvent::UpdateState(SetStageLabel("Viewing".to_string())));
                        context.send_event(LabEvent::UpdateState(SetFabricStats(
                            Some(context.fabric.stats_with_convergence(context.physics))
                        )));
                    }
                } else {
                    pretenser.iterate(&mut context);
                }
            }
            Converging(converger) => {
                converger.iterate(&mut context);
            }
            Viewing => {}
            Animating(animator) => {
                animator.iterate(&mut context);
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

        // Apply any stage transition and camera translation requested by the context
        let (new_stage, camera_translation) = context.apply_changes();
        if let Some(new_stage) = new_stage {
            self.stage = new_stage;
        }
        if let Some(translation) = camera_translation {
            self.pending_camera_translation = Some(translation);
        }
    }

    pub fn action(&mut self, crucible_action: CrucibleAction) {
        use CrucibleAction::*;

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
                // Preserve user's base scales before rebuilding
                let base_mass_scale = context.physics.base_mass_scale;
                let base_rigidity_scale = context.physics.base_rigidity_scale;
                
                let name = fabric_plan.name.clone();
                let mut plan_runner = PlanRunner::new(fabric_plan.clone());
                
                // Store the fabric_plan for later use (animate, converge, etc.)
                self.fabric_plan = Some(fabric_plan);
                
                // Apply user's base scales to plan_runner's physics before construction
                plan_runner.physics.base_mass_scale = base_mass_scale;
                plan_runner.physics.base_rigidity_scale = base_rigidity_scale;
                plan_runner.physics.update_effective_scales();

                context.replace_fabric(Fabric::new(name.clone()));
                plan_runner.copy_physics_into(&mut context);
                context.transition_to(RunningPlan(plan_runner));

                // Set fabric name immediately so title appears right away
                context.send_event(LabEvent::UpdateState(SetFabricName(name)));
                context.send_event(LabEvent::UpdateState(SetStageLabel("Building".to_string())));
                context.send_event(LabEvent::UpdateState(SetFabricStats(None)));
            }
            ToViewing => match &mut self.stage {
                Viewing => {
                    context.send_event(LabEvent::UpdateState(SetControlState(ControlState::Viewing)));
                }
                Animating(_) => {
                    // Unwrap muscles back to Fixed spans when transitioning back to Viewing
                    Animator::unwrap_muscles(&mut context);

                    self.stage = Viewing;

                    context.send_event(LabEvent::UpdateState(SetControlState(ControlState::Viewing)));
                }
                _ => {}
            },
            ToAnimating => {
                if let Viewing = &mut self.stage {
                    // Only animate if we have a fabric plan with an animate phase
                    if let Some(animate_phase) = self.fabric_plan.as_ref().and_then(|p| p.animate_phase.as_ref()) {
                        // Create animator and transition to Animating stage
                        let animator = Animator::new(animate_phase.clone(), &mut context);
                        self.stage = Animating(animator);

                        context.send_event(LabEvent::UpdateState(SetControlState(
                            ControlState::Animating,
                        )));
                    }
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
            IncreaseMass => {
                let new_mass = context.physics.base_mass_scale * 1.5;
                SetPhysicsParameter(PhysicsParameter {
                    feature: PhysicsFeature::MassScale,
                    value: new_mass,
                }).send(&context.radio);
            }
            IncreaseRigidity => {
                let new_rigidity = context.physics.base_rigidity_scale * 1.5;
                SetPhysicsParameter(PhysicsParameter {
                    feature: PhysicsFeature::RigidityScale,
                    value: new_rigidity,
                }).send(&context.radio);
            }
            ToEvolving(seed) => {
                let evolution = Evolution::new(seed);

                context.replace_fabric(evolution.fabric.clone());

                // Initialize the physics for evolution
                evolution.adopt_physica(&mut context);

                context.transition_to(Evolving(evolution));
            }
        }

        // Apply any stage transition and camera translation requested by the context
        let (new_stage, camera_translation) = context.apply_changes();
        if let Some(new_stage) = new_stage {
            self.stage = new_stage;
        }
        if let Some(translation) = camera_translation {
            self.pending_camera_translation = Some(translation);
        }
    }

    pub fn update_attachment_connections(&mut self) {
        // Directly update the attachment connections on the main fabric
        self.fabric.update_all_attachment_connections();
    }

    // fabric() and fabric_mut() methods removed - access fabric directly
}
