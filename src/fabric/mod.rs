/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use cgmath::num_traits::zero;
use cgmath::{EuclideanSpace, Matrix4, Point3, Transform, Vector3};
use std::collections::HashMap;
use std::fmt::Debug;

use crate::fabric::face::Face;
use crate::fabric::interval::Interval;
use crate::fabric::interval::Role::{Pull, Push, Spring};
use crate::fabric::interval::Span::{Approaching, Fixed};
use crate::fabric::joint::Joint;
use crate::fabric::material::{interval_material, IntervalMaterial};
use crate::fabric::physics::Physics;
use crate::fabric::progress::Progress;

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

#[derive(Clone)]
pub struct FabricStats {
    pub scale: f32,
    pub joint_count: usize,
    pub max_height: f32,
    pub push_count: usize,
    pub push_range: (f32, f32),
    pub push_total: f32,
    pub pull_count: usize,
    pub pull_range: (f32, f32),
    pub pull_total: f32,
    pub muscles: bool,
}

impl Debug for FabricStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let FabricStats {
            scale,
            joint_count,
            max_height,
            push_count,
            push_total,
            push_range,
            pull_count,
            pull_range,
            pull_total,
            muscles,
        } = self;
        let muscle_string = if *muscles { "Type M to activate muscles." } else { "" };
        write!(f,
               "At scale {:.0}:1 this structure has {:?} joints (up to height {:.0}mm), {:?} pushes ({:.1} to {:.1}mm, total {:.2}m), and {:?} pulls ({:.1} to {:.1}mm, total {:.2}m). {}",
               scale,
               joint_count,
               max_height * scale,
               push_count,
               push_range.0 * scale,
               push_range.1 * scale,
               push_total * scale / 1000.0,
               pull_count,
               pull_range.0 * scale,
               pull_range.1 * scale,
               pull_total * scale / 1000.0,
               muscle_string
        )
    }
}

#[derive(Clone, Debug)]
pub struct Fabric {
    pub age: u64,
    pub progress: Progress,
    pub muscle_nuance: f32,
    pub joints: Vec<Joint>,
    pub intervals: HashMap<UniqueId, Interval>,
    pub faces: HashMap<UniqueId, Face>,
    pub scale: f32,
    unique_id: usize,
}

impl Default for Fabric {
    fn default() -> Fabric {
        Fabric {
            age: 0,
            progress: Progress::default(),
            muscle_nuance: 0.5,
            joints: Vec::new(),
            intervals: HashMap::new(),
            faces: HashMap::new(),
            scale: 1.0,
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

    pub fn iterate(&mut self, physics: &Physics) -> f32 {
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
        let mut max_speed_squared = 0.0;
        for joint in &mut self.joints {
            let speed_squared = joint.iterate(physics);
            if speed_squared > max_speed_squared {
                max_speed_squared = speed_squared;
            }
        }
        if self.progress.step() {
            // final step
            for interval in self.intervals.values_mut() {
                if let Approaching { length, .. } = interval.span {
                    interval.span = Fixed { length }
                }
            }
        }
        self.age += 1;
        max_speed_squared
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

    pub fn fabric_stats(&self, muscles:bool) -> FabricStats {
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
            match interval_material(interval.material).role {
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
        FabricStats {
            scale: self.scale,
            joint_count: self.joints.len(),
            max_height,
            push_count,
            push_range,
            push_total,
            pull_count,
            pull_range,
            pull_total,
            muscles,
        }
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Default, Hash, Eq, Ord, PartialOrd)]
pub struct UniqueId(usize);
