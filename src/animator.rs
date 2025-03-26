use crate::fabric::physics::Physics;
use crate::fabric::Fabric;

pub struct Animator {
    pub physics: Physics,
    pub forward: bool,
}

impl Animator {
    pub fn new(physics: Physics) -> Self {
        Self {
            physics,
            forward: true,
        }
    }

    pub fn iterate(&mut self, fabric: &mut Fabric) {
        fabric.iterate(&self.physics);
        let increment = if self.forward {
            fabric.muscle_nuance_increment
        } else {
            -fabric.muscle_nuance_increment
        };
        fabric.muscle_nuance += increment;
        if fabric.muscle_nuance < 0.0 {
            fabric.muscle_nuance = 0.0;
            self.forward = true;
        } else if fabric.muscle_nuance > 1.0 {
            fabric.muscle_nuance = 1.0;
            self.forward = false;
        }
    }
}
