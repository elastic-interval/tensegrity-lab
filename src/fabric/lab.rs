use crate::build::tenscript::pretense_phase::PretensePhase;
use crate::build::tenscript::pretenser::Pretenser;
use crate::crucible::LabAction;
use crate::fabric::lab::Stage::*;
use crate::fabric::physics::Physics;
use crate::fabric::Fabric;

#[derive(Clone, PartialEq)]
enum Stage {
    Start,
    Standing,
    MuscleCycle(f32),
}

pub struct Lab {
    stage: Stage,
    physics: Physics,
    pretense_phase: PretensePhase,
}

impl Lab {
    pub fn new(
        Pretenser {
            pretense_phase,
            physics,
            ..
        }: Pretenser,
    ) -> Self {
        Self {
            stage: Start,
            physics,
            pretense_phase,
        }
    }

    pub fn iterate(&mut self, fabric: &mut Fabric) {
        self.stage = match self.stage {
            Start =>
                if let Some(movement) = &self.pretense_phase.muscle_movement {
                    MuscleCycle(1.0 / movement.countdown as f32)
                } else {
                    Standing
                },
            Standing => {
                fabric.iterate(&self.physics);
                Standing
            }
            MuscleCycle(increment) => {
                fabric.iterate(&self.physics);
                fabric.muscle_nuance += increment;
                if fabric.muscle_nuance < 0.0 {
                    fabric.muscle_nuance = 0.0;
                    MuscleCycle(-increment)
                } else if fabric.muscle_nuance > 1.0 {
                    fabric.muscle_nuance = 1.0;
                    MuscleCycle(-increment)
                } else {
                    MuscleCycle(increment)
                }
            }
        };
    }

    pub fn action(&mut self, action: LabAction, fabric: &mut Fabric) {
        match action {
            LabAction::GravityChanged(gravity) => self.physics.gravity = gravity,
            LabAction::MuscleChanged(nuance) => {
                fabric.muscle_nuance = nuance;
            }
            LabAction::MuscleTestToggle => match self.stage {
                Standing => {
                    if let Some(movement) = &self.pretense_phase.muscle_movement {
                        self.stage = MuscleCycle(1.0 / movement.countdown as f32)
                    }
                }
                MuscleCycle(_) => {
                    fabric.muscle_nuance = 0.5;
                    self.stage = Standing
                }
                _ => {}
            },
        }
    }
}
