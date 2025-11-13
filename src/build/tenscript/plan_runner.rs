use crate::build::tenscript::build_phase::BuildPhase;
use crate::build::tenscript::plan_context::PlanContext;
use crate::build::tenscript::plan_runner::Stage::*;
use crate::build::tenscript::pretense_phase::PretensePhase;
use crate::build::tenscript::shape_phase::{ShapeCommand, ShapePhase};
use crate::build::tenscript::{FabricPlan, TenscriptError};
use crate::crucible_context::CrucibleContext;
use crate::fabric::physics::presets::{LIQUID, PRETENSING};
use crate::fabric::physics::Physics;
use crate::units::{Seconds, IMMEDIATE, MOMENT};

#[derive(Clone, Debug, Copy, PartialEq)]
enum Stage {
    Initialize,
    GrowStep,
    GrowApproach,
    GrowCalm,
    Shaping,
    Completed,
}

pub struct PlanRunner {
    pub physics: Physics,
    stage: Stage,
    build_phase: BuildPhase,
    shape_phase: ShapePhase,
    pretense_phase: PretensePhase,
    disabled: Option<TenscriptError>,
    scale: f32,
}

impl PlanRunner {
    pub fn new(
        FabricPlan {
            shape_phase,
            build_phase,
            pretense_phase,
            scale,
            ..
        }: FabricPlan,
    ) -> Self {
        Self {
            physics: LIQUID,
            shape_phase,
            build_phase,
            pretense_phase,
            scale,
            stage: Initialize,
            disabled: None,
        }
    }

    pub fn copy_physics_into(&self, context: &mut CrucibleContext) {
        *context.physics = self.physics.clone();
    }

    pub fn iterate(&mut self, context: &mut CrucibleContext) -> Result<(), TenscriptError> {
        for _ in context.physics.iterations() {
            context.fabric.iterate(context.physics);
        }

        if context.fabric.progress.is_busy() || self.disabled.is_some() {
            return Ok(());
        }
        let (next_stage, seconds) = match self.stage {
            Initialize => {
                self.build_phase
                    .init(context.fabric, context.brick_library)?;
                context.fabric.scale = self.get_scale();

                (GrowApproach, MOMENT)
            }
            GrowStep => {
                if self.build_phase.is_growing() {
                    self.build_phase
                        .growth_step(context.fabric, context.brick_library)?;

                    (GrowApproach, MOMENT)
                } else if self.shape_phase.needs_shaping() {
                    self.shape_phase.marks = self.build_phase.marks.split_off(0);
                    (Shaping, IMMEDIATE)
                } else {
                    (Completed, IMMEDIATE)
                }
            }
            GrowApproach => (GrowCalm, MOMENT),
            GrowCalm => (GrowStep, IMMEDIATE),
            Shaping => match self
                .shape_phase
                .shaping_step(context.fabric, context.brick_library)?
            {
                ShapeCommand::Noop => (Shaping, IMMEDIATE),
                ShapeCommand::StartProgress(seconds) => (Shaping, seconds),
                ShapeCommand::Stiffness(percent) => {
                    self.physics.stiffness_factor *= percent / 100.0;
                    // Update physics when stiffness changes
                    *context.physics = self.physics.clone();

                    (Shaping, IMMEDIATE)
                }
                ShapeCommand::Viscosity(percent) => {
                    self.physics.viscosity *= percent / 100.0;
                    // Update physics when viscosity changes
                    *context.physics = self.physics.clone();

                    (Shaping, IMMEDIATE)
                }
                ShapeCommand::Drag(percent) => {
                    self.physics.drag *= percent / 100.0;
                    // Update physics when drag changes
                    *context.physics = self.physics.clone();

                    (Shaping, IMMEDIATE)
                }
                ShapeCommand::Terminate => (Completed, IMMEDIATE)
            },
            Completed => (Completed, IMMEDIATE),
        };
        context.fabric.progress.start(seconds);
        self.stage = next_stage;

        Ok(())
    }

    pub fn disable(&mut self, error: TenscriptError) {
        self.disabled = Some(error);
    }

    pub fn is_done(&self) -> bool {
        self.stage == Completed
    }

    pub fn get_scale(&self) -> f32 {
        self.scale
    }

    pub fn pretense_phase(&self) -> PretensePhase {
        self.pretense_phase.clone()
    }

