/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::fabric::face::Face;
use crate::fabric::interval::Interval;
use crate::fabric::interval::Role::{Pull, Push, Spring};
use crate::fabric::interval::Span::{Approaching, Fixed, Muscle};
use crate::fabric::joint::Joint;
use crate::fabric::material::Material::{NorthMaterial, SouthMaterial};
use crate::fabric::material::{interval_material, IntervalMaterial};
use crate::fabric::physics::Physics;
use crate::fabric::progress::Progress;
use cgmath::num_traits::zero;
use cgmath::{EuclideanSpace, InnerSpace, Matrix4, Point3, Transform, Vector3};
use std::collections::HashMap;
use std::fmt::Debug;

pub mod brick;
pub mod face;
pub mod interval;
pub mod joint;
pub mod physics;
pub mod progress;
pub mod vulcanize;

pub mod correction;
pub mod export;
pub mod joint_incident;
pub mod material;

pub const MAX_INTERVALS: usize = 5000;
pub const ROOT3: f32 = 1.732_050_8;

#[derive(Clone, Debug)]
pub struct FabricStats {
    pub age: u64,
    pub scale: f32,
    pub joint_count: usize,
    pub max_height: f32,
    pub push_count: usize,
    pub push_range: (f32, f32),
    pub push_total: f32,
    pub pull_count: usize,
    pub pull_range: (f32, f32),
    pub pull_total: f32,
}

#[derive(Clone, Debug)]
pub struct Fabric {
    pub age: u64,
    pub progress: Progress,
    pub joints: Vec<Joint>,
    pub intervals: HashMap<UniqueId, Interval>,
    pub faces: HashMap<UniqueId, Face>,
    pub scale: f32,
    muscle_forward: Option<bool>,
    muscle_nuance: f32,
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
            scale: 1.0,
            muscle_nuance: 0.5,
            muscle_forward: None,
            unique_id: 0,
        }
    }
}

impl Fabric {
    pub fn apply_matrix4(&mut self, matrix: Matrix4<f32>) {
        for joint in &mut self.joints {
            joint.location = matrix.transform_point(joint.location);
            joint.velocity = matrix.transform_vector(joint.velocity);
        }
    }

    pub fn centralize(&mut self, altitude: Option<f32>) {
        let mut midpoint: Vector3<f32> = zero();
        for joint in self.joints.iter() {
            midpoint += joint.location.to_vec();
        }
        midpoint /= self.joints.len() as f32;
        midpoint.y = 0.0;
        for joint in self.joints.iter_mut() {
            joint.location -= midpoint;
        }
        if let Some(altitude) = altitude {
            let min_y = self
                .joints
                .iter()
                .map(|Joint { location, .. }| location.y)
                .min_by(|a, b| a.partial_cmp(b).unwrap());
            if let Some(min_y) = min_y {
                for joint in &mut self.joints {
                    joint.location.y -= min_y - altitude;
                }
            }
        }
    }

    pub fn prepare_for_pretensing(&mut self, push_extension: f32) {
        for interval in self.intervals.values_mut() {
            let IntervalMaterial { role, support, .. } = interval_material(interval.material);
            if !support {
                let length = interval.fast_length(&self.joints);
                interval.span = match role {
                    Push => Approaching {
                        initial: length,
                        length: length * push_extension,
                    },
                    Pull | Spring => Fixed { length },
                };
            }
        }
        for joint in self.joints.iter_mut() {
            joint.force = zero();
            joint.velocity = zero();
        }
    }

