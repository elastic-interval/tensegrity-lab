/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::fabric::physics::{Physics, SurfaceCharacter::*};
use crate::fabric::{Fabric, Force, Location, UniqueId, Velocity};
use crate::units::Grams;
use crate::ITERATION_DURATION;
use cgmath::num_traits::zero;
use cgmath::{InnerSpace, MetricSpace, Point3};
use itertools::Itertools;

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
        self.intervals
            .iter()
            .enumerate()
            .filter_map(|(idx, interval_opt)| {
                interval_opt.as_ref().and_then(|interval| {
                    if interval.touches(index) {
                        Some(UniqueId(idx))
                    } else {
                        None
                    }
                })
            })
            .collect_vec()
            .into_iter()
            .for_each(|id| {
                self.remove_interval(id);
            });
        self.intervals
            .iter_mut()
            .filter_map(|interval_opt| interval_opt.as_mut())
            .for_each(|interval| interval.joint_removed(index));
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

pub const AMBIENT_MASS: Grams = Grams(0.01);
const STICKY_DOWN_DRAG_FACTOR: f32 = 0.8;

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

    pub fn iterate(&mut self, physics: &Physics, scale: f32) {
        let surface_character = &physics.surface_character;
        let drag = physics.drag();
        let viscosity = physics.viscosity();

        let altitude = self.location.y;
        let mass = *self.accumulated_mass;
        let dt = ITERATION_DURATION.secs;

        // Surface contact tolerance - treat joints very close to surface as "on surface"
        // This prevents hovering/wobbling from numerical precision issues
        const SURFACE_TOLERANCE: f32 = 0.01; // mm - joints within this distance are "on surface"

        if altitude > SURFACE_TOLERANCE || !surface_character.has_gravity() {
            // Apply gravity: acceleration (mm/s²) × dt (s) / scale = velocity change (internal_unit/s)
            self.velocity.y -= *surface_character.acceleration_of_gravity() * dt / scale;
            let speed_squared = self.velocity.magnitude2();

            // Apply spring forces and damping
            self.velocity +=
                (self.force / mass / scale) * dt - self.velocity * speed_squared * viscosity * dt;
            self.velocity *= 1.0 - drag * dt;

            // Clamp velocity to prevent numerical instability
            const MAX_VELOCITY: f32 = 100.0; // internal_unit/s
            let speed = self.velocity.magnitude();
            if speed > MAX_VELOCITY {
                self.velocity *= MAX_VELOCITY / speed;
            }
        } else {
            let depth = -altitude;
            let degree_submerged: f32 = depth.min(1.0); // Clamp to [0, 1]

            // Apply forces from intervals (scaled to internal units)
            self.velocity += (self.force / mass / scale) * dt;

            match surface_character {
                Absent => {
                    // No surface interaction
                }
                Frozen => {
                    // Completely locked to surface
                    self.velocity = zero();
                    let mut pos = self.location;
                    pos.y = 0.0;
                    self.location = pos;
                }
                Sticky => {
                    // High friction surface - resists horizontal motion and prevents sinking

                    // Very strong horizontal friction
                    let friction = if self.velocity.y < 0.0 {
                        STICKY_DOWN_DRAG_FACTOR // 0.8 - strong damping when pushing down
                    } else {
                        1.0 - drag * dt // Normal drag when pulling up
                    };
                    self.velocity.x *= friction;
                    self.velocity.z *= friction;

                    // Strong upward force to prevent sinking - much stronger than other surfaces
                    // Multiply by mass for buoyancy force
                    let antigravity = physics.surface_character.antigravity()
                        * *surface_character.acceleration_of_gravity()
                        * mass
                        * degree_submerged
                        * 50.0;
                    self.velocity.y += (antigravity / scale) * dt;

                    // Hard clamp: don't allow sinking below surface
                    if self.velocity.y < 0.0 {
                        self.velocity.y *= 0.5; // Dampen downward motion
                    }

                    // If significantly submerged, force back to surface
                    if depth > 0.1 {
                        let mut pos = self.location;
                        pos.y = -0.1;
                        self.location = pos;
                        self.velocity.y = 0.0;
                    }
                }
                Bouncy => {
                    // Elastic collision - reflects velocity with energy loss
                    // Strong resistance to horizontal slipping
                    if self.velocity.y < 0.0 {
                        // Bounce back with coefficient of restitution ~0.5
                        self.velocity.y *= -0.5;
                    }

                    // Strong horizontal friction on contact - resist slipping
                    let horizontal_friction = 0.6; // High friction coefficient
                    self.velocity.x *= horizontal_friction;
                    self.velocity.z *= horizontal_friction;

                    // Push out of surface
                    // Multiply by mass for buoyancy force
                    let antigravity = physics.surface_character.antigravity()
                        * *surface_character.acceleration_of_gravity()
                        * mass
                        * degree_submerged
                        * 5.0;
                    self.velocity.y += (antigravity / scale) * dt;
                }
                Slippery => {
                    // Surface that holds joints on contact (like Frozen) but allows horizontal sliding
                    // Once a joint touches, it cannot leave the surface - prevents bouncing/wobbling

                    // Clamp to surface and zero all vertical motion
                    let mut pos = self.location;
                    pos.y = 0.0;
                    self.location = pos;
                    self.velocity.y = 0.0;

                    let speed_horizontal = (self.velocity.x * self.velocity.x
                        + self.velocity.z * self.velocity.z)
                        .sqrt();

                    // Base surface damping coefficient (independent of physics parameters)
                    // Applied to both drag and viscosity for consistent strong damping
                    const SURFACE_DAMPING: f32 = 50.0;

                    // Linear damping: surface damping multiplier + physics drag/viscosity
                    // Physics values increase during convergence for additional damping
                    let linear_friction = 1.0
                        - ((SURFACE_DAMPING + drag) * dt
                            + SURFACE_DAMPING * viscosity * speed_horizontal * dt);

                    // Quadratic damping (speed-squared) - strongly damps fast motion
                    // This is key for suppressing oscillations
                    let quadratic_damping = 1.0 - (2.0 * speed_horizontal * speed_horizontal * dt);

                    // Combine both damping effects
                    let total_friction = (linear_friction * quadratic_damping.max(0.0)).max(0.0);

                    self.velocity.x *= total_friction;
                    self.velocity.z *= total_friction;
                }
            }
        }
        // Update position
        self.location = &self.location + self.velocity * dt;
    }
}
