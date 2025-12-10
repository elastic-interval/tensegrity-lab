use crate::build::animator::Animator;
use crate::build::dsl::fabric_plan_executor::FabricPlanExecutor;
use crate::build::dsl::FabricPlan;
use crate::build::evo::evolution::Evolution;
use crate::build::oven::Oven;
use crate::crucible::Stage::*;
use crate::crucible_context::CrucibleContext;
use crate::fabric::physics::presets::{ANIMATING, VIEWING};
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
    fabric_plan: Option<FabricPlan>,
    last_stage_label: Option<String>,
}

impl Crucible {
    pub fn new(radio: Radio) -> Self {
        Self {
            stage: Empty,
            radio,
            fabric: Fabric::new("Empty".to_string()),
            physics: VIEWING,
            fabric_plan: None,
            last_stage_label: None,
        }
    }

    /// Check if animation is available for the current fabric plan
    pub fn animation_available(&self) -> bool {
        self.fabric_plan
            .as_ref()
            .map(|p| p.animate_phase.is_some())
            .unwrap_or(false)
    }

    /// Get ControlState::Viewing with correct animation_available flag
    pub fn viewing_state(&self) -> ControlState {
        ControlState::Viewing {
            animation_available: self.animation_available(),
        }
    }
}

impl Crucible {
    fn finalize_to_viewing(&mut self) {
        self.fabric.zero_velocities();
        self.physics = self.viewing_physics();
        self.stage = Viewing;
    }

    fn viewing_physics(&self) -> Physics {
        self.plan_physics(VIEWING)
    }

    fn animating_physics(&self) -> Physics {
        self.plan_physics(ANIMATING)
    }

    fn plan_physics(&self, base: Physics) -> Physics {
        self.fabric_plan
            .as_ref()
            .map(|plan| Physics {
                surface: plan.pretense_phase.surface,
                ..base.clone()
            })
            .unwrap_or(base)
    }

