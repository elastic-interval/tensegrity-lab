use crate::build::dsl::build_phase::BuildPhase;
use crate::build::dsl::plan_context::PlanContext;
use crate::build::dsl::plan_runner::Stage::*;
use crate::build::dsl::pretense_phase::PretensePhase;
use crate::build::dsl::shape_phase::{ShapeCommand, ShapePhase};
use crate::build::dsl::FabricPlan;
use crate::crucible_context::CrucibleContext;
use crate::fabric::physics::presets::CONSTRUCTION;
use crate::fabric::physics::Physics;
use crate::units::{Meters, Seconds, Unit, IMMEDIATE, MOMENT};
use crate::{Age, LabEvent, StateChange};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Debug, Copy, PartialEq)]
pub enum Stage {
    Initialize,
    BuildStep,
    BuildApproach,
    BuildCalm,
    Shaping,
    Completed,
}

pub struct PlanRunner {
    pub physics: Physics,
    pub stage: Stage,
    pub build_phase: BuildPhase,
    shape_phase: ShapePhase,
    pretense_phase: PretensePhase,
    disabled: Option<String>,
    scale: Meters,
    stage_start_age: Option<Age>,
    stage_duration: Seconds,
}

impl PlanRunner {
    pub fn new(
        FabricPlan {
            shape_phase,
            build_phase,
            pretense_phase,
            dimensions,
            ..
        }: FabricPlan,
    ) -> Self {
        let mut physics = CONSTRUCTION;
        // Scale is applied to the seed face and propagated through the face
        // hierarchy, so intervals are shorter by the scale factor. Adjust
        // rigidity to maintain equivalent build dynamics.
        let s = dimensions.scale.f32();
        physics.tweak.rigidity_multiplier *= s.powf(1.5);
        Self {
            physics,
            shape_phase,
            build_phase,
            pretense_phase,
            scale: dimensions.scale,
            stage: Initialize,
            disabled: None,
            stage_start_age: None,
            stage_duration: IMMEDIATE,
        }
    }

    pub fn copy_physics_into(&self, context: &mut CrucibleContext) {
        *context.physics = self.physics.clone();
    }

    /// Check if stage duration elapsed
    fn stage_elapsed(&self, current_age: Age) -> bool {
        self.stage_start_age.map_or(true, |start| {
            let elapsed = current_age.elapsed_since(start);
            elapsed >= self.stage_duration
        })
    }

    /// Headless stage advance (used by `FabricPlanExecutor`, no events emitted).
    pub fn check_and_advance_stage_simple(&mut self, context: &mut PlanContext) -> bool {
        let (stage_changed, _entered_shaping) = self.advance_stage(context.fabric);
        stage_changed
    }

    /// UI-driven stage advance: same logic as the headless version but emits a
    /// `SetStageLabel("Shaping")` event when the build-step transition lands
    /// on `Shaping`. Returns true if a stage transition occurred.
    pub fn check_and_advance_stage(&mut self, context: &mut CrucibleContext) -> bool {
        let (stage_changed, entered_shaping) = self.advance_stage(context.fabric);
        if entered_shaping {
            context.send_event(LabEvent::UpdateState(StateChange::SetStageLabel(
                "Shaping".to_string(),
            )));
        }
        stage_changed
    }

    /// Pure stage transition logic. Returns `(stage_changed, entered_shaping)`
    /// — `entered_shaping` is true only when this call moved from `BuildStep`
    /// to `Shaping`, so the UI caller knows to emit a label event.
    fn advance_stage(&mut self, fabric: &mut crate::fabric::Fabric) -> (bool, bool) {
        if !(self.stage_elapsed(fabric.age) && self.disabled.is_none()) {
            return (false, false);
        }
        let mut entered_shaping = false;
        let (next_stage, seconds) = match self.stage {
            Initialize => {
                self.build_phase.init(fabric);
                (BuildApproach, MOMENT)
            }
            BuildStep => {
                if self.build_phase.is_building() {
                    self.build_phase.build_step(fabric);
                    (BuildApproach, MOMENT)
                } else if self.shape_phase.needs_shaping() {
                    let bindings = self.build_phase.labels.split_off(0);
                    self.shape_phase.install_labels(&bindings);
                    entered_shaping = true;
                    (Shaping, IMMEDIATE)
                } else {
                    (Completed, IMMEDIATE)
                }
            }
            BuildApproach => (BuildCalm, MOMENT),
            BuildCalm => (BuildStep, IMMEDIATE),
            Shaping => match self.shape_phase.shaping_step(fabric) {
                ShapeCommand::Noop => (Shaping, IMMEDIATE),
                ShapeCommand::StartProgress(seconds) => (Shaping, seconds),
                ShapeCommand::Rigidity(_percent) => (Shaping, IMMEDIATE),
                ShapeCommand::Terminate => (Completed, IMMEDIATE),
            },
            Completed => (Completed, IMMEDIATE),
        };

        let stage_changed = self.stage != next_stage;
        self.stage_start_age = Some(fabric.age);
        self.stage_duration = seconds;
        self.stage = next_stage;
        (stage_changed, entered_shaping)
    }

    pub fn iterate(&mut self, context: &mut CrucibleContext) {
        // Iterate frame by frame, checking progress after each iteration
        // Stage logic executes at exact fabric time, outer loop adjusts dynamically to maintain target time scale
        static TOTAL_ITERATIONS: AtomicUsize = AtomicUsize::new(0);
        for _ in 0..1000 {
            let iter_count = TOTAL_ITERATIONS.fetch_add(1, Ordering::Relaxed);
            let should_log = iter_count <= 100 || (iter_count >= 11900 && iter_count <= 12100);
            if should_log {
                let (min_y, max_y) = context.fabric.altitude_range();
                let height = max_y - min_y;
                let radius = context.fabric.bounding_radius();
                let stage_elapsed = self.stage_elapsed(context.fabric.age);

                eprintln!(
                    "[UI-{:05}] joints:{:3} height:{:8.3} radius:{:8.5} elapsed:{} stage:{:?}",
                    iter_count,
                    context.fabric.joints.len(),
                    height,
                    radius,
                    stage_elapsed,
                    self.stage
                );
            }

            context.fabric.iterate(context.physics);

            // Check if we need to advance to the next stage
            self.check_and_advance_stage(context);
        }
    }

    pub fn disable(&mut self, error: String) {
        self.disabled = Some(error);
    }

    pub fn is_done(&self) -> bool {
        self.stage == Completed
    }

    pub fn get_scale(&self) -> Meters {
        self.scale
    }

    pub fn pretense_phase(&self) -> PretensePhase {
        self.pretense_phase.clone()
    }
}

