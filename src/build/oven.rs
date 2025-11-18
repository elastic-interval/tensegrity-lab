use crate::build::tenscript::brick::{Baked, Prototype};
use crate::crucible_context::CrucibleContext;
use crate::fabric::physics::presets::CONSTRUCTION;
use crate::ITERATIONS_PER_FRAME;
use crate::fabric::Fabric;
use crate::{Radio, StateChange};
use std::rc::Rc;
use crate::fabric::material::Material;

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

    pub fn copy_physics_into(&self, context: &mut CrucibleContext) {
        *context.physics = CONSTRUCTION;
    }

    pub fn iterate(&mut self, context: &mut CrucibleContext) -> Option<Baked> {
        if self.finished {
            return None;
        }

        for _ in 0..ITERATIONS_PER_FRAME {
            context.fabric.iterate(context.physics);
        }
        let max_velocity = context.fabric.max_velocity();
        let age = context.fabric.age;
        if age.brick_baked() && max_velocity < SPEED_LIMIT {
            self.finished = true;
            println!("Fabric settled in {age} at velocity {max_velocity}");
            let (_, max) = self.fabric.strain_limits(Material::Pull);
            StateChange::SetAppearanceFunction(Rc::new(move |interval| {
                let strain = interval.strain;
                let intensity = strain / max;
                let role = interval.role;
                Some(
                    role.appearance()
                        .with_color([intensity, intensity, intensity, 1.0]),
                )
            }))
            .send(&self.radio.clone());
            match Baked::try_from(self.fabric.clone()) {
                Ok(baked) => {
                    self.fabric.check_orphan_joints();

                    // Update the context's fabric with our changes
                    context.replace_fabric(self.fabric.clone());

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
