use crate::build::tenscript::brick::{Baked, Prototype};
use crate::fabric::physics::presets::PROTOTYPE_FORMATION;
use crate::fabric::Fabric;

pub struct Oven {
    pub fabric: Fabric,
}

impl Oven {
    pub fn new(prototype: Prototype) -> Self {
        let fabric = Fabric::from(prototype);
        Self { fabric }
    }

    pub fn iterate(&mut self) -> Option<Baked> {
        let mut speed_squared = 1.0;
        for _ in 0..60 {
            speed_squared = self.fabric.iterate(&PROTOTYPE_FORMATION);
        }
        let age = self.fabric.age;
        if age > 20000 && speed_squared < 1e-11 {
            println!("Fabric settled in iteration {age} at speed squared {speed_squared}");
            match Baked::try_from(self.fabric.clone()) {
                Ok(baked) => {
                    self.fabric.check_orphan_joints();
                    println!("Baked it!");
                    return Some(baked);
                }
                Err(problem) => {
                    println!("Cannot create brick: {problem}");
                    std::process::exit(0)
                }
            }
        }
        None
    }

    pub fn prototype_fabric(&self) -> Fabric {
        self.fabric.clone()
    }
}