    /// Run the plan to completion headlessly (without UI)
    /// This builds the structure, applies pretensing, and settles it with gravity
    /// Returns after approximately `settle_seconds` of fabric time
    pub fn run_headless(
        plan: FabricPlan,
        context: &mut PlanContext,
        settle_seconds: f32,
    ) -> Result<(), TenscriptError> {
        let mut runner = Self::new(plan);
        
        // Set scale
        context.fabric.scale = runner.get_scale();
        
        // Phase 1: Build with LIQUID physics
        *context.physics = LIQUID;
        runner.build_phase.init(context.fabric, context.brick_library)?;
        
        while runner.build_phase.is_growing() {
            runner.build_phase.growth_step(context.fabric, context.brick_library)?;
        }
        
        // Run LIQUID physics for 30 seconds to let structure form
        // TICK_MICROSECONDS = 250, so 1 second = 4000 iterations
        for _ in 0..(30.0 * 4000.0) as usize {
            context.fabric.iterate(context.physics);
        }
        
        // Phase 1.5: Shaping (if needed)
        if runner.shape_phase.needs_shaping() {
            runner.shape_phase.marks = runner.build_phase.marks.split_off(0);
            
            // Execute all shaping operations
            loop {
                use crate::build::tenscript::shape_phase::ShapeCommand;
                
                match runner.shape_phase.shaping_step(context.fabric, context.brick_library)? {
                    ShapeCommand::Noop => {},
                    ShapeCommand::StartProgress(seconds) => {
                        // Start the progress countdown
                        context.fabric.progress.start(seconds);
                        // Run physics until progress is complete
                        while context.fabric.progress.is_busy() {
                            context.fabric.iterate(context.physics);
                        }
                    },
                    ShapeCommand::Stiffness(percent) => {
                        runner.physics.stiffness_factor *= percent / 100.0;
                        *context.physics = runner.physics.clone();
                    },
                    ShapeCommand::Viscosity(percent) => {
                        runner.physics.viscosity *= percent / 100.0;
                        *context.physics = runner.physics.clone();
                    },
                    ShapeCommand::Drag(percent) => {
                        runner.physics.drag *= percent / 100.0;
                        *context.physics = runner.physics.clone();
                    },
                    ShapeCommand::Terminate => break,
                }
            }
        }
        
        // Phase 2: Pretensing (complete pretense phase)
        *context.physics = PRETENSING;
        
        // Step 1: Remove faces and add triangles
        let face_ids: Vec<_> = context.fabric.faces.keys().copied().collect();
        for face_id in face_ids {
            let face = context.fabric.face(face_id);
            if !face.has_prism {
                context.fabric.add_face_triangle(face_id);
            }
            context.fabric.remove_face(face_id);
        }
        
        // Step 2: Slacken and centralize
        context.fabric.slacken();
        let altitude = runner.pretense_phase.altitude.unwrap_or(0.0) / context.fabric.scale;
        context.fabric.centralize(Some(altitude));
        
        // Step 3: Set pretenst and run pretensing
        let pretenst_factor = runner.pretense_phase.pretenst.unwrap_or(PRETENSING.pretenst);
        let seconds_to_pretense = runner.pretense_phase.seconds.unwrap_or(Seconds(15.0));
        context.fabric.set_pretenst(pretenst_factor, seconds_to_pretense);
        
        // Run pretensing until progress is complete (like the UI does)
        while context.fabric.progress.is_busy() {
            context.fabric.iterate(context.physics);
        }
        
        // Step 4: Create muscles if needed
        if let Some(muscle_movement) = &runner.pretense_phase.muscle_movement {
            context.fabric.create_muscles(muscle_movement.contraction);
        }
        
        // Phase 3: Settle with viewing physics (includes gravity)
        *context.physics = runner.pretense_phase.viewing_physics();
        
        // Settle for the requested time
        // The structure needs time to extend upward after pretensing
        // In the UI, this continues indefinitely, but we settle for a fixed time
        let settle_iterations = (settle_seconds * 4000.0) as usize;
        for _ in 0..settle_iterations {
            context.fabric.iterate(context.physics);
        }
        
        Ok(())
    }
}

#[cfg(test)]
#[path = "plan_runner_test.rs"]
mod plan_runner_test;
