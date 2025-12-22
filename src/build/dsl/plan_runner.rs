use crate::build::dsl::build_phase::BuildPhase;
use crate::build::dsl::plan_context::PlanContext;
use crate::build::dsl::plan_runner::Stage::*;
use crate::build::dsl::pretense_phase::PretensePhase;
use crate::build::dsl::shape_phase::{ShapeCommand, ShapePhase};
use crate::build::dsl::FabricPlan;
use crate::crucible_context::CrucibleContext;
use crate::fabric::physics::presets::CONSTRUCTION;
use crate::fabric::physics::Physics;
use crate::units::{Meters, IMMEDIATE, MOMENT};
use crate::{LabEvent, StateChange};
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
        Self {
            physics: CONSTRUCTION,
            shape_phase,
            build_phase,
            pretense_phase,
            scale: dimensions.scale,
            stage: Initialize,
            disabled: None,
        }
    }

    pub fn copy_physics_into(&self, context: &mut CrucibleContext) {
        *context.physics = self.physics.clone();
    }

    /// Simplified version for use with PlanContext (no events)
    pub fn check_and_advance_stage_simple(&mut self, context: &mut PlanContext) -> bool {
        if !context.fabric.progress.is_busy() && self.disabled.is_none() {
            let (next_stage, seconds) = match self.stage {
                Initialize => {
                    self.build_phase.init(context.fabric);
                    (BuildApproach, MOMENT)
                }
                BuildStep => {
                    if self.build_phase.is_building() {
                        self.build_phase.build_step(context.fabric);
                        (BuildApproach, MOMENT)
                    } else if self.shape_phase.needs_shaping() {
                        self.shape_phase.marks = self.build_phase.marks.split_off(0);
                        (Shaping, IMMEDIATE)
                    } else {
                        // Build complete - transition immediately so executor can start PRETENSE phase
                        (Completed, IMMEDIATE)
                    }
                }
                BuildApproach => (BuildCalm, MOMENT),
                BuildCalm => (BuildStep, IMMEDIATE),
                Shaping => match self.shape_phase.shaping_step(context.fabric) {
                    ShapeCommand::Noop => (Shaping, IMMEDIATE),
                    ShapeCommand::StartProgress(seconds) => (Shaping, seconds),
                    ShapeCommand::Rigidity(_percent) => (Shaping, IMMEDIATE),
                    ShapeCommand::Terminate => (Completed, IMMEDIATE),
                },
                Completed => (Completed, IMMEDIATE),
            };

            let stage_changed = self.stage != next_stage;
            context.fabric.progress.start(seconds);
            self.stage = next_stage;
            stage_changed
        } else {
            false
        }
    }

    /// Check if progress has completed and advance to the next stage if needed.
    /// This should be called AFTER running one fabric iteration.
    /// Returns true if a stage transition occurred.
    pub fn check_and_advance_stage(&mut self, context: &mut CrucibleContext) -> bool {
        if !context.fabric.progress.is_busy() && self.disabled.is_none() {
            let (next_stage, seconds) = match self.stage {
                Initialize => {
                    self.build_phase.init(context.fabric);
                    (BuildApproach, MOMENT)
                }
                BuildStep => {
                    if self.build_phase.is_building() {
                        self.build_phase.build_step(context.fabric);
                        (BuildApproach, MOMENT)
                    } else if self.shape_phase.needs_shaping() {
                        self.shape_phase.marks = self.build_phase.marks.split_off(0);
                        context.send_event(LabEvent::UpdateState(StateChange::SetStageLabel(
                            "Shaping".to_string(),
                        )));
                        (Shaping, IMMEDIATE)
                    } else {
                        (Completed, IMMEDIATE)
                    }
                }
                BuildApproach => (BuildCalm, MOMENT),
                BuildCalm => (BuildStep, IMMEDIATE),
                Shaping => match self.shape_phase.shaping_step(context.fabric) {
                    ShapeCommand::Noop => (Shaping, IMMEDIATE),
                    ShapeCommand::StartProgress(seconds) => (Shaping, seconds),
                    ShapeCommand::Rigidity(_percent) => (Shaping, IMMEDIATE),
                    ShapeCommand::Terminate => (Completed, IMMEDIATE),
                },
                Completed => (Completed, IMMEDIATE),
            };

            let stage_changed = self.stage != next_stage;
            context.fabric.progress.start(seconds);
            self.stage = next_stage;
            stage_changed
        } else {
            false
        }
    }

    pub fn iterate(&mut self, context: &mut CrucibleContext) {
        // Iterate frame by frame, checking progress after each iteration
        // Stage logic executes at exact fabric time, outer loop adjusts dynamically to maintain target time scale
        static TOTAL_ITERATIONS: AtomicUsize = AtomicUsize::new(0);
        for _ in 0..1000 {
            // Nominal value, outer loop adjusts dynamically
            // DETAILED LOGGING FOR FIRST 100 AND AROUND 12000
            let iter_count = TOTAL_ITERATIONS.fetch_add(1, Ordering::Relaxed);
            let should_log = iter_count <= 100 || (iter_count >= 11900 && iter_count <= 12100);
            if should_log {
                let (min_y, max_y) = context.fabric.altitude_range();
                let height = max_y - min_y;
                let radius = context.fabric.bounding_radius();
                let progress_busy = context.fabric.progress.is_busy();

                eprintln!(
                    "[UI-{:05}] joints:{:3} height:{:8.3} radius:{:8.5} busy:{} stage:{:?}",
                    iter_count,
                    context.fabric.joints.len(),
                    height,
                    radius,
                    progress_busy,
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

#[cfg(test)]
#[path = "plan_runner_test.rs"]
mod plan_runner_test;
