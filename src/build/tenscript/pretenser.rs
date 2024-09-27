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
    MuscleWait,
    Pretenst,
}

#[derive(Clone)]
pub struct Pretenser {
    stage: Stage,
    pretensing_countdown: usize,
    muscle_wait: usize,
    pub pretense_phase: PretensePhase,
    pub physics: Physics,
}

const DEFAULT_PRETENSE_FACTOR: f32 = 1.03;
const MUSCLE_WAIT: usize = 20000;
const PRETENSING_COUNTDOWN: usize = 30000;

const DEFAULT_ALTITUDE: f32 = 0.0;

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
            pretensing_countdown: PRETENSING_COUNTDOWN,
            muscle_wait: MUSCLE_WAIT,
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
                fabric.prepare_for_pretensing(factor, DEFAULT_ALTITUDE);
                fabric.progress.start(self.pretensing_countdown);
                Pretensing
            }
            Pretensing => {
                fabric.iterate(&self.physics);
                if fabric.progress.is_busy() {
                    Pretensing
                } else {
                    fabric.stay_above = false;
                    if self.pretense_phase.muscle_movement.is_some() {
                        MuscleWait
                    } else {
                        Pretenst
                    }
                }
            }
            MuscleWait => {
                self.muscle_wait -= 1;
                if self.muscle_wait == 0 {
                    let Some(muscle_movement) = &self.pretense_phase.muscle_movement  else {
                        panic!("expected a muscle movement")
                    };
                    fabric.activate_muscles(muscle_movement);
                    fabric.progress.start(MUSCLE_WAIT);
                    Pretenst
                } else {
                    fabric.iterate(&self.physics);
                    MuscleWait
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
