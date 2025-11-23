/// FabricPlanExecutor: A unified machine that executes a complete FabricPlan
/// in pure fabric time (iterations), independent of frames.
///
/// This machine orchestrates BUILD → PRETENSE → CONVERGE phases and can be used
/// identically by both UI (headful) and headless tests.
///
/// The executor maintains an execution log of critical events with precise timing,
/// which can be inspected and verified in tests.

use crate::build::dsl::brick_library::BrickLibrary;
use crate::build::dsl::plan_runner::PlanRunner;
use crate::build::dsl::pretenser::Pretenser;
use crate::build::dsl::FabricPlan;
use crate::build::converger::Converger;
use crate::fabric::Fabric;
use crate::fabric::physics::Physics;
use crate::fabric::physics::presets::{CONSTRUCTION, PRETENSING};
use crate::fabric::physics::SurfaceCharacter;
use crate::units::Percent;

#[derive(Debug, PartialEq)]
pub enum IterateResult {
    Continue,
    Complete,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ExecutorStage {
    Building,
    Pretensing,
    Converging,
    Complete,
}

/// Events that occur during fabric plan execution
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionEvent {
    /// Construction started
    Started { iteration: usize },
    /// Transitioned to a new stage
    StageTransition { iteration: usize, from: String, to: String },
    /// Growth step executed
    GrowthStep { iteration: usize, joint_count: usize },
    /// Growth completed
    GrowthComplete { iteration: usize, final_joint_count: usize },
    /// Faces removed during pretension
    FacesRemoved { iteration: usize, removed_count: usize, remaining_joints: usize },
    /// Pretension applied
    PretensionApplied { iteration: usize, pretenst_percent: f32 },
    /// Physics changed
    PhysicsChanged { iteration: usize, description: String },
    /// Construction completed
    Completed { iteration: usize },
}

impl ExecutionEvent {
    pub fn iteration(&self) -> usize {
        match self {
            ExecutionEvent::Started { iteration } => *iteration,
            ExecutionEvent::StageTransition { iteration, .. } => *iteration,
            ExecutionEvent::GrowthStep { iteration, .. } => *iteration,
            ExecutionEvent::GrowthComplete { iteration, .. } => *iteration,
            ExecutionEvent::FacesRemoved { iteration, .. } => *iteration,
            ExecutionEvent::PretensionApplied { iteration, .. } => *iteration,
            ExecutionEvent::PhysicsChanged { iteration, .. } => *iteration,
            ExecutionEvent::Completed { iteration } => *iteration,
        }
    }

    pub fn fabric_time_seconds(&self) -> f32 {
        self.iteration() as f32 / 4000.0
    }
}

impl std::fmt::Display for ExecutionEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let time = format!("{:8.2}s", self.fabric_time_seconds());
        let iter = format!("iter {:7}", self.iteration());

        match self {
            ExecutionEvent::Started { .. } =>
                write!(f, "[{} | {}] Construction started", time, iter),
            ExecutionEvent::StageTransition { from, to, .. } =>
                write!(f, "[{} | {}] {} → {}", time, iter, from, to),
            ExecutionEvent::GrowthStep { joint_count, .. } =>
                write!(f, "[{} | {}] Growth step (joints: {})", time, iter, joint_count),
            ExecutionEvent::GrowthComplete { final_joint_count, .. } =>
                write!(f, "[{} | {}] Growth complete (final joints: {})", time, iter, final_joint_count),
            ExecutionEvent::FacesRemoved { removed_count, remaining_joints, .. } =>
                write!(f, "[{} | {}] Removed {} faces (joints: {})", time, iter, removed_count, remaining_joints),
            ExecutionEvent::PretensionApplied { pretenst_percent, .. } =>
                write!(f, "[{} | {}] Pretension applied ({}%)", time, iter, pretenst_percent),
            ExecutionEvent::PhysicsChanged { description, .. } =>
                write!(f, "[{} | {}] Physics: {}", time, iter, description),
            ExecutionEvent::Completed { .. } =>
                write!(f, "[{} | {}] Construction completed", time, iter),
        }
    }
}

pub struct FabricPlanExecutor {
    stage: ExecutorStage,
    plan_runner: Option<PlanRunner>,
    pretenser: Option<Pretenser>,
    converger: Option<Converger>,
    pub fabric: Fabric,
    pub physics: Physics,
    plan: FabricPlan,
    current_iteration: usize,
    execution_log: Vec<ExecutionEvent>,
    pending_camera_translation: Option<cgmath::Vector3<f32>>,
    stored_surface_character: Option<SurfaceCharacter>,
}

impl FabricPlanExecutor {
    pub fn new(plan: FabricPlan) -> Self {
        let fabric = Fabric::new(plan.name.clone());
        let plan_runner = PlanRunner::new(plan.clone());
        let physics = CONSTRUCTION;

        let mut executor = Self {
            stage: ExecutorStage::Building,
            plan_runner: Some(plan_runner),
            pretenser: None,
            converger: None,
            fabric,
            physics,
            plan,
            current_iteration: 0,
            execution_log: Vec::new(),
            pending_camera_translation: None,
            stored_surface_character: None,
        };

        executor.log_event(ExecutionEvent::Started { iteration: 0 });
        executor
    }

