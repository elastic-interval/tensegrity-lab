use crate::crucible::LabAction;
use crate::fabric::Fabric;
use crate::fabric::lab::Stage::{*};
use crate::fabric::physics::Physics;
use crate::fabric::physics::presets::AIR_GRAVITY;

#[derive(Clone, PartialEq)]
enum Stage {
    Start,
    Running,
}

pub struct Lab {
    stage: Stage,
    physics: Physics,
}

impl Default for Lab {
    fn default() -> Self {
        Self {
            stage: Start,
            physics: AIR_GRAVITY,
        }
    }
}

impl Lab {
    pub fn iterate(&mut self, fabric: &mut Fabric) {
        self.stage = match self.stage {
            Start => Running,
            Running => {
                fabric.iterate(&self.physics);
                Running
            }
        };
    }

    pub fn action(&mut self,action: LabAction) {
        match action {
            LabAction::GravityChanged(gravity) => {
                self.physics.gravity = gravity
            }
        }
    }
}