use crate::build::tenscript::{FabricPlan, TenscriptError};
use crate::build::tenscript::build_phase::BuildPhase;
use crate::build::tenscript::plan_runner::Stage::{*};
use crate::build::tenscript::shape_phase::{ShapeCommand, ShapePhase};
use crate::fabric::brick::BrickLibrary;
use crate::fabric::Fabric;
use crate::fabric::physics::{Physics, SurfaceCharacter};
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
    stage: Stage,
    build_phase: BuildPhase,
    shape_phase: ShapePhase,
    physics: Physics,
    surface_character: SurfaceCharacter,
}

impl PlanRunner {
    pub fn new(FabricPlan { shape_phase, build_phase, .. }: FabricPlan) -> Self {
        Self {
            shape_phase,
            build_phase,
            stage: Initialize,
            physics: LIQUID,
            surface_character: SurfaceCharacter::Frozen
        }
    }

    pub fn iterate(&mut self, fabric: &mut Fabric, brick_library: &dyn BrickLibrary) -> Result<(), TenscriptError> {
        fabric.iterate(&self.physics);
        if fabric.progress.is_busy() {
            return Ok(());
        }
        let (next_stage, countdown) = match self.stage {
            Initialize => {
                self.build_phase.init(fabric, brick_library)?;
                (GrowApproach, 500)
            }
            GrowStep => {
                if self.build_phase.is_growing() {
                    self.build_phase.growth_step(fabric, brick_library)?;
                    (GrowApproach, 500)
                } else if self.shape_phase.needs_shaping() {
                    self.shape_phase.marks = self.build_phase.marks.split_off(0);
                    (Shaping, 0)
                } else {
                    (Completed, 0)
                }
            }
            GrowApproach =>
                (GrowCalm, 500),
            GrowCalm =>
                (GrowStep, 0),
            Shaping =>
                match self.shape_phase.shaping_step(fabric) {
                    ShapeCommand::Noop =>
                        (Shaping, 0),
                    ShapeCommand::StartCountdown(countdown) =>
                        (Shaping, countdown),
                    ShapeCommand::SetViscosity(viscosity) => {
                        self.physics.viscosity = viscosity;
                        (Shaping, 0)
                    }
                    ShapeCommand::Bouncy => {
                        self.surface_character = SurfaceCharacter::Bouncy;
                        (Shaping, 0)
                    }
                    ShapeCommand::Terminate =>
                        (Completed, 0)
                }
            Completed =>
                (Completed, 0),
        };
        fabric.progress.start(countdown);
        self.stage = next_stage;
        Ok(())
    }

    pub fn is_done(&self) -> bool {
        self.stage == Completed
    }

    pub fn surface_character(&self) -> SurfaceCharacter {
        self.surface_character
    }
}