    fn log_event(&mut self, event: ExecutionEvent) {
        self.execution_log.push(event);
    }

    /// Get the execution log
    pub fn execution_log(&self) -> &[ExecutionEvent] {
        &self.execution_log
    }

    /// Print the execution log to stderr
    pub fn print_log(&self) {
        eprintln!("\n=== Execution Log ===");
        for event in &self.execution_log {
            eprintln!("{}", event);
        }
        eprintln!("=== End Log ({} events) ===\n", self.execution_log.len());
    }

    /// Find events of a specific type
    pub fn find_events<F>(&self, predicate: F) -> Vec<&ExecutionEvent>
    where
        F: Fn(&ExecutionEvent) -> bool,
    {
        self.execution_log.iter().filter(|e| predicate(e)).collect()
    }

    /// Get the current iteration count
    pub fn current_iteration(&self) -> usize {
        self.current_iteration
    }

    /// Execute one iteration of physics and check for stage transitions.
    /// This is frame-independent and operates in pure fabric time.
    /// Returns Complete when the execution is finished, Continue otherwise.
    pub fn iterate(&mut self, brick_library: &BrickLibrary) -> IterateResult {
        self.current_iteration += 1;

        // Run one physics iteration
        self.fabric.iterate(&self.physics);

        // Check for stage transitions
        match self.stage {
            ExecutorStage::Building => {
                // Collect information we need, then log after borrows end
                let mut events_to_log = Vec::new();

                if let Some(plan_runner) = &mut self.plan_runner {
                    use crate::build::dsl::plan_context::PlanContext;
                    let mut context = PlanContext::new(&mut self.fabric, &mut self.physics, brick_library);

                    let prev_stage = plan_runner.stage;
                    let was_growing = plan_runner.build_phase.is_growing();

                    // Always check and advance stage - plan_runner handles progress checking internally
                    plan_runner.check_and_advance_stage_simple(&mut context);

                    // Check if we should log growth steps
                    let new_stage = plan_runner.stage;
                    if prev_stage != new_stage {
                        if was_growing && prev_stage == crate::build::dsl::plan_runner::Stage::GrowStep {
                            events_to_log.push(ExecutionEvent::GrowthStep {
                                iteration: self.current_iteration,
                                joint_count: self.fabric.joints.len(),
                            });
                        }
                    }

                    // Check if BUILD phase growth is complete
                    let build_complete = plan_runner.is_done();
                    if build_complete {
                        events_to_log.push(ExecutionEvent::GrowthComplete {
                            iteration: self.current_iteration,
                            final_joint_count: self.fabric.joints.len(),
                        });
                    }

                    // Now log events
                    for event in events_to_log {
                        self.log_event(event);
                    }

                    // Transition to PRETENSE if BUILD is complete
                    if build_complete {
                        self.transition_to_pretense();
                    }
                } else {
                    // Now log events
                    for event in events_to_log {
                        self.log_event(event);
                    }
                }
            }
            ExecutorStage::Pretensing => {
                // Check if pretension is complete (progress is no longer busy)
                if !self.fabric.progress.is_busy() {
                    // Pretension complete - check if we should transition to converge
                    if self.plan.converge_phase.is_some() {
                        self.transition_to_converge();
                    } else {
                        self.complete();
                    }
                }
            }
            ExecutorStage::Converging => {
                // Use fabric's built-in progress tracking for convergence
                // Update physics with convergence progress for gradually increasing damping
                let progress = self.fabric.progress.nuance();
                self.physics.update_convergence_progress(progress);

                // Check if convergence is complete
                if !self.fabric.progress.is_busy() {
                    self.complete();
                }
            }
            ExecutorStage::Complete => {}
        }

        // Return Complete when execution is finished, Continue otherwise
        if self.stage == ExecutorStage::Complete {
            IterateResult::Complete
        } else {
            IterateResult::Continue
        }
    }

    /// Access to plan_runner for tests (temporary, until refactoring is complete)
    pub fn plan_runner(&self) -> Option<&PlanRunner> {
        self.plan_runner.as_ref()
    }

    pub fn plan_runner_mut(&mut self) -> Option<&mut PlanRunner> {
        self.plan_runner.as_mut()
    }

