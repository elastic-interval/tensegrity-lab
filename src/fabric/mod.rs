/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use std::cmp::Ordering;
use std::collections::HashMap;

use cgmath::{EuclideanSpace, Matrix4, MetricSpace, Point3, Transform, Vector3};
use cgmath::num_traits::zero;

use crate::build::tenscript::Spin;
use crate::fabric::face::Face;
use crate::fabric::interval::{Interval, Material, Role};
use crate::fabric::interval::Role::{Measure, Pull, Push};
use crate::fabric::interval::Span::{Approaching, Fixed};
use crate::fabric::joint::Joint;
use crate::fabric::physics::Physics;

pub mod annealing;
pub mod face;
pub mod interval;
pub mod joint;
pub mod physics;
pub mod twist;

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Progress {
    limit: usize,
    count: usize,
}

impl Progress {
    pub fn start(&mut self, countdown: usize) {
        self.count = 0;
        self.limit = countdown;
    }

    fn step(&mut self) -> bool { // true if it takes the final step
        let count = self.count + 1;
        if count > self.limit {
            return false;
        }
        self.count = count;
        self.count == self.limit // final step?
    }

    pub fn is_busy(&self) -> bool {
        self.count < self.limit
    }

    pub fn nuance(&self) -> f32 {
        if self.limit == 0 {
            1.0
        } else {
            (self.count as f32) / (self.limit as f32)
        }
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Default, Hash, Eq)]
pub struct UniqueId {
    pub id: usize,
}

#[derive(Clone)]
pub struct Fabric {
    pub age: u64,
    pub progress: Progress,
    pub joints: Vec<Joint>,
    pub intervals: HashMap<UniqueId, Interval>,
    pub faces: HashMap<UniqueId, Face>,
    pub push_material: Material,
    pub pull_material: Material,
    unique_id: usize,
}

impl Default for Fabric {
    fn default() -> Fabric {
        Fabric {
            age: 0,
            progress: Progress::default(),
            joints: Vec::new(),
            intervals: HashMap::new(),
            faces: HashMap::new(),
            push_material: Material {
                stiffness: 3.0,
                mass: 1.0,
            },
            pull_material: Material {
                stiffness: 1.0,
                mass: 0.1,
            },
            unique_id: 0,
        }
    }
}

impl Fabric {
    pub fn get_joint_count(&self) -> u16 {
        self.joints.len() as u16
    }

    pub fn get_interval_count(&self) -> u16 {
        self.intervals.len() as u16
    }

    pub fn get_face_count(&self) -> u16 {
        self.faces.len() as u16
    }

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
        self.intervals.values_mut().for_each(|interval| interval.joint_removed(index));
    }

    pub fn create_interval(&mut self, alpha_index: usize, omega_index: usize, role: Role, length: f32) -> UniqueId {
        let initial_length = self.joints[alpha_index].location.distance(self.joints[omega_index].location);
        let span = Approaching { initial_length, length };
        let id = self.create_id();
        let material = match role {
            Push => self.push_material,
            Pull | Measure => self.pull_material
        };
        self.intervals.insert(id, Interval::new(alpha_index, omega_index, role, material, span));
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

    pub fn interval_measures(&self) -> impl Iterator<Item=&Interval> {
        self.intervals.values().filter(|Interval { role, .. }| *role == Measure)
    }

    pub fn create_face(&mut self, scale: f32, spin: Spin, radial_intervals: [UniqueId; 3], push_intervals: [UniqueId; 3]) -> UniqueId {
        let id = self.create_id();
        self.faces.insert(id, Face { scale, spin, radial_intervals, push_intervals });
        id
    }

    pub fn face(&self, id: UniqueId) -> &Face {
        self.faces.get(&id).unwrap()
    }

    pub fn remove_face(&mut self, id: UniqueId) {
        let face = self.face(id);
        let middle_joint = face.middle_joint(self);
        for interval_id in face.radial_intervals {
            self.remove_interval(interval_id);
        }
        self.remove_joint(middle_joint);
        self.faces.remove(&id);
    }

    pub fn apply_matrix4(&mut self, matrix: Matrix4<f32>) {
        for joint in &mut self.joints {
            joint.location = matrix.transform_point(joint.location);
            joint.velocity = matrix.transform_vector(joint.velocity);
        }
    }

    pub fn centralize(&mut self) {
        let mut midpoint: Vector3<f32> = zero();
        for joint in self.joints.iter() {
            midpoint += joint.location.to_vec();
        }
        midpoint /= self.joints.len() as f32;
        midpoint.y = 0.0;
        for joint in self.joints.iter_mut() {
            joint.location -= midpoint;
        }
    }

    pub fn set_altitude(&mut self, altitude: f32) -> f32 {
        let bottom = self.joints.iter()
            .map(|joint| joint.location.y)
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        match bottom {
            None => 0.0,
            Some(low_y) => {
                let up = altitude - low_y;
                if up > 0.0 {
                    for joint in &mut self.joints {
                        joint.location.y += up;
                    }
                }
                up
            }
        }
    }

    pub fn prepare_for_pretensing(&mut self, push_extension: f32) -> f32 {
        for interval in self.intervals.values_mut() {
            let length = interval.length(&self.joints);
            interval.span = match interval.role {
                Push => Approaching { initial_length: length, length: length * push_extension },
                Pull | Measure => Fixed { length }
            };
        }
        for joint in self.joints.iter_mut() {
            joint.force = zero();
            joint.velocity = zero();
        }
        self.set_altitude(1.0)
    }

    pub fn iterate(&mut self, physics: &Physics) {
        for joint in &mut self.joints {
            joint.reset();
        }
        for interval in self.intervals.values_mut() {
            interval.iterate(&mut self.joints, &self.progress, physics);
        }
        for joint in &mut self.joints {
            joint.iterate(physics)
        }
        if self.progress.step() { // final step
            for interval in self.intervals.values_mut() {
                if let Approaching { length, .. } = interval.span {
                    interval.span = Fixed { length }
                }
            }
        }
        self.age += 1;
    }

    pub fn midpoint(&self) -> Point3<f32> {
        let mut midpoint: Point3<f32> = Point3::origin();
        for joint in &self.joints {
            midpoint += joint.location.to_vec();
        }
        let denominator = if self.joints.is_empty() { 1 } else { self.joints.len() } as f32;
        midpoint / denominator
    }

    fn create_id(&mut self) -> UniqueId {
        let id = UniqueId { id: self.unique_id };
        self.unique_id += 1;
        id
    }
}
