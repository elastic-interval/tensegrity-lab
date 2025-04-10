use crate::build::tenscript::brick::{Baked, Prototype};
use crate::fabric::physics::presets::PROTOTYPE_FORMATION;
use crate::fabric::Fabric;

const SPEED_LIMIT: f32 = 3e-6;

pub struct Oven {
    pub fabric: Fabric,
}

impl Oven {
    pub fn new(prototype: Prototype) -> Self {
        let fabric = Fabric::from(prototype);
        Self {
            fabric,
        }
    }

    pub fn iterate(&mut self) -> Option<Baked> {
        for _ in 0..60 {
            self.fabric.iterate(&PROTOTYPE_FORMATION);
        }
        let max_velocity = self.fabric.max_velocity();
        let age = self.fabric.age;
        if age.brick_baked() && max_velocity < SPEED_LIMIT {
            println!("Fabric settled in {age} at velocity {max_velocity}");
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
