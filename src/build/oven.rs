use crate::build::tenscript::brick::{Baked, Prototype};
use crate::fabric::Fabric;
use crate::fabric::physics::presets::PROTOTYPE_FORMATION;

pub struct Oven {
    prototype_fabric: Fabric,
}

impl Oven {
    pub fn new(prototype: Prototype) -> Self {
        let prototype_fabric = Fabric::from(prototype);
        Self { prototype_fabric }
    }

    pub fn iterate(&mut self, fabric: &mut Fabric) -> Option<Baked> {
        let mut speed_squared = 1.0;
        for _ in 0..60 {
            speed_squared = fabric.iterate(&PROTOTYPE_FORMATION);
        }
        let age = fabric.age;
        if age % 1000 == 0 {
            log::info!("Fabric settling age {age} at speed squared {speed_squared}");
        }
        if age > 1000 && speed_squared < 1e-12 {
            log::info!("Fabric settled in iteration {age} at speed squared {speed_squared}");
            match Baked::try_from(fabric.clone()) {
                Ok(baked) => {
                    return Some(baked);
                }
                Err(problem) => {
                     panic!("Cannot create brick: {problem}");
                }
            }
        }
        None
    }

    pub fn prototype_fabric(&self) -> Fabric {
        self.prototype_fabric.clone()
    }
}
