use crate::fabric::physics::Physics;
use crate::fabric::Fabric;
use crate::messages::PhysicsTesterAction;

pub struct PhysicsTester {
    fabric: Fabric,
    physics: Physics,
}

impl PhysicsTester {
    pub fn new(fabric: &Fabric, physics: Physics) -> Self {
        Self {
            fabric: fabric.clone(),
            physics,
        }
    }

    pub fn iterate(&mut self) {
        self.fabric.iterate(&self.physics);
    }

    pub fn action(&mut self, action: PhysicsTesterAction) {
        match action {
            PhysicsTesterAction::SetPhysicalParameter(parameter) => {
                self.physics.accept(parameter);
            }
        }
    }

    pub fn fabric(&self) -> &Fabric {
        &self.fabric
    }

}

