use crate::build::tenscript::pretense_phase::PretensePhase;
use crate::build::tenscript::pretenser::Stage::*;
use crate::fabric::physics::presets::AIR_GRAVITY;
use crate::fabric::physics::{Physics, SurfaceCharacter};
use crate::fabric::Fabric;

#[derive(Clone, PartialEq)]
enum Stage {
    Start,
    Slacken,
    Pretensing,
    Pretenst,
}

#[derive(Clone)]
pub struct Pretenser {
    stage: Stage,
    pretensing_countdown: usize,
    pub pretense_phase: PretensePhase,
    pub physics: Physics,
}

const DEFAULT_PRETENSE_FACTOR: f32 = 1.03;

impl Pretenser {
    pub fn new(pretense_phase: PretensePhase) -> Self {
        let surface_character = pretense_phase.surface_character;
        let gravity = if surface_character == SurfaceCharacter::Absent {
            0.0
        } else {
            AIR_GRAVITY.gravity
        };
        Self {
            stage: Start,
            pretense_phase,
            pretensing_countdown: 20000,
            physics: Physics {
                surface_character,
                gravity,
                ..AIR_GRAVITY
            },
        }
    }

    pub fn iterate(&mut self, fabric: &mut Fabric) {
        self.stage = match self.stage {
            Start => Slacken,
            Slacken => {
                let factor = self
                    .pretense_phase
                    .pretense_factor
                    .unwrap_or(DEFAULT_PRETENSE_FACTOR);
                fabric.prepare_for_pretensing(factor);
                fabric.progress.start(self.pretensing_countdown);
                Pretensing
            }
            Pretensing => {
                fabric.iterate(&self.physics);
                if fabric.progress.is_busy() {
                    Pretensing
                } else {
                    if let Some(muscle_movement) = &self.pretense_phase.muscle_movement {
                        fabric.activate_muscles(muscle_movement);
                    };
                    fabric.keep_above = false;
                    Pretenst
                }
            }
            Pretenst => {
                fabric.iterate(&self.physics);
                Pretenst
            }
        };
    }

    pub fn is_done(&self) -> bool {
        self.stage == Pretenst
    }

    pub fn physics(&self) -> Physics {
        self.physics.clone()
    }
}
