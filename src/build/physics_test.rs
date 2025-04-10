use std::rc::Rc;
use crate::fabric::physics::Physics;
use crate::fabric::Fabric;
use crate::messages::{PhysicsFeature, PhysicsTesterAction, Radio, StateChange};

const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];

pub struct PhysicsTester {
    pub fabric: Fabric,
    pub physics: Physics,
    radio: Radio,
}

impl PhysicsTester {
    pub fn new(fabric: &Fabric, physics: Physics, radio: Radio) -> Self {
        let mut fabric = fabric.clone();
        physics.broadcast(&radio);
        fabric.activate_muscles(true);
        Self {
            fabric,
            physics,
            radio,
        }
    }

    pub fn iterate(&mut self) {
        self.fabric.iterate(&self.physics);
    }

    pub fn action(&mut self, action: PhysicsTesterAction) {
        match action {
            PhysicsTesterAction::SetPhysicalParameter(parameter) => {
                self.physics.accept(parameter);
                match parameter.feature {
                    PhysicsFeature::Pretenst => {
                        self.fabric.set_pretenst(parameter.value, 100);
                    }
                    PhysicsFeature::StrainLimit => {
                        let strain_limit = self.physics.strain_limit;
                        StateChange::SetColorFunction(Rc::new(move |interval| {
                            if interval.strain > strain_limit {
                                Some(GREEN)
                            } else {
                                None
                            }
                        })).send(&self.radio.clone());
                    }
                    _ => {}
                }
            }
            PhysicsTesterAction::DumpPhysics => {
                println!("{:?}", self.physics);
            }
        }
    }
}
