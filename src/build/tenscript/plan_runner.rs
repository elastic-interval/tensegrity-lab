use crate::build::tenscript::{FabricPlan, shape_phase};
use crate::build::tenscript::build_phase::BuildPhase;
use crate::build::tenscript::plan_runner::Stage::{*};
use crate::build::tenscript::shape_phase::ShapePhase;
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
    stage: Stage,
    build_phase: BuildPhase,
    shape_phase: ShapePhase,
    physics: Physics,
}

impl PlanRunner {
    pub fn new(FabricPlan { shape_phase, build_phase, .. }: FabricPlan) -> Self {
        Self {
            shape_phase,
            build_phase,
            stage: Initialize,
            physics: LIQUID,
        }
    }
}

impl PlanRunner {
    pub fn iterate(&mut self, fabric: &mut Fabric) {
        fabric.iterate(&self.physics);
        if fabric.progress.is_busy() {
            return;
        }
        let (next_stage, countdown) = match self.stage {
            Initialize => {
                self.build_phase.init(fabric);
                (GrowApproach, 200)
            }
            GrowStep => {
                if self.build_phase.is_growing() {
                    self.build_phase.growth_step(fabric);
                    (GrowApproach, 200)
                } else if self.shape_phase.needs_shaping() {
                    self.shape_phase.marks = self.build_phase.marks.split_off(0);
                    (Shaping, 0)
                } else {
                    (Completed, 0)
                }
            }
            GrowApproach =>
                (GrowCalm, 200),
            GrowCalm =>
                (GrowStep, 0),
            Shaping =>
                match self.shape_phase.shaping_step(fabric) {
                    shape_phase::Command::Noop =>
                        (Shaping, 0),
                    shape_phase::Command::StartCountdown(countdown) =>
                        (Shaping, countdown),
                    shape_phase::Command::SetViscosity(viscosity) => {
                        self.physics.viscosity = viscosity;
                        (Shaping, 0)
                    }
                    shape_phase::Command::Terminate =>
                        (Completed, 0)
                }
            Completed =>
                (Completed, 0),
        };
        fabric.progress.start(countdown);
        self.stage = next_stage;
    }

    pub fn is_done(&self) -> bool {
        self.stage == Completed
    }
}
