use crate::build::tenscript::brick::{Baked, Prototype};
use crate::fabric::material::Material;
use crate::fabric::physics::presets::PROTOTYPE_FORMATION;
use crate::fabric::Fabric;
use crate::messages::{Radio, StateChange};
use std::rc::Rc;

const SPEED_LIMIT: f32 = 3e-6;

pub struct Oven {
    pub fabric: Fabric,
    finished: bool,
    radio: Radio,
}

impl Oven {
    pub fn new(prototype: Prototype, radio: Radio) -> Self {
        let fabric = Fabric::from(prototype);
        Self {
            fabric,
            radio,
            finished: false,
        }
    }

    pub fn iterate(&mut self) -> Option<Baked> {
        if self.finished {
            return None;
        }
        for _ in 0..60 {
            self.fabric.iterate(&PROTOTYPE_FORMATION);
        }
        let max_velocity = self.fabric.max_velocity();
        let age = self.fabric.age;
        if age.brick_baked() && max_velocity < SPEED_LIMIT {
            self.finished = true;
            println!("Fabric settled in {age} at velocity {max_velocity}");
            let (_, max) = self.fabric.strain_limits(Material::Pull);
            // let strains = self
            //     .fabric
            //     .interval_values()
            //     .filter(|interval| matches!(interval_material(interval.material).role, Role::Pull))
            //     .map(|interval| interval.strain).collect_vec();
            StateChange::SetAppearanceFunction(Rc::new(move |interval| {
                let strain = interval.strain;
                let intensity = strain / max;
                let role = interval.material.properties().role;
                Some(
                    role.appearance()
                        .with_color([intensity, intensity, intensity, 1.0]),
                )
            }))
            .send(&self.radio.clone());
            match Baked::try_from(self.fabric.clone()) {
                Ok(baked) => {
                    self.fabric.check_orphan_joints();
                    println!("Baked it!");
                    return Some(baked);
                }
                Err(problem) => {
                    println!("Cannot create brick: {problem}");
                }
            }
        }
        None
    }

    pub fn prototype_fabric(&self) -> Fabric {
        self.fabric.clone()
    }
}