    pub fn iterate(&mut self, iterations_per_frame: usize) {
        // Check if RunningPlan completed and needs transition
        if let RunningPlan(executor) = &mut self.stage {
            use crate::build::dsl::fabric_plan_executor::IterateResult;

            for _ in 0..iterations_per_frame {
                match executor.iterate() {
                    IterateResult::Complete => {
                        // Sync fabric and physics from executor
                        self.fabric = executor.fabric.clone();
                        self.physics = executor.physics.clone();
                        // Send FabricBuilt with complete stats (including dynamics)
                        let _ = self.radio.send_event(LabEvent::FabricBuilt(self.fabric.fabric_stats()));
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
                ExecutorStage::Falling => {
                    format!("Falling {}", self.fabric.progress.countdown())
                }
                ExecutorStage::Settling => {
                    format!("Settling {}", self.fabric.progress.countdown())
                }
                ExecutorStage::Complete => "Complete".to_string(),
            };

            let should_update = matches!(
                executor.stage(),
                ExecutorStage::Pretensing | ExecutorStage::Falling | ExecutorStage::Settling
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
            Viewing => {
                // Run physics at real-time speed in viewing mode
                for _ in 0..iterations_per_frame {
                    self.fabric.iterate(&self.physics);
                }
            }
            Animating(animator) => {
                // Create a context for animator
                let mut context = CrucibleContext::new(
                    &mut self.fabric,
                    &mut self.physics,
                    &self.radio,
                );
                animator.iterate(&mut context, iterations_per_frame);

                // Apply any stage transition
                if let Some(new_stage) = context.apply_changes() {
                    self.stage = new_stage;
                }
            }
            PhysicsTesting(tester) => {
                // Create a context for tester
                let mut context = CrucibleContext::new(
                    &mut self.fabric,
                    &mut self.physics,
                    &self.radio,
                );
                tester.iterate(&mut context, iterations_per_frame);

                // Apply any stage transition
                if let Some(new_stage) = context.apply_changes() {
                    self.stage = new_stage;
                }
            }
            BakingBrick(oven) => {
                // Create a context for oven
                let mut context = CrucibleContext::new(
                    &mut self.fabric,
                    &mut self.physics,
                    &self.radio,
                );
                if let Some(new_fabric) = oven.iterate(&mut context) {
                    context.replace_fabric(new_fabric);
                }
            }
            Evolving(evolution) => {
                // Create a context for evolution
                let mut context = CrucibleContext::new(
                    &mut self.fabric,
                    &mut self.physics,
                    &self.radio,
                );
                evolution.iterate(&mut context);

                // Apply any stage transition
                if let Some(new_stage) = context.apply_changes() {
                    self.stage = new_stage;
                }
            }
        }
    }

    pub fn action(&mut self, crucible_action: CrucibleAction) {
        use CrucibleAction::*;

        // Handle BuildFabric separately (doesn't need context since executor owns fabric/physics)
        if let BuildFabric(fabric_plan) = crucible_action {
            // Preserve user's scaling tweaks before rebuilding
            let mass_multiplier = self.physics.mass_multiplier();
            let rigidity_multiplier = self.physics.rigidity_multiplier();

            let name = fabric_plan.name.clone();
            let mut executor = FabricPlanExecutor::new(fabric_plan.clone());

            // Store the fabric_plan for later use (animate, settle, etc.)
            self.fabric_plan = Some(fabric_plan);

            // Apply user's scaling tweaks to executor's physics before construction
            use crate::TweakFeature::*;
            executor
                .physics
                .accept_tweak(MassScale.parameter(mass_multiplier));
            executor
                .physics
                .accept_tweak(RigidityScale.parameter(rigidity_multiplier));

            // Sync executor's fabric/physics to Crucible (will be updated each frame)
            self.fabric = executor.fabric.clone();
            self.physics = executor.physics.clone();

            // Transition to RunningPlan with executor
            self.stage = RunningPlan(executor);

            // Set fabric name immediately so title appears right away
            let _ = self
                .radio
                .send_event(LabEvent::UpdateState(SetFabricName(name.to_string())));
            let _ = self
                .radio
                .send_event(LabEvent::UpdateState(SetStageLabel("Building".to_string())));
            let _ = self
                .radio
                .send_event(LabEvent::UpdateState(SetFabricStats(None)));
            return;
        }

        // Clone physics for passing to tester (avoids borrow checker issues)
        let tester_physics = self.physics.clone();
        let viewing_state = self.viewing_state();

        // Create a context for this action
        let mut context = CrucibleContext::new(
            &mut self.fabric,
            &mut self.physics,
            &self.radio,
        );

        match crucible_action {
            StartBaking => {
                let oven = Oven::new(self.radio.clone());
                let fresh_fabric = oven.create_fresh_fabric();
                StateChange::SetFabricName(format!("{}", oven.current_brick_name())).send(&self.radio);
                oven.send_stage_label();
                context.replace_fabric(fresh_fabric);
                // Initialize the physics for baking
                oven.copy_physics_into(&mut context);
                context.transition_to(BakingBrick(oven));
            }
            CycleBrick => {
                if let BakingBrick(oven) = &mut self.stage {
                    let fresh_fabric = oven.next_brick();
                    context.replace_fabric(fresh_fabric);
                }
            }
            BuildFabric(_) => {
                // Already handled above
                unreachable!()
            }
            DropFromHeight => {
                use crate::units::Millimeters;
                // Centralize fabric at 1m altitude - this handles everything
                CentralizeFabric(Some(Millimeters(1000.0))).send(&self.radio);
            }
            CentralizeFabric(altitude) => {
                // Convert altitude from mm to internal coordinate system
                let altitude_internal = altitude.map(|mm| *mm / context.fabric.scale);
                let translation = context.fabric.centralize_translation(altitude_internal);
                context.fabric.apply_translation(translation);
                context.fabric.zero_velocities();
            }
            ClearSelection => {
                // Clear UI selection without changing crucible stage
                let control_state = match &self.stage {
                    Viewing => viewing_state.clone(),
                    Animating(_) => ControlState::Animating,
                    _ => return,
                };
                context.send_event(LabEvent::UpdateState(SetControlState(control_state)));
            }
            AdjustAnimationPeriod(factor) => {
                if let Animating(animator) = &mut self.stage {
                    animator.adjust_period(factor);
                    let period = animator.period_secs();
                    context.send_event(LabEvent::UpdateState(SetStageLabel(
                        format!("Period: {:.4}s", period),
                    )));
                }
            }
            ToViewing => match &mut self.stage {
                Viewing => {
                    context.send_event(LabEvent::UpdateState(SetControlState(
                        viewing_state.clone(),
                    )));
                }
                Animating(animator) => {
                    animator.remove_actuators(&mut context);
                    self.stage = Viewing;

                    context.send_event(LabEvent::UpdateState(SetControlState(
                        viewing_state.clone(),
                    )));
                    drop(context);
                    self.physics = self.viewing_physics();
                    return;
                }
                PhysicsTesting(_) => {
                    context.fabric.zero_velocities();
                    self.stage = Viewing;
                    context.send_event(LabEvent::UpdateState(SetControlState(
                        viewing_state.clone(),
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
                        .cloned()
                    {
                        // Create animator and transition to Animating stage
                        let animator = Animator::new(animate_phase, &mut context);
                        self.stage = Animating(animator);

                        context.send_event(LabEvent::UpdateState(SetControlState(
                            ControlState::Animating,
                        )));
                        drop(context);
                        // Switch to animation physics (slow time)
                        self.physics = self.animating_physics();
                        return;
                    }
                }
            }
            ToPhysicsTesting(scenario) => {
                if let Viewing = &mut self.stage {
                    let tester = PhysicsTester::new(context.fabric.clone(), tester_physics, self.radio.clone());
                    context.replace_fabric(tester.fabric.clone());
                    tester.copy_physics_into(&mut context);
                    context.transition_to(PhysicsTesting(tester));
                    context.send_event(LabEvent::UpdateState(SetControlState(
                        ControlState::PhysicsTesting(scenario),
                    )));
                    context.send_event(LabEvent::UpdateState(SetStageLabel(
                        "Testing Physics".to_string(),
                    )));
                }
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

        // Apply any stage transition requested by the context
        if let Some(new_stage) = context.apply_changes() {
            self.stage = new_stage;
        }
    }

    pub fn update_attachment_connections(&mut self) {
        self.fabric.update_all_attachment_connections();
    }
}
