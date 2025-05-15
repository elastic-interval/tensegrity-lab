/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::fabric::face::Face;
use crate::fabric::interval::Span::{Approaching, Fixed, Muscle, Pretenst};
use crate::fabric::interval::{Interval, Role};
use crate::fabric::joint::Joint;
use crate::fabric::material::Material::{North, South};
use crate::fabric::material::MaterialProperties;
use crate::fabric::physics::Physics;
use crate::fabric::progress::Progress;
use crate::Age;
use cgmath::num_traits::zero;
use cgmath::{EuclideanSpace, InnerSpace, Matrix4, Point3, Transform, Vector3};
use std::collections::HashMap;
use std::fmt::Debug;

pub mod attachment;
pub mod brick;
pub mod error;
pub mod correction;
pub mod face;
pub mod interval;
pub mod joint;
pub mod joint_incident;
pub mod material;
pub mod physics;
pub mod progress;
pub mod vulcanize;

#[cfg(not(target_arch = "wasm32"))]
pub mod export;

#[derive(Clone, Debug)]
pub struct FabricStats {
    pub name: String,
    pub age: Age,
    pub scale: f32,
    pub joint_count: usize,
    pub max_height: f32,
    pub push_count: usize,
    pub push_range: (f32, f32),
    pub push_total: f32,
    pub pull_count: usize,
    pub pull_range: (f32, f32),
    pub pull_total: f32,
    pub fabric: Option<Fabric>,
}

#[derive(Clone, Debug)]
pub struct Fabric {
    pub name: String,
    pub age: Age,
    pub progress: Progress,
    pub joints: Vec<Joint>,
    pub intervals: Vec<Option<Interval>>,
    pub interval_count: usize,
    pub faces: HashMap<UniqueId, Face>,
    pub scale: f32,
    muscle_forward: Option<bool>,
    muscle_nuance: f32,
}

impl Fabric {
    pub fn new(name: String) -> Self {
        Self {
            name,
            age: Age::default(),
            progress: Progress::default(),
            joints: Vec::new(),
            intervals: Vec::new(),
            interval_count: 0,
            faces: HashMap::new(),
            scale: 1.0,
            muscle_nuance: 0.5,
            muscle_forward: None,
        }
    }

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

    pub fn slacken(&mut self) {
        for interval_opt in self.intervals.iter_mut().filter(|i| i.is_some()) {
            let interval = interval_opt.as_mut().unwrap();
            let MaterialProperties { support, .. } = interval.material.properties();
            if !support {
                interval.span = Fixed {
                    length: interval.fast_length(&self.joints),
                };
            }
        }
        for joint in self.joints.iter_mut() {
            joint.force = zero();
            joint.velocity = zero();
        }
    }

    pub fn set_pretenst(&mut self, pretenst: f32, countdown: usize) {
        for interval_opt in self.intervals.iter_mut().filter(|i| i.is_some()) {
            let interval = interval_opt.as_mut().unwrap();
            let MaterialProperties { role, support, .. } = interval.material.properties();
            if !support {
                match &mut interval.span {
                    Fixed { length } => {
                        if matches!(role, Role::Pushing) {
                            interval.span = Pretenst {
                                begin: *length,
                                length: *length * (1.0 + pretenst / 100.0),
                                slack: *length,
                                finished: false,
                            };
                        }
                    }
                    Pretenst { length, slack, .. } => {
                        interval.span = Pretenst {
                            begin: *length,
                            length: *slack * (1.0 + pretenst / 100.0),
                            slack: *slack,
                            finished: false,
                        }
                    }
                    _ => {}
                }
            }
        }
        self.progress.start(countdown);
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
            .enumerate()
            .filter_map(|(index, interval_opt)| {
                interval_opt.as_ref().and_then(|interval| {
                    if interval.strain > strain_limit {
                        Some(UniqueId(index))
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    pub fn iterate(&mut self, physics: &Physics) {
        for joint in &mut self.joints {
            joint.reset();
        }
        for interval_opt in self.intervals.iter_mut().filter(|i| i.is_some()) {
            let interval = interval_opt.as_mut().unwrap();
            interval.iterate(
                &mut self.joints,
                &self.progress,
                self.muscle_nuance,
                physics,
            );
        }
        let elapsed = self.age.tick();
        for joint in &mut self.joints {
            joint.iterate(physics, elapsed);
        }
        if self.progress.step() {
            // final step
            for interval_opt in self.intervals.iter_mut().filter(|i| i.is_some()) {
                let interval = interval_opt.as_mut().unwrap();
                match &mut interval.span {
                    Fixed { .. } => {}
                    Pretenst { finished, .. } => {
                        *finished = true;
                    }
                    Approaching { length, .. } => {
                        interval.span = Fixed { length: *length };
                    }
                    Muscle { .. } => {}
                }
            }
            
            // Update all attachment connections at the end of the pretenst phase
            // This assigns each pull interval to its nearest attachment point on connected push intervals
            self.update_all_attachment_connections();
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
    }

    pub fn create_muscles(&mut self, contraction: f32) {
        self.muscle_nuance = 0.5;
        for interval_opt in self.intervals.iter_mut().filter(|i| i.is_some()) {
            let interval = interval_opt.as_mut().unwrap();
            let Fixed { length } = interval.span else {
                continue;
            };
            let contracted = length * contraction;
            if interval.material == North {
                interval.span = Muscle {
                    length,
                    contracted,
                    reverse: false,
                };
            }
            if interval.material == South {
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
        // Find an empty slot or create a new one
        for (index, interval_opt) in self.intervals.iter().enumerate() {
            if interval_opt.is_none() {
                return UniqueId(index);
            }
        }
        // No empty slots found, add a new one
        let id = UniqueId(self.intervals.len());
        self.intervals.push(None);
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
        for interval_opt in self.intervals.iter().filter(|i| i.is_some()) {
            let interval = interval_opt.as_ref().unwrap();
            let length = interval.length(&self.joints);
            let material = interval.material.properties();
            if !material.support {
                match material.role {
                    Role::Pushing => {
                        push_count += 1;
                        push_total += length;
                        if length < push_range.0 {
                            push_range.0 = length;
                        }
                        if length > push_range.1 {
                            push_range.1 = length;
                        }
                    }
                    Role::Pulling => {
                        pull_count += 1;
                        pull_total += length;
                        if length < pull_range.0 {
                            pull_range.0 = length;
                        }
                        if length > pull_range.1 {
                            pull_range.1 = length;
                        }
                    }
                    _ => unreachable!(),
                }
            }
        }
        FabricStats {
            name: self.name.clone(),
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
            fabric: Some(self.clone()),
        }
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Default, Hash, Eq, Ord, PartialOrd)]
pub struct UniqueId(pub usize);
