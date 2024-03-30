/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use cgmath::num_traits::zero;
use cgmath::{EuclideanSpace, InnerSpace, MetricSpace, Point3, Vector3};
use fast_inv_sqrt::InvSqrt32;

use crate::fabric::interval::Role::*;
use crate::fabric::interval::Span::*;
use crate::fabric::joint::Joint;
use crate::fabric::physics::Physics;
use crate::fabric::{Fabric, Link, Progress, UniqueId};

impl Fabric {
    pub fn create_interval(
        &mut self,
        alpha_index: usize,
        omega_index: usize,
        Link {
            ideal,
            material_name,
        }: Link,
    ) -> UniqueId {
        let id = self.create_id();
        let initial = self.joints[alpha_index]
            .location
            .distance(self.joints[omega_index].location);
        let material = self.material(material_name);
        let interval = Interval::new(
            alpha_index,
            omega_index,
            material,
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
        self.intervals.remove(&id);
    }

    pub fn interval_values(&self) -> impl Iterator<Item=&Interval> {
        self.intervals.values()
    }

}

#[derive(Clone, Copy, Debug)]
pub enum Span {
    Fixed { length: f32 },
    Approaching { length: f32, initial: f32 },
    Muscle { max: f32, min: f32 },
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Role {
    Push,
    Pull,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    pub name: &'static str,
    pub role: Role,
    pub stiffness: f32,
    pub mass: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Interval {
    pub alpha_index: usize,
    pub omega_index: usize,
    pub material: usize,
    pub span: Span,
    pub unit: Vector3<f32>,
    pub strain: f32,
}

impl Interval {
    pub fn new(alpha_index: usize, omega_index: usize, material: usize, span: Span) -> Interval {
        Interval {
            alpha_index,
            omega_index,
            material,
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

    pub fn length(&mut self, joints: &[Joint]) -> f32 {
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

    pub fn ideal(&self) -> f32 {
        match self.span {
            Fixed { length, .. } | Approaching { length, .. } => length,
            Muscle { max, min, .. } => (max + min) / 2.0,
        }
    }

    pub fn iterate(
        &mut self,
        joints: &mut [Joint],
        materials: &[Material],
        progress: &Progress,
        muscle_nuance: f32,
        physics: &Physics,
    ) {
        let ideal = match self.span {
            Fixed { length } => length,
            Approaching {
                initial, length, ..
            } => {
                let nuance = progress.nuance();
                initial * (1.0 - nuance) + length * nuance
            }
            Muscle { max, min } => min * (1.0 - muscle_nuance) + max * muscle_nuance,
        };
        let real_length = self.length(joints);
        let Material {
            role,
            stiffness,
            mass,
            ..
        } = materials[self.material];
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
