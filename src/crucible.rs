use crate::build::animator::Animator;
use crate::build::dsl::brick_builders::build_brick_library;
use crate::build::evo::evolution::Evolution;
use crate::build::oven::Oven;
use crate::build::dsl::brick_library::BrickLibrary;
use crate::build::dsl::fabric_plan_executor::FabricPlanExecutor;
use crate::build::dsl::FabricPlan;
use crate::crucible::Stage::*;
use crate::crucible_context::CrucibleContext;
use crate::fabric::physics::presets::BASE_PHYSICS;
use crate::fabric::physics::Physics;
use crate::fabric::physics_test::PhysicsTester;
use crate::fabric::Fabric;
use crate::{ControlState, CrucibleAction, LabEvent, Radio, StateChange};
use StateChange::*;

pub enum Stage {
    Empty,
    RunningPlan(FabricPlanExecutor),
    Viewing,
    Animating(Animator),
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
    last_stage_label: Option<String>,
}

impl Crucible {
    pub fn new(radio: Radio) -> Self {
        Self {
            stage: Empty,
            radio,
            fabric: Fabric::new("Empty".to_string()),
            physics: BASE_PHYSICS,
            pending_camera_translation: None,
            fabric_plan: None,
            last_stage_label: None,
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

    /// Get the target time scale for the current stage
    /// 5.0 for construction (5× speedup), 1.0 for physics testing (real time), 0.0 for viewing
    pub fn target_time_scale(&self) -> f32 {
        match &self.stage {
            PhysicsTesting(_) => 1.0,
            Viewing => 0.0,
            _ => 5.0,
        }
    }
}

impl Crucible {
    /// Finalize and transition to Viewing stage
    fn finalize_to_viewing(&mut self) {
        self.fabric.zero_velocities();
        self.fabric.frozen = true;
        self.physics.disable_convergence();
        self.stage = Viewing;
    }

    pub fn iterate(&mut self, brick_library: &BrickLibrary, iterations_per_frame: usize) {
        // Check if RunningPlan completed and needs transition
        if let RunningPlan(executor) = &mut self.stage {
            use crate::build::dsl::fabric_plan_executor::IterateResult;

            for _ in 0..iterations_per_frame {
                match executor.iterate(brick_library) {
                    IterateResult::Complete => {
                        // Sync fabric and physics from executor
                        self.fabric = executor.fabric.clone();
                        self.physics = executor.physics.clone();
                        // Send FabricBuilt with complete stats (including dynamics)
                        let stats = self.fabric.stats_with_dynamics(&self.physics);
                        let _ = self.radio.send_event(LabEvent::FabricBuilt(stats));
                        // Check for and apply camera translation before finalizing
                        if let Some(translation) = executor.take_camera_translation() {
                            self.pending_camera_translation = Some(translation);
                        }
                        // Finalize and exit immediately
                        self.finalize_to_viewing();
                        return;
                    }
                    IterateResult::Continue => {
                        // Continue iterating
                    }
                }
            }

            // Always sync fabric and physics from executor to Crucible
            self.fabric = executor.fabric.clone();
            self.physics = executor.physics.clone();

            // Check for and apply camera translation from executor
            if let Some(translation) = executor.take_camera_translation() {
                self.pending_camera_translation = Some(translation);
            }

            // Check if BUILD phase is done and we should start PRETENSE
            use crate::build::dsl::fabric_plan_executor::ExecutorStage;
            if matches!(executor.stage(), ExecutorStage::Building) {
                if let Some(plan_runner) = executor.plan_runner() {
                    if plan_runner.is_done() {
                        // BUILD phase complete - start PRETENSE
                        executor.start_pretension();
                    }
                }
            }

            // Send stage label updates based on executor stage
            let stage_label = match executor.stage() {
                ExecutorStage::Building => "Building".to_string(),
                ExecutorStage::Pretensing => {
                    format!("Pretensing {}", self.fabric.progress.countdown())
                }
                ExecutorStage::Converging => {
                    format!("Converging {}", self.fabric.progress.countdown())
                }
                ExecutorStage::Complete => "Complete".to_string(),
            };

            // Always update during Pretensing/Converging (for countdown), otherwise only when changed
            let should_update = matches!(
                executor.stage(),
                ExecutorStage::Pretensing | ExecutorStage::Converging
            ) || self
                .last_stage_label
                .as_ref()
                .map_or(true, |s| s != &stage_label);

            if should_update {
                self.last_stage_label = Some(stage_label.clone());
                let _ = self
                    .radio
                    .send_event(LabEvent::UpdateState(SetStageLabel(stage_label)));
            }

            return;
        }

        // Handle other stages
        match &mut self.stage {
            Empty => {}
            RunningPlan(_) => {} // Already handled above
            Viewing => {}
            Animating(animator) => {
                // Create a context for animator
                let mut context = CrucibleContext::new(
                    &mut self.fabric,
                    &mut self.physics,
                    &self.radio,
                    brick_library,
                );
                animator.iterate(&mut context);

                // Apply any stage transition and camera translation
                let (new_stage, camera_translation) = context.apply_changes();
                if let Some(new_stage) = new_stage {
                    self.stage = new_stage;
                }
                if let Some(translation) = camera_translation {
                    self.pending_camera_translation = Some(translation);
                }
            }
            PhysicsTesting(tester) => {
                // Create a context for tester
                let mut context = CrucibleContext::new(
                    &mut self.fabric,
                    &mut self.physics,
                    &self.radio,
                    brick_library,
                );
                tester.iterate(&mut context, iterations_per_frame);

                // Apply any stage transition
                let (new_stage, camera_translation) = context.apply_changes();
                if let Some(new_stage) = new_stage {
                    self.stage = new_stage;
                }
                if let Some(translation) = camera_translation {
                    self.pending_camera_translation = Some(translation);
                }
            }
            BakingBrick(oven) => {
                // Create a context for oven
                let mut context = CrucibleContext::new(
                    &mut self.fabric,
                    &mut self.physics,
                    &self.radio,
                    brick_library,
                );
                if let Some(baked) = oven.iterate(&mut context) {
                    panic!("Better way to bake bricks please?: {:?}", baked);
                }

                // Apply any stage transition
                let (new_stage, camera_translation) = context.apply_changes();
                if let Some(new_stage) = new_stage {
                    self.stage = new_stage;
                }
                if let Some(translation) = camera_translation {
                    self.pending_camera_translation = Some(translation);
                }
            }
            Evolving(evolution) => {
                // Create a context for evolution
                let mut context = CrucibleContext::new(
                    &mut self.fabric,
                    &mut self.physics,
                    &self.radio,
                    brick_library,
                );
                evolution.iterate(&mut context);

                // Apply any stage transition
                let (new_stage, camera_translation) = context.apply_changes();
                if let Some(new_stage) = new_stage {
                    self.stage = new_stage;
                }
                if let Some(translation) = camera_translation {
                    self.pending_camera_translation = Some(translation);
                }
            }
        }
    }

    pub fn action(&mut self, crucible_action: CrucibleAction) {
        use CrucibleAction::*;

        // Handle BuildFabric separately (doesn't need context since executor owns fabric/physics)
        if let BuildFabric(fabric_plan) = crucible_action {
            // Preserve user's scaling tweaks before rebuilding
            let mass_scale = self.physics.mass_scale();
            let rigidity_scale = self.physics.rigidity_scale();

            let name = fabric_plan.name.clone();
            let mut executor = FabricPlanExecutor::new(fabric_plan.clone());

            // Store the fabric_plan for later use (animate, converge, etc.)
            self.fabric_plan = Some(fabric_plan);

            // Apply user's scaling tweaks to executor's physics before construction
            use crate::TweakFeature::*;
            executor
                .physics
                .accept_tweak(MassScale.parameter(mass_scale));
            executor
                .physics
                .accept_tweak(RigidityScale.parameter(rigidity_scale));

            // Sync executor's fabric/physics to Crucible (will be updated each frame)
            self.fabric = executor.fabric.clone();
            self.physics = executor.physics.clone();

            // Transition to RunningPlan with executor
            self.stage = RunningPlan(executor);

            // Set fabric name immediately so title appears right away
            let _ = self
                .radio
                .send_event(LabEvent::UpdateState(SetFabricName(name.clone())));
            let _ = self
                .radio
                .send_event(LabEvent::UpdateState(SetStageLabel("Building".to_string())));
            let _ = self
                .radio
                .send_event(LabEvent::UpdateState(SetFabricStats(None)));
            return;
        }

        // Create a brick library for the context using Rust DSL
        let dummy_brick_library = BrickLibrary::new(build_brick_library());

        // Clone physics for passing to tester (avoids borrow checker issues)
        let tester_physics = self.physics.clone();

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
            BuildFabric(_) => {
                // Already handled above
                unreachable!()
            }
            CentralizeFabric(altitude) => {
                // Convert altitude from mm to internal coordinate system
                let altitude_internal = altitude.map(|mm| *mm / context.fabric.scale);
                let translation = context.fabric.centralize_translation(altitude_internal);
                context.fabric.apply_translation(translation);
                context.fabric.zero_velocities();
                context.send_event(LabEvent::FabricCentralized(translation));
            }
            ToViewing => match &mut self.stage {
                Viewing => {
                    context.send_event(LabEvent::UpdateState(SetControlState(
                        ControlState::Viewing,
                    )));
                }
                Animating(_) => {
                    // Unwrap muscles back to Fixed spans when transitioning back to Viewing
                    Animator::unwrap_muscles(&mut context);

                    self.stage = Viewing;

                    context.send_event(LabEvent::UpdateState(SetControlState(
                        ControlState::Viewing,
                    )));
                }
                PhysicsTesting(_) => {
                    // Freeze the fabric when exiting PhysicsTesting
                    context.fabric.zero_velocities();
                    context.fabric.frozen = true;

                    self.stage = Viewing;

                    context.send_event(LabEvent::UpdateState(SetControlState(
                        ControlState::Viewing,
                    )));
                    context.send_event(LabEvent::UpdateState(SetStageLabel("Viewing".to_string())));
                }
                _ => {}
            },
            ToAnimating => {
                if let Viewing = &mut self.stage {
                    // Only animate if we have a fabric plan with an animate phase
                    if let Some(animate_phase) = self
                        .fabric_plan
                        .as_ref()
                        .and_then(|p| p.animate_phase.as_ref())
                    {
                        // Create animator and transition to Animating stage
                        let animator = Animator::new(animate_phase.clone(), &mut context);
                        self.stage = Animating(animator);

                        context.send_event(LabEvent::UpdateState(SetControlState(
                            ControlState::Animating,
                        )));
                    }
                }
            }
            ToPhysicsTesting(scenario) => {
                if let Viewing = &mut self.stage {
                    let mut fabric = context.fabric.clone();
                    // Unfreeze the fabric so physics changes can take effect
                    fabric.frozen = false;

                    let tester = PhysicsTester::new(fabric, tester_physics, self.radio.clone());

                    context.replace_fabric(tester.fabric.clone());

                    tester.copy_physics_into(&mut context);

                    context.transition_to(PhysicsTesting(tester));

                    context.send_event(LabEvent::UpdateState(SetControlState(
                        ControlState::PhysicsTesting(scenario),
                    )));
                    context.send_event(LabEvent::UpdateState(SetStageLabel(
                        "Testing Physics".to_string(),
                    )));

                    // Send initial stats with convergence data
                    context.send_event(LabEvent::UpdateState(SetFabricStats(Some(
                        context.fabric.stats_with_dynamics(context.physics),
                    ))));
                }
                // Silently ignore if not in Viewing stage - can only test physics after convergence
            }
            TesterDo(action) => match &mut self.stage {
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
