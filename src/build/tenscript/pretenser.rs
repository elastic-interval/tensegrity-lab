use crate::build::tenscript::pretense_phase::PretensePhase;
use crate::build::tenscript::pretenser::Stage::*;
use crate::crucible::Holder;
use crate::fabric::physics::presets::AIR_GRAVITY;
use crate::fabric::physics::Physics;
use crate::fabric::Fabric;

#[derive(Clone, PartialEq)]
enum Stage {
    Start,
    Slacken,
    Pretensing,
    CreateMuscles,
    Pretenst,
}

#[derive(Clone)]
pub struct Pretenser {
    pub fabric: Fabric,
    pub pretense_phase: PretensePhase,
    pub physics: Physics,
    stage: Stage,
    countdown: usize,
}

impl Pretenser {
    pub fn new(pretense_phase: PretensePhase, fabric: Fabric) -> Self {
        let surface_character = pretense_phase.surface_character;
        let stiffness = pretense_phase.stiffness.unwrap_or(AIR_GRAVITY.stiffness);
        let physics = Physics {
            surface_character,
            stiffness,
            ..AIR_GRAVITY
        };
        let countdown = pretense_phase.countdown.unwrap_or(7000);
        Self {
            fabric,
            stage: Start,
            pretense_phase,
            countdown,
            physics,
        }
    }

    pub fn iterate(&mut self) {
        self.stage = match self.stage {
            Start => Slacken,
            Slacken => {
                self.fabric.slacken();
                let factor = self.pretense_phase.pretenst.unwrap_or(self.physics.pretenst);
                self.fabric.set_pretenst(factor, self.countdown);
                Pretensing
            }
            Pretensing => {
                self.fabric.iterate(&self.physics);
                if self.fabric.progress.is_busy() {
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
                if self.fabric.progress.is_busy() {
                    self.fabric.iterate(&self.physics);
                    CreateMuscles
                } else {
                    let Some(muscle_movement) = &self.pretense_phase.muscle_movement else {
                        panic!("expected a muscle movement")
                    };
                    self.fabric.create_muscles(muscle_movement.contraction);
                    self.physics.cycle_ticks = muscle_movement.countdown as f32;
                    self.fabric.progress.start(500);
                    Pretenst
                }
            }
            Pretenst => {
                self.fabric.iterate(&self.physics);
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

    pub fn holder(&self) -> Holder {
        Holder {
            fabric: self.fabric.clone(),
            physics: self.physics.clone(),
        }
    }
}
