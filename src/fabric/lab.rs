use crate::build::tenscript::final_phase::FinalPhase;
use crate::build::tenscript::pretenser::Pretenser;
use crate::crucible::LabAction;
use crate::fabric::Fabric;
use crate::fabric::lab::Stage::{*};
use crate::fabric::physics::Physics;

#[derive(Clone, PartialEq)]
enum Stage {
    Start,
    Standing,
    MuscleCycle(f32),
}

pub struct Lab {
    stage: Stage,
    physics: Physics,
    final_phase: FinalPhase,
}

impl Lab {
    pub fn new(Pretenser { final_phase, physics, .. }: Pretenser) -> Self {
        Self {
            stage: Start,
            physics,
            final_phase,
        }
    }

    pub fn iterate(&mut self, fabric: &mut Fabric) {
        self.stage = match self.stage {
            Start => Standing,
            Standing => {
                fabric.iterate(&self.physics);
                Standing
            }
            MuscleCycle(increment) => {
                fabric.iterate(&self.physics);
                fabric.muscle_rotation += increment;
                if fabric.muscle_rotation > 1.0 {
                    fabric.muscle_rotation -= 1.0;
                }
                MuscleCycle(increment)
            }
        };
    }

    pub fn action(&mut self, action: LabAction, fabric: &mut Fabric) {
        match action {
            LabAction::GravityChanged(gravity) => {
                self.physics.gravity = gravity
            }
            LabAction::MuscleChanged(rotation) => {
                fabric.muscle_rotation = rotation;
            }
            LabAction::MuscleTest => {
                match self.stage {
                    Standing => {
                        if let Some(movement) = &self.final_phase.muscle_movement {
                            self.stage = MuscleCycle(1.0 / movement.countdown as f32)
                        }
                    }
                    MuscleCycle(_) => {
                        fabric.muscle_rotation = 0.0;
                        self.stage = Standing
                    }
                    _ => {}
                }
            }
        }
    }
}