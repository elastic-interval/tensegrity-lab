use crate::build::settler::Settler;
use crate::build::dsl::plan_runner::PlanRunner;
use crate::build::dsl::pretenser::Pretenser;
use crate::build::dsl::FabricPlan;
use crate::fabric::physics::presets::{CONSTRUCTION, PRETENSING};
use crate::fabric::physics::Physics;
use crate::fabric::physics::SurfaceCharacter;
use crate::fabric::Fabric;
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
    Falling,
    Settling,
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
    settler: Option<Settler>,
    pub fabric: Fabric,
    pub physics: Physics,
    plan: FabricPlan,
    current_iteration: usize,
    execution_log: Vec<ExecutionEvent>,
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
            settler: None,
            fabric,
            physics,
            plan,
            current_iteration: 0,
            execution_log: Vec::new(),
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
    pub fn iterate(&mut self) -> IterateResult {
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
                    let mut context = PlanContext::new(&mut self.fabric, &mut self.physics);

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
                if !self.fabric.progress.is_busy() {
                    self.transition_to_fall();
                }
            }
            ExecutorStage::Falling => {
                if !self.fabric.progress.is_busy() {
                    self.transition_to_settle();
                }
            }
            ExecutorStage::Settling => {
                let progress = self.fabric.progress.nuance();
                self.physics.update_settling_progress(progress);
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

        self.plan_runner = None;
        self.stage = ExecutorStage::Pretensing;
    }

    fn transition_to_fall(&mut self) {
        self.log_event(ExecutionEvent::StageTransition {
            iteration: self.current_iteration,
            from: "PRETENSE".to_string(),
            to: "FALL".to_string(),
        });

        let mass_scale = self.physics.mass_scale();
        let rigidity_scale = self.physics.rigidity_scale();

        use crate::fabric::physics::presets::BASE_PHYSICS;
        self.physics = BASE_PHYSICS;

        if let Some(surface) = self.stored_surface_character {
            self.physics.surface_character = surface;
        }

        use crate::TweakFeature::*;
        self.physics.accept_tweak(MassScale.parameter(mass_scale));
        self.physics.accept_tweak(RigidityScale.parameter(rigidity_scale));

        self.log_event(ExecutionEvent::PhysicsChanged {
            iteration: self.current_iteration,
            description: "FALLING".to_string(),
        });

        self.fabric.progress.start(self.plan.fall_phase.seconds);

        self.pretenser = None;
        self.stage = ExecutorStage::Falling;
    }

    fn transition_to_settle(&mut self) {
        self.log_event(ExecutionEvent::StageTransition {
            iteration: self.current_iteration,
            from: "FALL".to_string(),
            to: "SETTLE".to_string(),
        });

        self.physics.enable_settling();

        self.log_event(ExecutionEvent::PhysicsChanged {
            iteration: self.current_iteration,
            description: "SETTLING".to_string(),
        });

        self.fabric.progress.start(self.plan.settle_phase.seconds);
        self.stage = ExecutorStage::Settling;
    }

    fn complete(&mut self) {
        self.log_event(ExecutionEvent::Completed {
            iteration: self.current_iteration,
        });

        self.fabric.zero_velocities();
        self.settler = None;
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
}
