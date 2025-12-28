/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::fabric::joint_path::JointPath;
use crate::fabric::physics::{Physics, SurfaceInteraction};
use crate::fabric::{Fabric, JointKey};
use crate::units::{Grams, Meters, Unit};
use crate::ITERATION_DURATION;
use glam::Vec3;

impl Fabric {
    /// Create a joint with a specific path (for structured brick creation)
    pub fn create_joint_with_path(&mut self, point: Vec3, path: JointPath) -> JointKey {
        self.joints.insert(Joint::new(point, path))
    }

    /// Create a joint with default path (for legacy code and non-brick joints)
    pub fn create_joint(&mut self, point: Vec3) -> JointKey {
        self.create_joint_with_path(point, JointPath::default())
    }

    pub fn location(&self, key: JointKey) -> Vec3 {
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

    pub fn distance(&self, alpha_key: JointKey, omega_key: JointKey) -> Meters {
        Meters(self.location(alpha_key).distance(self.location(omega_key)))
    }

    /// Find a joint by its path (linear search, only for setup)
    pub fn joint_key_by_path(&self, path: &JointPath) -> Option<JointKey> {
        self.joints
            .iter()
            .find(|(_, joint)| &joint.path == path)
            .map(|(key, _)| key)
    }
}

pub const AMBIENT_MASS: Grams = Grams(100.0);

#[derive(Clone, Debug)]
pub struct Joint {
    pub path: JointPath,
    pub location: Vec3,
    pub force: Vec3,
    pub velocity: Vec3,
    pub accumulated_mass: Grams,
}

impl Joint {
    pub fn new(location: Vec3, path: JointPath) -> Joint {
        Joint {
            path,
            location,
            force: Vec3::ZERO,
            velocity: Vec3::ZERO,
            accumulated_mass: AMBIENT_MASS,
        }
    }

    pub fn reset(&mut self) {
        self.force = Vec3::ZERO;
        self.accumulated_mass = AMBIENT_MASS;
    }

    pub fn reset_with_mass(&mut self, ambient_mass: Grams) {
        self.force = Vec3::ZERO;
        self.accumulated_mass = ambient_mass;
    }

    pub fn iterate(&mut self, physics: &Physics) {
        let drag = physics.drag();
        let viscosity = physics.viscosity();
        let mass = self.accumulated_mass.f32();
        let dt = ITERATION_DURATION.secs;

        // Force is in Newtons, mass in grams (converted to kg)
        // a = F/m gives m/s², multiply by dt gives velocity change in m/s
        let force_velocity = (self.force / mass) * dt;

        match &physics.surface {
            None => {
                // No surface, no gravity - free floating
                let speed_squared = self.velocity.length_squared();
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
        self.location = self.location + self.velocity * dt;
    }
}