    pub fn max_velocity(&self) -> f32 {
        let index = self
            .joints
            .iter()
            .enumerate()
            .map(|(a, b)| (a, b.velocity.magnitude2()))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(index, _)| index);
        match index {
            None => 0.0,
            Some(index) => self.joints[index].velocity.magnitude(),
        }
    }

    pub fn failed_intervals(&self, strain_limit: f32) -> Vec<UniqueId> {
        self.intervals
            .iter()
            .filter(|(_, interval)| interval.strain > strain_limit)
            .map(|(id, _)| *id)
            .collect()
    }

    pub fn iterate(&mut self, physics: &Physics) {
        for joint in &mut self.joints {
            joint.reset();
        }
        for interval in self.intervals.values_mut() {
            interval.iterate(
                &mut self.joints,
                &self.progress,
                self.muscle_nuance,
                physics,
            );
        }
        for joint in &mut self.joints {
            joint.iterate(physics);
        }
        if self.progress.step() {
            // final step
            for interval in self.intervals.values_mut() {
                if let Approaching { length, .. } = interval.span {
                    interval.span = Fixed { length }
                }
            }
        }
        if let Some(forward) = self.muscle_forward {
            let increment = 1.0 / physics.cycle_ticks * if forward { 1.0 } else { -1.0 };
            self.muscle_nuance += increment;
            if self.muscle_nuance < 0.0 {
                self.muscle_nuance = 0.0;
                self.muscle_forward = Some(true);
            } else if self.muscle_nuance > 1.0 {
                self.muscle_nuance = 1.0;
                self.muscle_forward = Some(false);
            }
        }
        self.age += 1;
    }

    pub fn create_muscles(&mut self, contraction: f32) {
        self.muscle_nuance = 0.5;
        for interval in self.intervals.values_mut() {
            let Fixed { length } = interval.span else {
                continue;
            };
            let contracted = length * contraction;
            if interval.material == NorthMaterial {
                interval.span = Muscle {
                    length,
                    contracted,
                    reverse: false,
                };
            }
            if interval.material == SouthMaterial {
                interval.span = Muscle {
                    length,
                    contracted,
                    reverse: true,
                };
            }
        }
    }

    pub fn activate_muscles(&mut self, go: bool) {
        self.muscle_nuance = 0.5;
        self.muscle_forward = go.then_some(true);
    }

    pub fn midpoint(&self) -> Point3<f32> {
        let mut midpoint: Point3<f32> = Point3::origin();
        for joint in &self.joints {
            midpoint += joint.location.to_vec();
        }
        let denominator = if self.joints.is_empty() {
            1
        } else {
            self.joints.len()
        } as f32;
        midpoint / denominator
    }

    fn create_id(&mut self) -> UniqueId {
        let id = UniqueId(self.unique_id);
        self.unique_id += 1;
        id
    }

    pub fn check_orphan_joints(&self) {
        for joint in 0..self.joints.len() {
            let touching = self
                .interval_values()
                .any(|interval| interval.touches(joint));
            if !touching {
                panic!("Found an orphan joint!");
            }
        }
    }

    pub fn fabric_stats(&self) -> FabricStats {
        let mut push_range = (1000.0, 0.0);
        let mut pull_range = (1000.0, 0.0);
        let mut push_count = 0;
        let mut push_total = 0.0;
        let mut pull_count = 0;
        let mut pull_total = 0.0;
        let mut max_height = 0.0;
        for Joint { location, .. } in self.joints.iter() {
            if location.y > max_height {
                max_height = location.y;
            }
        }
        for interval in self.intervals.values() {
            let length = interval.length(&self.joints);
            let material = interval_material(interval.material);
            if !material.support {
                match material.role {
                    Push => {
                        push_count += 1;
                        push_total += length;
                        if length < push_range.0 {
                            push_range.0 = length;
                        }
                        if length > push_range.1 {
                            push_range.1 = length;
                        }
                    }
                    Pull => {
                        pull_count += 1;
                        pull_total += length;
                        if length < pull_range.0 {
                            pull_range.0 = length;
                        }
                        if length > pull_range.1 {
                            pull_range.1 = length;
                        }
                    }
                    Spring => unreachable!(),
                }
            }
        }
        FabricStats {
            age: self.age,
            scale: self.scale,
            joint_count: self.joints.len(),
            max_height,
            push_count,
            push_range,
            push_total,
            pull_count,
            pull_range,
            pull_total,
        }
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Default, Hash, Eq, Ord, PartialOrd)]
pub struct UniqueId(usize);
