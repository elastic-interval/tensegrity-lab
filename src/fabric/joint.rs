/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::fabric::physics::{Physics, SurfaceInteraction};
use crate::fabric::{Fabric, Force, Location, Velocity};
use crate::units::Grams;
use crate::ITERATION_DURATION;
use cgmath::num_traits::zero;
use cgmath::{InnerSpace, MetricSpace, Point3};

impl Fabric {
    pub fn create_joint(&mut self, point: Point3<f32>) -> usize {
        let index = self.joints.len();
        self.joints.push(Joint::new(point));
        index
    }
    pub fn location(&self, index: usize) -> Point3<f32> {
        self.joints[index].location
    }

    pub fn remove_joint(&mut self, index: usize) {
        self.joints.remove(index);
        let to_remove: Vec<_> = self.intervals
            .iter()
            .filter_map(|(key, interval)| {
                if interval.touches(index) {
                    Some(key)
                } else {
                    None
                }
            })
            .collect();
        for key in to_remove {
            self.remove_interval(key);
        }
        for interval in self.intervals.values_mut() {
            interval.joint_removed(index);
        }
    }

    pub fn distance(&self, alpha_index: usize, omega_index: usize) -> f32 {
        self.location(alpha_index)
            .distance(self.location(omega_index))
    }

    pub fn ideal(&self, alpha_index: usize, omega_index: usize, strain: f32) -> f32 {
        let distance = self.distance(alpha_index, omega_index);
        distance / (1.0 + strain * distance)
    }
}

pub const AMBIENT_MASS: Grams = Grams(100.0);

#[derive(Clone, Debug)]
pub struct Joint {
    pub location: Location,
    pub force: Force,
    pub velocity: Velocity,
    pub accumulated_mass: Grams,
}

impl Joint {
    pub fn new(location: Point3<f32>) -> Joint {
        Joint {
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
