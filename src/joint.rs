/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use cgmath::{InnerSpace, Point3, Vector3};
use cgmath::num_traits::zero;
use crate::world::{Physics, SurfaceCharacter};

const RESURFACE: f32 = 0.01;
const AMBIENT_MASS: f32 = 0.001;
const AMBIENT_DRAG_FACTOR: f32 = 0.9999;
const STICKY_DOWN_DRAG_FACTOR: f32 = 0.8;

#[derive(Clone, Copy, Debug)]
pub struct Joint {
    pub location: Point3<f32>,
    pub force: Vector3<f32>,
    pub velocity: Vector3<f32>,
    pub speed2: f32,
    pub interval_mass: f32,
}

impl Joint {
    pub fn new(location: Point3<f32>) -> Joint {
        Joint {
            location,
            force: zero(),
            velocity: zero(),
            speed2: 0.0,
            interval_mass: AMBIENT_MASS,
        }
    }

    pub fn reset(&mut self) {
        self.force = zero();
        self.interval_mass = AMBIENT_MASS;
    }

    pub fn iterate(&mut self, surface_character: SurfaceCharacter, physics: &Physics) {
        let Physics { gravity, antigravity, viscosity, .. } = physics;
        let altitude = self.location.y;
        self.speed2 = self.velocity.magnitude2();
        if self.speed2 > 0.01 {
            panic!("speed too high {:?}", self);
        }
        if altitude >= 0.0 || *gravity == 0.0 {
            self.velocity.y -= gravity;
            self.velocity += self.force / self.interval_mass - self.velocity * self.speed2 * *viscosity;
            self.velocity *= AMBIENT_DRAG_FACTOR;
        } else {
            let degree_submerged: f32 = if -altitude < 1.0 { -altitude } else { 0.0 };
            let antigravity = antigravity * degree_submerged;
            self.velocity += self.force / self.interval_mass;
            match surface_character {
                SurfaceCharacter::Frozen => {
                    self.velocity = zero();
                    self.location.y = -RESURFACE;
                }
                SurfaceCharacter::Sticky => {
                    if self.velocity.y < 0.0 {
                        self.velocity.x *= STICKY_DOWN_DRAG_FACTOR;
                        self.velocity.y += antigravity;
                        self.velocity.z *= STICKY_DOWN_DRAG_FACTOR;
                    } else {
                        self.velocity.x *= AMBIENT_DRAG_FACTOR;
                        self.velocity.y += antigravity;
                        self.velocity.z *= AMBIENT_DRAG_FACTOR;
                    }
                }
                SurfaceCharacter::Bouncy => {
                    let degree_cushioned: f32 = 1.0 - degree_submerged;
                    self.velocity *= degree_cushioned;
                    self.velocity.y += antigravity;
                }
            }
        }
        self.location += self.velocity
    }
}
