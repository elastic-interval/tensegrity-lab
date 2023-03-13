use crate::build::tenscript::final_phase::FinalPhase;
use crate::build::tenscript::pretenser::Stage::{*};
use crate::fabric::Fabric;
use crate::fabric::physics::{Physics, SurfaceCharacter};
use crate::fabric::physics::presets::AIR_GRAVITY;

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
    speed_threshold: f32,
    pub final_phase: FinalPhase,
    pub physics: Physics,
}

const DEFAULT_PRETENSE_FACTOR: f32 = 1.03;

impl Pretenser {
    pub fn new(final_phase: FinalPhase) -> Self {
        let surface_character = final_phase.surface_character;
        let gravity = if surface_character == SurfaceCharacter::Absent { 0.0 } else { AIR_GRAVITY.gravity };
        Self {
            stage: Start,
            final_phase,
            pretensing_countdown: 20000,
            speed_threshold: 1e-6,
            physics: Physics { surface_character, gravity, ..AIR_GRAVITY },
        }
    }

    pub fn iterate(&mut self, fabric: &mut Fabric) {
        self.stage = match self.stage {
            Start => Slacken,
            Slacken => {
                let factor = self.final_phase.pretense_factor.unwrap_or(DEFAULT_PRETENSE_FACTOR);
                fabric.prepare_for_pretensing(factor);
                fabric.progress.start(self.pretensing_countdown);
                self.final_phase.create_hangers(fabric);
                Pretensing
            }
            Pretensing => {
                fabric.iterate(&self.physics);
                if fabric.progress.is_busy() {
                    Pretensing
                } else {
                    self.final_phase.check_muscles(fabric);
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
