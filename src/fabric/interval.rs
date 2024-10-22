/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use std::ops::Mul;
use cgmath::{EuclideanSpace, InnerSpace, MetricSpace, Point3, Vector3};
use cgmath::num_traits::zero;
use fast_inv_sqrt::InvSqrt32;

use crate::fabric::{Fabric, Progress, UniqueId};
use crate::fabric::interval::Role::*;
use crate::fabric::interval::Span::*;
use crate::fabric::joint::Joint;
use crate::fabric::material::{interval_material, IntervalMaterial, Material};
use crate::fabric::physics::Physics;

impl Fabric {
    pub fn create_interval(
        &mut self,
        alpha_index: usize,
        omega_index: usize,
        ideal: f32,
        material: Material,
        group: usize,
    ) -> UniqueId {
        let id = self.create_id();
        let initial = self.joints[alpha_index]
            .location
            .distance(self.joints[omega_index].location);
        let interval = Interval::new(
            alpha_index,
            omega_index,
            material,
            group,
            Approaching {
                initial,
                length: ideal,
            },
        );
        self.intervals.insert(id, interval);
        id
    }

    pub fn interval(&self, id: UniqueId) -> &Interval {
        self.intervals.get(&id).unwrap()
    }

    pub fn remove_interval(&mut self, id: UniqueId) {
        if self.intervals.remove(&id).is_none() {
            panic!("Removing nonexistent interval {:?}", id);
        }
    }

    pub fn interval_values(&self) -> impl Iterator<Item=&Interval> {
        self.intervals.values()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Span {
    Fixed { length: f32 },
    Approaching { length: f32, initial: f32 },
    Muscle { length: f32, contracted: f32, reverse: bool },
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Role {
    Push,
    Pull,
    Spring,
}

#[derive(Clone, Copy, Debug)]
pub struct Interval {
    pub alpha_index: usize,
    pub omega_index: usize,
    pub material: Material,
    pub group: usize,
    pub span: Span,
    pub unit: Vector3<f32>,
    pub strain: f32,
}

pub const FACE_RADIAL_GROUP: usize = 255;
pub const JOINER_GROUP: usize = 254;
pub const SPACER_GROUP: usize = 253;

impl Interval {
    pub fn new(alpha_index: usize, omega_index: usize, material: Material, group: usize, span: Span) -> Interval {
        Interval {
            alpha_index,
            omega_index,
            material,
            group,
            span,
            unit: zero(),
            strain: 0.0,
        }
    }

    pub fn key(&self) -> (usize, usize) {
        if self.alpha_index < self.omega_index {
            (self.alpha_index, self.omega_index)
        } else {
            (self.omega_index, self.alpha_index)
        }
    }

    pub fn joint_removed(&mut self, index: usize) {
        if self.alpha_index > index {
            self.alpha_index -= 1;
        }
        if self.omega_index > index {
            self.omega_index -= 1;
        }
    }

    pub fn locations<'a>(&self, joints: &'a [Joint]) -> (&'a Point3<f32>, &'a Point3<f32>) {
        (
            &joints[self.alpha_index].location,
            &joints[self.omega_index].location,
        )
    }

    pub fn midpoint(&self, joints: &[Joint]) -> Point3<f32> {
        let (alpha, omega) = self.locations(joints);
        Point3::from_vec((alpha.to_vec() + omega.to_vec()) / 2f32)
    }

    pub fn fast_length(&mut self, joints: &[Joint]) -> f32 {
        let (alpha_location, omega_location) = self.locations(joints);
        self.unit = omega_location - alpha_location;
        let magnitude_squared = self.unit.magnitude2();
        if magnitude_squared < 0.00001 {
            return 0.00001;
        }
        let inverse_square_root = magnitude_squared.inv_sqrt32();
        self.unit *= inverse_square_root;
        1.0 / inverse_square_root
    }

    pub fn length(& self, joints: &[Joint]) -> f32 {
        let (alpha_location, omega_location) = self.locations(joints);
        let tween = omega_location - alpha_location;
        let magnitude_squared = tween.magnitude2();
        if magnitude_squared < 0.00001 {
            return 0.00001;
        }
        magnitude_squared.sqrt()
    }

    pub fn ideal(&self) -> f32 {
        match self.span {
            Fixed { length, .. } | Approaching { length, .. } | Muscle { length, .. } => length,
        }
    }

    pub fn iterate(
        &mut self,
        joints: &mut [Joint],
        progress: &Progress,
        muscle_nuance: f32,
        physics: &Physics,
    ) {
        let ideal = match self.span {
            Fixed { length } => length,
            Approaching {
                initial, length, ..
            } => {
                let progress_nuance = progress.nuance();
                initial * (1.0 - progress_nuance) + length * progress_nuance
            }
            Muscle { length, contracted, reverse } => {
                let nuance = if reverse { 1.0 - muscle_nuance } else { muscle_nuance };
                let progress_nuance = progress.nuance();
                let muscle_length = contracted * (1.0 - nuance) + length * nuance;
                length * (1.0 - progress_nuance) + muscle_length * progress_nuance
            }
        };
        let real_length = self.fast_length(joints);
        let IntervalMaterial {
            role,
            stiffness,
            mass,
            ..
        } = interval_material(self.material);
        self.strain = (real_length - ideal) / ideal;
        match role {
            Push if real_length > ideal => self.strain = 0.0, // do not pull
            Pull if real_length < ideal => self.strain = 0.0, // do not push
            _ => {}
        };
        let force = self.strain * stiffness * physics.stiffness;
        let force_vector: Vector3<f32> = self.unit * force / 2.0;
        joints[self.alpha_index].force += force_vector;
        joints[self.omega_index].force -= force_vector;
        let half_mass = mass * real_length / 2.0;
        joints[self.alpha_index].interval_mass += half_mass;
        joints[self.omega_index].interval_mass += half_mass;
    }

    pub fn touches(&self, joint: usize) -> bool {
        self.alpha_index == joint || self.omega_index == joint
    }
    
    pub fn ray_from(&self, joint_index: usize) -> Vector3<f32> {
        if self.alpha_index == joint_index {
            self.unit
        } else if self.omega_index == joint_index {
            self.unit.mul(-1.0)
        } else {
            panic!()
        }
    }

    pub fn other_joint(&self, joint_index: usize) -> usize {
        if self.alpha_index == joint_index {
            self.omega_index
        } else if self.omega_index == joint_index {
            self.alpha_index
        } else {
            panic!()
        }
    }

    pub fn joint_with(
        &self,
        Interval {
            alpha_index,
            omega_index,
            ..
        }: &Interval,
    ) -> Option<usize> {
        if self.alpha_index == *alpha_index || self.alpha_index == *omega_index {
            Some(self.alpha_index)
        } else if self.omega_index == *alpha_index || self.omega_index == *omega_index {
            Some(self.omega_index)
        } else {
            None
        }
    }
}
