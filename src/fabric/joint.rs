/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::fabric::physics::{Physics, SurfaceInteraction};
use crate::fabric::{Fabric, Force, JointId, JointKey, Location, Velocity};
use crate::units::Grams;
use crate::{Age, ITERATION_DURATION};
use cgmath::num_traits::zero;
use cgmath::{InnerSpace, MetricSpace, Point3};

impl Fabric {
    pub fn create_joint(&mut self, point: Point3<f32>) -> JointKey {
        let id = JointId(self.joint_by_id.len());
        let born = self.age;
        let key = self.joints.insert(Joint::new(point, id, born));
        self.joint_by_id.push(key);
        key
    }

    pub fn location(&self, key: JointKey) -> Point3<f32> {
        self.joints[key].location
    }

    pub fn remove_joint(&mut self, key: JointKey) {
        // Remove all intervals that touch this joint
        let to_remove: Vec<_> = self
            .intervals
            .iter()
            .filter_map(|(interval_key, interval)| {
                if interval.alpha_key == key || interval.omega_key == key {
                    Some(interval_key)
                } else {
                    None
                }
            })
            .collect();
        for interval_key in to_remove {
            self.remove_interval(interval_key);
        }
        // Simply remove the joint - no index adjustment needed with SlotMap!
        self.joints.remove(key);
    }

    pub fn distance(&self, alpha_key: JointKey, omega_key: JointKey) -> f32 {
        self.location(alpha_key).distance(self.location(omega_key))
    }

    /// Distance between joints by their JointId (creation order)
    pub fn distance_by_id(&self, alpha_id: JointId, omega_id: JointId) -> f32 {
        let alpha_key = self.joint_by_id[*alpha_id];
        let omega_key = self.joint_by_id[*omega_id];
        self.distance(alpha_key, omega_key)
    }

    pub fn ideal(&self, alpha_key: JointKey, omega_key: JointKey, strain: f32) -> f32 {
        let distance = self.distance(alpha_key, omega_key);
        distance / (1.0 + strain * distance)
    }

    /// Resolve a JointId to a JointKey
    pub fn joint_key_by_id(&self, id: JointId) -> Option<JointKey> {
        self.joint_by_id.get(id.0).copied()
    }
}

pub const AMBIENT_MASS: Grams = Grams(100.0);

#[derive(Clone, Debug)]
pub struct Joint {
    pub id: JointId,
    /// Fabric age when this joint was born. Joints born together are symmetric.
    pub born: Age,
    pub location: Location,
    pub force: Force,
    pub velocity: Velocity,
    pub accumulated_mass: Grams,
}

impl Joint {
    pub fn new(location: Point3<f32>, id: JointId, born: Age) -> Joint {
        Joint {
            id,
            born,
            location,
            force: zero(),
            velocity: zero(),
            accumulated_mass: AMBIENT_MASS,
        }
    }

    pub fn reset(&mut self) {
        self.force = zero();
        self.accumulated_mass = AMBIENT_MASS;
    }

    pub fn reset_with_mass(&mut self, ambient_mass: Grams) {
        self.force = zero();
        self.accumulated_mass = ambient_mass;
    }

    pub fn iterate(&mut self, physics: &Physics) {
        let drag = physics.drag();
        let viscosity = physics.viscosity();
        let mass = *self.accumulated_mass;
        let dt = ITERATION_DURATION.secs;

        // Force is in Newtons, mass in grams (converted to kg)
        // a = F/m gives m/s², multiply by dt gives velocity change in m/s
        let force_velocity = (self.force / mass) * dt;

        match &physics.surface {
            None => {
                // No surface, no gravity - free floating
                let speed_squared = self.velocity.magnitude2();
                self.velocity += force_velocity - self.velocity * speed_squared * viscosity * dt;
                self.velocity *= 1.0 - drag * dt;
            }
            Some(surface) => {
                let result = surface.interact(SurfaceInteraction {
                    altitude: self.location.y,
                    velocity: self.velocity,
                    force_velocity,
                    drag,
                    viscosity,
                    mass,
                    dt,
                });
                self.velocity = result.velocity;
                if let Some(y) = result.clamp_y {
                    self.location.y = y;
                }
            }
        }
        self.location = &self.location + self.velocity * dt;
    }
}
