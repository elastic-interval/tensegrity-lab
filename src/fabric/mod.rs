/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use std::collections::HashMap;

use cgmath::{EuclideanSpace, Matrix4, Point3, Transform, Vector3};
use cgmath::num_traits::zero;

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

pub mod joint_incident;
pub mod material;
pub mod correction;

pub const MAX_INTERVALS: usize = 5000;

#[derive(Clone, Debug)]
pub struct Fabric {
    pub age: u64,
    pub progress: Progress,
    pub muscle_nuance: f32,
    pub joints: Vec<Joint>,
    pub intervals: HashMap<UniqueId, Interval>,
    pub faces: HashMap<UniqueId, Face>,
    pub stay_above: bool,
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
            stay_above: true,
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

    pub fn prepare_for_pretensing(&mut self, push_extension: f32) {
        for interval in self.intervals.values_mut() {
            let length = interval.length(&self.joints);
            let IntervalMaterial { role, .. } = interval_material(interval.material);
            interval.span = match role {
                Push => Approaching {
                    initial: length,
                    length: length * push_extension,
                },
                Pull | Spring => Fixed { length },
            };
        }
        for joint in self.joints.iter_mut() {
            joint.force = zero();
            joint.velocity = zero();
        }
        self.centralize();
        self.set_altitude(1.0);
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
        if self.stay_above {
            let min_y = self.joints
                .iter()
                .map(|Joint { location, .. }| location.y)
                .min_by(|a, b| a.partial_cmp(b).unwrap());
            if let Some(min_y) = min_y {
                for joint in &mut self.joints {
                    joint.location.y -= min_y - 1.0;
                }
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

    pub fn check_orphan_joints(&self)  {
        for joint in 0..self.joints.len() {
            let touching = self
                .interval_values()
                .any(|interval| interval.touches(joint));
            if !touching {
                panic!("Found an orphan joint!");
            }
        }
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Default, Hash, Eq, Ord, PartialOrd)]
pub struct UniqueId(usize);



