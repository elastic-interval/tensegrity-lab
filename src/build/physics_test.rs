use crate::fabric::physics::Physics;
use crate::fabric::Fabric;
use crate::{PhysicsFeature, Radio, StateChange, TesterAction};
use std::rc::Rc;

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

    pub fn action(&mut self, action: TesterAction) {
        use TesterAction::*;
        match action {
            SetPhysicalParameter(parameter) => {
                self.physics.accept(parameter);
                match parameter.feature {
                    PhysicsFeature::Pretenst => {
                        self.fabric.set_pretenst(parameter.value, 100);
                    }
                    PhysicsFeature::StrainLimit => {
                        let strain_limit = self.physics.strain_limit;
                        StateChange::SetAppearanceFunction(Rc::new(move |interval| {
                            if interval.strain > strain_limit {
                                let role = interval.material.properties().role;
                                Some(role.appearance().highlighted())
                            } else {
                                None
                            }
                        }))
                        .send(&self.radio.clone());
                    }
                    _ => {}
                }
            }
            DumpPhysics => {
                println!("{:?}", self.physics);
            }
            _ => {}
        }
    }
}
