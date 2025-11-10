use crate::build::tenscript::build_phase::BuildPhase;
use crate::build::tenscript::plan_runner::Stage::*;
use crate::build::tenscript::pretense_phase::PretensePhase;
use crate::build::tenscript::shape_phase::{ShapeCommand, ShapePhase};
use crate::build::tenscript::{FabricPlan, TenscriptError};
use crate::crucible_context::CrucibleContext;
use crate::fabric::physics::presets::LIQUID;
use crate::fabric::physics::Physics;
use crate::units::{IMMEDIATE, MOMENT};

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
}
