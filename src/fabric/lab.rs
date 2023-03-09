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
}

impl Lab {
    pub(crate) fn new(physics: Physics) -> Self {
        Self { stage: Start, physics }
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
            LabAction::GravityChanged(gravity) => {
                self.physics.gravity = gravity
            }
            LabAction::MuscleChanged(nuance) => {
                fabric.muscle_nuance = nuance;
            }
            LabAction::MuscleTest(increment) => {
                self.stage = MuscleCycle(increment)
            }
        }
    }
}