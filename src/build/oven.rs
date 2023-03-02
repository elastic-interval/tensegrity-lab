use crate::build::brick::{Baked, Prototype};
use crate::build::tenscript::FaceAlias;
use crate::fabric::Fabric;
use crate::fabric::physics::presets::PROTOTYPE_FORMATION;

pub struct Oven {
    prototype_fabric: Fabric,
    alias: FaceAlias,
}

impl Oven {
    pub fn new(prototype: Prototype) -> Self {
        println!("Settling and capturing prototype number {:?}", prototype.alias);
        let alias = prototype.alias.clone();
        let prototype_fabric = Fabric::from(prototype);
        Self { prototype_fabric, alias }
    }

    pub fn iterate(&mut self, fabric: &mut Fabric) -> Option<Baked> {
        let mut speed_squared = 1.0;
        for _ in 0..60 {
            speed_squared = fabric.iterate(&PROTOTYPE_FORMATION);
        }
        let age = fabric.age;
        if age > 1000 && speed_squared < 1e-12 {
            println!("Fabric settled in iteration {age} at speed squared {speed_squared}");
            match Baked::try_from((fabric.clone(), self.alias.clone())) {
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