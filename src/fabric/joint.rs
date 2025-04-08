/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use cgmath::num_traits::zero;
use cgmath::{InnerSpace, MetricSpace, Point3, Vector3};

use crate::fabric::physics::Physics;
use crate::fabric::physics::SurfaceCharacter::*;
use crate::fabric::Fabric;

impl Fabric {
    pub fn create_joint(&mut self, point: Point3<f32>) -> usize {
        let index = self.joints.len();
        self.joints.push(Joint::new(point));
        index
    }

    pub fn create_fixed_joint(&mut self, point: Point3<f32>) -> usize {
        let index = self.joints.len();
        let mut joint = Joint::new(point);
        joint.fixed = true;
        self.joints.push(joint);
        index
    }

    pub fn location(&self, index: usize) -> Point3<f32> {
        self.joints[index].location
    }

    pub fn remove_joint(&mut self, index: usize) {
        self.joints.remove(index);
        self.intervals
            .values_mut()
            .for_each(|interval| interval.joint_removed(index));
    }

    pub fn distance(&self, alpha_index: usize, omega_index: usize) -> f32 {
        self.location(alpha_index)
            .distance(self.location(omega_index))
    }

    pub fn ideal(&self, alpha_index: usize, omega_index: usize, strain: f32) -> f32 {
        let distance = self.distance(alpha_index, omega_index);
        distance / (1.0 + strain)
    }
}

const RESURFACE: f32 = 0.01;
const AMBIENT_MASS: f32 = 0.01;
const STICKY_DOWN_DRAG_FACTOR: f32 = 0.8;

#[derive(Clone, Copy, Debug)]
pub struct Joint {
    pub location: Point3<f32>,
    pub force: Vector3<f32>,
    pub velocity: Vector3<f32>,
    pub accumulated_mass: f32,
    pub fixed: bool,
}

impl Joint {
    pub fn new(location: Point3<f32>) -> Joint {
        Joint {
            location,
            force: zero(),
            velocity: zero(),
            accumulated_mass: AMBIENT_MASS,
            fixed: false,
        }
    }

    pub fn reset(&mut self) {
        self.force = zero();
        self.accumulated_mass = AMBIENT_MASS;
    }

    pub fn iterate(&mut self, physics: &Physics) {
        if self.fixed {
            return;
        }
        let Physics {
            surface_character,
            mass,
            viscosity,
            drag,
            ..
        } = physics;
        let altitude = self.location.y;
        let mass = self.accumulated_mass * mass;
        if altitude >= 0.0 || !surface_character.has_gravity() {
            self.velocity.y -= surface_character.gravity();
            let speed_squared = self.velocity.magnitude2();
            self.velocity += self.force / mass - self.velocity * speed_squared * *viscosity;
            self.velocity *= 1.0 - *drag;
        } else {
            let degree_submerged: f32 = if -altitude < 1.0 { -altitude } else { 0.0 };
            let antigravity = physics.surface_character.antigravity() * degree_submerged;
            self.velocity += self.force / mass;
            match surface_character {
                Absent => {}
                Frozen => {
                    self.velocity = zero();
                    self.location.y = -RESURFACE;
                }
                Sticky => {
                    if self.velocity.y < 0.0 {
                        self.velocity.x *= STICKY_DOWN_DRAG_FACTOR;
                        self.velocity.y += antigravity;
                        self.velocity.z *= STICKY_DOWN_DRAG_FACTOR;
                    } else {
                        self.velocity.x *= 1.0 - drag;
                        self.velocity.y += antigravity;
                        self.velocity.z *= 1.0 - drag;
                    }
                }
                Bouncy => {
                    let degree_cushioned: f32 = 1.0 - degree_submerged;
                    self.velocity *= degree_cushioned;
                    self.velocity.y += antigravity;
                }
            }
        }
        self.location += self.velocity;
    }
}
