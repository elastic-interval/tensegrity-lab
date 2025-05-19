use crate::build::tenscript::pretense_phase::PretensePhase;
use crate::build::tenscript::pretenser::Stage::*;
use crate::crucible_context::CrucibleContext;
use crate::fabric::physics::presets::AIR_GRAVITY;
use crate::fabric::physics::Physics;

#[derive(Clone, Debug, PartialEq, Copy)]
enum Stage {
    Start,
    Slacken,
    Pretensing,
    CreateMuscles,
    Pretenst,
}

#[derive(Clone)]
pub struct Pretenser {
    pub pretense_phase: PretensePhase,
    pub physics: Physics,
    stage: Stage,
    countdown: usize,
}

impl Pretenser {
    pub fn new(pretense_phase: PretensePhase) -> Self {
        let pretenst = pretense_phase.pretenst.unwrap_or(AIR_GRAVITY.pretenst);
        let surface_character = pretense_phase.surface_character;
        let stiffness = pretense_phase.stiffness.unwrap_or(AIR_GRAVITY.stiffness);
        let physics = Physics {
            pretenst,
            surface_character,
            stiffness,
            ..AIR_GRAVITY
        };
        let countdown = pretense_phase.countdown.unwrap_or(7000);
        Self {
            stage: Start,
            pretense_phase,
            countdown,
            physics,
        }
    }

    pub fn initialize_physics(&self, context: &mut CrucibleContext) {
        *context.physics = self.physics.clone();
    }

    pub fn iterate(&mut self, context: &mut CrucibleContext) {
        // Process the current stage
        self.stage = match self.stage {
            Start => Slacken,
            Slacken => {
                context.fabric.slacken();
                let factor = self
                    .pretense_phase
                    .pretenst
                    .unwrap_or(self.physics.pretenst);
                context.fabric.set_pretenst(factor, self.countdown);

                Pretensing
            }
            Pretensing => {
                for _ in context.physics.iterations() {
                    context.fabric.iterate(context.physics);
                }

                if context.fabric.progress.is_busy() {
                    Pretensing
                } else {
                    if self.pretense_phase.muscle_movement.is_some() {
                        CreateMuscles
                    } else {
                        Pretenst
                    }
                }
            }
            CreateMuscles => {
                if context.fabric.progress.is_busy() {
                    // Perform a single physics iteration
                    context.fabric.iterate(context.physics);

                    CreateMuscles
                } else {
                    let Some(muscle_movement) = &self.pretense_phase.muscle_movement else {
                        panic!("expected a muscle movement")
                    };
                    context.fabric.create_muscles(muscle_movement.contraction);
                    self.physics.cycle_ticks = muscle_movement.countdown as f32;
                    // Update physics when cycle_ticks changes
                    *context.physics = self.physics.clone();
                    context.fabric.progress.start(500);

                    Pretenst
                }
            }
            Pretenst => {
                for _ in context.physics.iterations() {
                    context.fabric.iterate(context.physics);
                }

                Pretenst
            }
        };
    }

    pub fn is_done(&self) -> bool {
        self.stage == Pretenst
    }

    pub fn physics(&self) -> &Physics {
        &self.physics
    }
}
