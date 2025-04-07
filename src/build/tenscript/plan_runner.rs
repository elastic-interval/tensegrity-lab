use crate::build::tenscript::{FabricPlan, TenscriptError};
use crate::build::tenscript::brick_library::BrickLibrary;
use crate::build::tenscript::build_phase::BuildPhase;
use crate::build::tenscript::plan_runner::Stage::*;
use crate::build::tenscript::pretense_phase::PretensePhase;
use crate::build::tenscript::shape_phase::{ShapeCommand, ShapePhase};
use crate::fabric::Fabric;
use crate::fabric::physics::Physics;
use crate::fabric::physics::presets::LIQUID;

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
    pub fabric: Fabric,
    stage: Stage,
    build_phase: BuildPhase,
    shape_phase: ShapePhase,
    pretense_phase: PretensePhase,
    pub(crate) physics: Physics,
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
            fabric: Fabric::default(),
            shape_phase,
            build_phase,
            pretense_phase,
            scale,
            stage: Initialize,
            physics: LIQUID,
            disabled: None,
        }
    }

    pub fn iterate(
        &mut self,
        brick_library: &BrickLibrary,
    ) -> Result<(), TenscriptError> {
        self.fabric.iterate(&self.physics);
        if self.fabric.progress.is_busy() || self.disabled.is_some() {
            return Ok(());
        }
        let (next_stage, countdown) = match self.stage {
            Initialize => {
                self.build_phase.init(&mut self.fabric, brick_library)?;
                self.fabric.scale = self.get_scale();
                (GrowApproach, 500)
            }
            GrowStep => {
                if self.build_phase.is_growing() {
                    self.build_phase.growth_step(&mut self.fabric, brick_library)?;
                    (GrowApproach, 500)
                } else if self.shape_phase.needs_shaping() {
                    self.shape_phase.marks = self.build_phase.marks.split_off(0);
                    (Shaping, 0)
                } else {
                    (Completed, 0)
                }
            }
            GrowApproach => (GrowCalm, 500),
            GrowCalm => (GrowStep, 0),
            Shaping => match self.shape_phase.shaping_step(&mut self.fabric, brick_library)? {
                ShapeCommand::Noop => (Shaping, 0),
                ShapeCommand::StartCountdown(countdown) => (Shaping, countdown),
                ShapeCommand::Stiffness(percent) => {
                    self.physics.stiffness *= percent/100.0;
                    (Shaping, 0)
                }
                ShapeCommand::Viscosity(percent) => {
                    self.physics.viscosity *= percent/100.0;
                    (Shaping, 0)
                }
                ShapeCommand::Drag(percent) => {
                    self.physics.drag *= percent/100.0;
                    (Shaping, 0)
                }
                ShapeCommand::Terminate => (Completed, 0),
            },
            Completed => (Completed, 0),
        };
        self.fabric.progress.start(countdown);
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