    fn transition_to_pretense(&mut self) {
        self.log_event(ExecutionEvent::StageTransition {
            iteration: self.current_iteration,
            from: "BUILD".to_string(),
            to: "PRETENSE".to_string(),
        });

        // Preserve user's scaling tweaks
        let mass_scale = self.physics.mass_scale();
        let rigidity_scale = self.physics.rigidity_scale();

        // Set fabric scale from plan
        if let Some(plan_runner) = &self.plan_runner {
            self.fabric.scale = plan_runner.get_scale();
        }

        // Remove faces
        let face_count_before = self.fabric.faces.len();
        let face_ids: Vec<_> = self.fabric.faces.keys().copied().collect();
        for face_id in face_ids {
            let face = self.fabric.face(face_id);
            if !face.has_prism {
                self.fabric.add_face_triangle(face_id);
            }
            self.fabric.remove_face(face_id);
        }

        self.log_event(ExecutionEvent::FacesRemoved {
            iteration: self.current_iteration,
            removed_count: face_count_before,
            remaining_joints: self.fabric.joints.len(),
        });

        // Apply pretension
        self.fabric.slacken();
        let altitude = self.plan.pretense_phase.altitude
            .unwrap_or(0.0) / self.fabric.scale;
        let translation = self.fabric.centralize_translation(Some(altitude));
        self.fabric.apply_translation(translation);

        // Store camera translation for Crucible to apply
        self.pending_camera_translation = Some(translation);

        let pretenst_percent = self.plan.pretense_phase.pretenst
            .map(|p| Percent(p))
            .unwrap_or(PRETENSING.pretenst);
        let pretense_duration = self.plan.pretense_phase.seconds
            .unwrap_or(crate::units::Seconds(15.0));
        self.fabric.set_pretenst(pretenst_percent, pretense_duration);

        self.log_event(ExecutionEvent::PretensionApplied {
            iteration: self.current_iteration,
            pretenst_percent: pretenst_percent.0,
        });

        // Switch to PRETENSING physics
        self.physics = PRETENSING;

        // Store surface_character from plan for later use during CONVERGE
        // PRETENSING should NOT have gravity - that only appears during CONVERGE
        if let Some(surface) = self.plan.pretense_phase.surface_character {
            self.stored_surface_character = Some(surface);
        }

        self.log_event(ExecutionEvent::PhysicsChanged {
            iteration: self.current_iteration,
            description: "PRETENSING".to_string(),
        });

        // Restore user's scaling tweaks
        use crate::TweakFeature::*;
        self.physics.accept_tweak(MassScale.parameter(mass_scale));
        self.physics.accept_tweak(RigidityScale.parameter(rigidity_scale));

        self.plan_runner = None; // No longer needed
        self.stage = ExecutorStage::Pretensing;
    }

    fn transition_to_converge(&mut self) {
        if self.plan.converge_phase.is_some() {
            self.log_event(ExecutionEvent::StageTransition {
                iteration: self.current_iteration,
                from: "PRETENSE".to_string(),
                to: "CONVERGE".to_string(),
            });

            // Preserve user's scaling tweaks before switching physics
            let mass_scale = self.physics.mass_scale();
            let rigidity_scale = self.physics.rigidity_scale();

            // Switch to BASE_PHYSICS for convergence (lower drag allows visible falling)
            use crate::fabric::physics::presets::BASE_PHYSICS;
            self.physics = BASE_PHYSICS;

            // NOW apply the surface_character stored during PRETENSE
            // This is when gravity should appear!
            if let Some(surface) = self.stored_surface_character {
                self.physics.surface_character = surface;
            }

            // Restore user's scaling tweaks
            use crate::TweakFeature::*;
            self.physics.accept_tweak(MassScale.parameter(mass_scale));
            self.physics.accept_tweak(RigidityScale.parameter(rigidity_scale));

            // Enable convergence mode (gradually increases damping over time)
            self.physics.enable_convergence();

            self.log_event(ExecutionEvent::PhysicsChanged {
                iteration: self.current_iteration,
                description: "CONVERGING".to_string(),
            });

            // Start progress tracking for convergence duration
            let converge_phase = self.plan.converge_phase.as_ref().unwrap();
            self.fabric.progress.start(converge_phase.seconds);

            self.pretenser = None; // No longer needed
            self.stage = ExecutorStage::Converging;
        } else {
            // No converge phase - go directly to Complete
            self.complete();
        }
    }

    fn complete(&mut self) {
        self.log_event(ExecutionEvent::Completed {
            iteration: self.current_iteration,
        });

        self.fabric.zero_velocities();
        self.fabric.frozen = true;
        self.converger = None;
        self.stage = ExecutorStage::Complete;
    }

    pub fn is_complete(&self) -> bool {
        self.stage == ExecutorStage::Complete
    }

    pub fn stage(&self) -> &ExecutorStage {
        &self.stage
    }

    /// Manually trigger transition to PRETENSE phase
    /// This should be called when BUILD phase is complete and you're ready to apply pretension
    pub fn start_pretension(&mut self) {
        if self.stage == ExecutorStage::Building {
            self.transition_to_pretense();
        }
    }

    /// Take and clear any pending camera translation
    pub fn take_camera_translation(&mut self) -> Option<cgmath::Vector3<f32>> {
        self.pending_camera_translation.take()
    }
}
