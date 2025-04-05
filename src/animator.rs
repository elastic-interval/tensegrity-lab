use crate::fabric::physics::Physics;
use crate::fabric::Fabric;

pub struct Animator {
    pub physics: Physics,
}

impl Animator {
    pub fn new(physics: Physics) -> Self {
        Self { physics }
    }

    pub fn iterate(&mut self, fabric: &mut Fabric) {
        fabric.iterate(&self.physics);
        fabric.muscle_advance()
    }
}
