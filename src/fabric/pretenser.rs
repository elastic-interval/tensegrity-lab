use crate::fabric::Fabric;
use crate::fabric::physics::{Physics, SurfaceCharacter};
use crate::fabric::physics::presets::AIR_GRAVITY;
use crate::fabric::pretenser::Stage::{*};

#[derive(Clone, PartialEq)]
enum Stage {
    Start,
    Slacken,
    Pretensing,
    Settling,
    Pretenst,
}

pub struct Pretenser {
    stage: Stage,
    pretenst_factor: f32,
    pretensing_countdown: usize,
    speed_threshold: f32,
    physics: Physics,
}

impl Pretenser {
    pub fn new(pretenst_factor: f32, surface_character: SurfaceCharacter) -> Self {
        let mut physics = AIR_GRAVITY;
        physics.surface_character = surface_character;
        Self {
            stage: Start,
            pretenst_factor,
            pretensing_countdown: 20000,
            speed_threshold: 1e-6,
            physics,
        }
    }

    pub fn iterate(&mut self, fabric: &mut Fabric) {
        self.stage = match self.stage {
            Start => Slacken,
            Slacken => {
                fabric.prepare_for_pretensing(self.pretenst_factor);
                fabric.progress.start(self.pretensing_countdown);
                Pretensing
            }
            Pretensing => {
                fabric.iterate(&self.physics);
                if fabric.progress.is_busy() {
                    Pretensing
                } else {
                    Settling
                }
            }
            Settling => {
                fabric.iterate(&self.physics);
                let speed2 = fabric.iterate(&self.physics);
                if speed2 > self.speed_threshold * self.speed_threshold {
                    Settling
                } else {
                    Pretenst
                }
            }
            Pretenst => Pretenst
        };
    }

    pub fn is_done(&self) -> bool {
        self.stage == Pretenst
    }
}
