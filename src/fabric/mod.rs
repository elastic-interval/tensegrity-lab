/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::build::dsl::brick_dsl::{BrickRole, FaceName};
use crate::fabric::face::Face;
use crate::fabric::interval::Span::{Approaching, Fixed, Pretensing};
use crate::fabric::interval::{Interval, Role};
use crate::fabric::joint::{Joint, AMBIENT_MASS};
use crate::fabric::physics::Physics;
use crate::fabric::progress::Progress;
use crate::units::{Grams, Meters, Percent, Seconds};
use crate::Age;
use cgmath::num_traits::zero;
use cgmath::{EuclideanSpace, InnerSpace, Matrix4, MetricSpace, Point3, Quaternion, Rotation, Transform, Vector3};
use std::collections::HashMap;
use std::fmt::Debug;

pub mod attachment;
pub mod brick;
pub mod error;
pub mod face;
pub mod interval;
pub mod joint;
pub mod joint_incident;
pub mod material;
pub mod fabric_sampler;
pub mod physics;
pub mod progress;
pub mod vulcanize;

#[cfg(not(target_arch = "wasm32"))]
pub mod export;
pub mod physics_test;

// Type aliases for physics quantities - improves readability and provides
// hook for future unit-aware types without cluttering code with f32 generics
pub type Location = Point3<f32>;
pub type Velocity = Vector3<f32>;
pub type Force = Vector3<f32>;

/// Statistics accumulated during iteration with zero-cost pass-through
#[derive(Clone, Debug, Default)]
pub struct IterationStats {
    pub kinetic_energy: f32,
    pub max_speed: f32,
    pub total_mass: f32,
    pub max_strain: f32,
    pub strain_sum: f32,
    pub strain_count: usize,
    max_speed_squared: f32,
}

impl IterationStats {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn avg_strain(&self) -> f32 {
        if self.strain_count > 0 {
            self.strain_sum / self.strain_count as f32
        } else {
            0.0
        }
    }

    #[inline]
    pub fn accumulate_strain(&mut self, strain: f32) {
        let abs_strain = strain.abs();
        self.strain_sum += abs_strain;
        self.strain_count += 1;
        if abs_strain > self.max_strain {
            self.max_strain = abs_strain;
        }
    }

    #[inline]
    pub fn accumulate_joint(&mut self, mass: f32, speed_squared: f32) {
        self.kinetic_energy += 0.5 * mass * speed_squared;
        self.total_mass += mass;
    }

    #[inline]
    pub fn update_max_speed_squared(&mut self, speed_squared: f32) {
        if speed_squared > self.max_speed_squared {
            self.max_speed_squared = speed_squared;
        }
    }

    /// Finalize max_speed by computing sqrt once at the end
    #[inline]
    pub fn finalize(&mut self) {
        self.max_speed = self.max_speed_squared.sqrt();
    }
}

/// Represents which end of an interval (alpha or omega)
/// This is used throughout the fabric module for consistent handling of interval ends
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntervalEnd {
    /// The alpha (start) end of an interval
    Alpha,
    /// The omega (end) end of an interval
    Omega,
}

impl IntervalEnd {
    /// Get the opposite end
    pub fn opposite(&self) -> Self {
        match self {
            IntervalEnd::Alpha => IntervalEnd::Omega,
            IntervalEnd::Omega => IntervalEnd::Alpha,
        }
    }

    /// Convert to a string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            IntervalEnd::Alpha => "alpha",
            IntervalEnd::Omega => "omega",
        }
    }
}

#[derive(Clone, Debug)]
pub struct FabricStats {
    pub name: String,
    pub age: Age,
    pub joint_count: usize,
    pub height: Meters,
    pub push_count: usize,
    pub push_range: (Meters, Meters),
    pub push_total: Meters,
    pub pull_count: usize,
    pub pull_range: (Meters, Meters),
    pub pull_total: Meters,
}

#[derive(Clone, Debug)]
pub struct Fabric {
    pub name: String,
    pub age: Age,
    pub progress: Progress,
    pub joints: Vec<Joint>,
    pub intervals: Vec<Option<Interval>>,
    pub interval_count: usize,
    pub next_interval_id: usize,
    pub faces: HashMap<UniqueId, Face>,
    pub frozen: bool,
    pub stats: IterationStats,
    cached_bounding_radius: f32,
    scale: f32,
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
            next_interval_id: 0,
            faces: HashMap::new(),
            frozen: false,
            stats: IterationStats::default(),
            cached_bounding_radius: 0.0,
            scale: 1.0,
        }
    }

    /// Returns the fabric's scale factor (set during construction)
    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn ambient_mass(&self) -> Grams {
        // Mass scales with scale^3.5: volume (scale³) plus slight reduction for small structures
        Grams(*AMBIENT_MASS * self.scale.powf(3.5))
    }

    pub fn apply_matrix4(&mut self, matrix: Matrix4<f32>) {
        for joint in &mut self.joints {
            joint.location = matrix.transform_point(joint.location);
            joint.velocity = matrix.transform_vector(joint.velocity);
        }
    }

    /// Calculate the translation needed to centralize the fabric
    pub fn centralize_translation(&self, altitude: Option<f32>) -> Vector3<f32> {
        let mut midpoint: Vector3<f32> = zero();
        for joint in self.joints.iter() {
            midpoint += joint.location.to_vec();
        }
        midpoint /= self.joints.len() as f32;
        midpoint.y = 0.0;

        let mut total_translation = -midpoint;

        // Calculate altitude adjustment if specified
        if let Some(altitude) = altitude {
            let min_y = self
                .joints
                .iter()
                .map(|Joint { location, .. }| location.y)
                .min_by(|a, b| a.partial_cmp(b).unwrap());
            if let Some(min_y) = min_y {
                let altitude_adjustment = min_y - altitude;
                total_translation.y -= altitude_adjustment;
            }
        }

        total_translation
    }

    /// Apply a translation to all joints
    pub fn apply_translation(&mut self, translation: Vector3<f32>) {
        for joint in self.joints.iter_mut() {
            joint.location += translation;
        }
    }

    /// Scale all coordinates and interval lengths by the given factor.
    /// This converts from internal units to meters when called with the plan's scale.
    /// After this, all coordinates are in meters directly.
    /// Mass scales with scale⁴ to compensate for gravity not scaling:
    /// - Volume scaling gives scale³
    /// - Additional scale factor represents using lighter materials at small scales
    ///   to maintain structural integrity against (relatively stronger) gravity
    pub fn apply_scale(&mut self, scale: Meters) {
        let s = *scale;
        self.scale = s;
        let mass_scale = s.powf(3.5); // scale^3.5: volume plus slight reduction for small structures
        // Scale all joint positions, velocities, and mass
        for joint in self.joints.iter_mut() {
            joint.location.x *= s;
            joint.location.y *= s;
            joint.location.z *= s;
            joint.velocity.x *= s;
            joint.velocity.y *= s;
            joint.velocity.z *= s;
            // Forces will be recalculated on next iteration
            joint.force = cgmath::num_traits::zero();
            // Scale mass with volume (thinner intervals at small scale)
            joint.accumulated_mass = Grams(*joint.accumulated_mass * mass_scale);
        }
        // Scale all interval ideal lengths
        for interval_opt in self.intervals.iter_mut() {
            if let Some(interval) = interval_opt {
                interval.scale_lengths(s);
            }
        }
        // Scale face scales
        for face in self.faces.values_mut() {
            face.scale *= s;
        }
        // Scale cached bounding radius
        self.cached_bounding_radius *= s;
    }

    /// Get the rotation matrix to orient the fabric so faces with Downwards(n) point down
    pub fn down_rotation(&self, brick_role: BrickRole) -> Matrix4<f32> {
        let downward_count = match brick_role {
            BrickRole::Seed(n) => n,
            _ => panic!("Brick role {:?} is not a seed", brick_role),
        };
        let downward_normals: Vec<_> = self
            .faces
            .values()
            .filter_map(|face| {
                face.aliases
                    .iter()
                    .find(|alias| alias.face_name == FaceName::Downwards(downward_count))
                    .map(|_| face.normal(self))
            })
            .collect();
        if downward_normals.len() != downward_count {
            panic!(
                "{:?} but found {} downward faces",
                brick_role,
                downward_normals.len()
            );
        }
        let down = downward_normals.into_iter().sum::<Vector3<f32>>().normalize();
        Matrix4::from(Quaternion::between_vectors(down, -Vector3::unit_y()))
    }

    /// Zero out all joint velocities and forces
    /// Useful when freezing the fabric to prevent accumulated velocity artifacts
    pub fn zero_velocities(&mut self) {
        for joint in self.joints.iter_mut() {
            joint.velocity = zero();
            joint.force = zero();
        }
    }

    pub fn slacken(&mut self) {
        for interval_opt in self.intervals.iter_mut().filter(|i| i.is_some()) {
            let interval = interval_opt.as_mut().unwrap();
            if !interval.has_role(Role::Support) {
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

    /// Set pretensing target for push intervals
    ///
    /// Extends push intervals by the specified percentage of their rest length.
    /// For example, with pretenst=1%, a 100mm push interval will target 101mm.
    ///
    /// Note: During the pretensing phase, the physics simulation continues to run,
    /// so actual extensions may vary slightly from the target due to forces from
    /// pull intervals and other structural dynamics.
    pub fn set_pretenst(&mut self, pretenst: Percent, seconds: Seconds) {
        let factor = pretenst.as_factor();

        for interval_opt in self.intervals.iter_mut().filter(|i| i.is_some()) {
            let interval = interval_opt.as_mut().unwrap();
            if !interval.has_role(Role::Support) {
                let is_pushing = interval.has_role(Role::Pushing);
                match interval.span {
                    Fixed {
                        length: rest_length,
                    } => {
                        if is_pushing {
                            interval.span = Pretensing {
                                start_length: rest_length,
                                target_length: rest_length * (1.0 + factor),
                                rest_length,
                                finished: false,
                            };
                        }
                    }
                    Pretensing {
                        target_length,
                        rest_length,
                        ..
                    } => {
                        interval.span = Pretensing {
                            start_length: target_length,
                            target_length: rest_length * (1.0 + factor),
                            rest_length,
                            finished: false,
                        }
                    }
                    _ => {}
                }
            }
        }
        self.progress.start(seconds);
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

    pub fn iterate(&mut self, physics: &Physics) -> f32 {
        if self.frozen {
            return 0.0;
        }
        self.stats.reset();
        let ambient_mass = self.ambient_mass();
        for joint in &mut self.joints {
            joint.reset_with_mass(ambient_mass);
        }
        for interval_opt in self.intervals.iter_mut().filter(|i| i.is_some()) {
            let interval = interval_opt.as_mut().unwrap();
            interval.iterate(
                &mut self.joints,
                &self.progress,
                physics,
            );
            self.stats.accumulate_strain(interval.strain);
        }
        let elapsed = self.age.tick();

        // Check for excessive speed and accumulate velocity/energy stats
        // Now in meters: 1000 m/s max speed (still very high, for safety)
        const MAX_SPEED_SQUARED: f32 = 1000.0 * 1000.0; // (m per tick)²
        let mut max_speed_squared = 0.0;

        for joint in self.joints.iter_mut() {
            joint.iterate(physics);
            let speed_squared = joint.velocity.magnitude2();
            let mass = *joint.accumulated_mass;
            self.stats.accumulate_joint(mass, speed_squared);
            self.stats.update_max_speed_squared(speed_squared);
            if speed_squared > max_speed_squared {
                max_speed_squared = speed_squared;
            }
        }
        self.stats.finalize();
        if max_speed_squared > MAX_SPEED_SQUARED || max_speed_squared.is_nan() {
            eprintln!(
                "Excessive speed detected: {:.2} mm/tick - freezing fabric",
                max_speed_squared.sqrt()
            );
            self.zero_velocities();
            self.frozen = true;
            return 0.0;
        }
        if self.progress.step(elapsed) {
            // final step
            for interval_opt in self.intervals.iter_mut().filter(|i| i.is_some()) {
                let interval = interval_opt.as_mut().unwrap();
                match &mut interval.span {
                    Fixed { .. } => {}
                    Pretensing { finished, .. } => {
                        *finished = true;
                    }
                    Approaching { target_length, .. } => {
                        interval.span = Approaching {
                            start_length: *target_length,
                            target_length: *target_length,
                        };
                    }
                }
            }
        }

        elapsed.as_micros() as f32
    }

    pub fn kinetic_energy(&self) -> f32 {
        self.joints
            .iter()
            .map(|joint| {
                let speed_squared = joint.velocity.magnitude2();
                0.5 * *joint.accumulated_mass * speed_squared
            })
            .sum()
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

    /// Returns the cached bounding radius (updated periodically during construction)
    pub fn bounding_radius(&self) -> f32 {
        self.cached_bounding_radius
    }

    /// Calculate the actual bounding radius from joint positions
    fn calculate_bounding_radius(&self) -> f32 {
        if self.joints.is_empty() {
            return 0.0;
        }
        let midpoint = self.midpoint();

        let max_distance_squared = self
            .joints
            .iter()
            .map(|joint| joint.location.distance2(midpoint))
            .fold(0.0_f32, |max, dist_sq| max.max(dist_sq));

        // Add a small margin to ensure everything is visible
        // Only one sqrt call at the end
        max_distance_squared.sqrt() * 1.1
    }

    /// Update the cached bounding radius
    pub fn update_bounding_radius(&mut self) {
        self.cached_bounding_radius = self.calculate_bounding_radius();
    }

    /// Returns (min_y, max_y)
    pub fn altitude_range(&self) -> (f32, f32) {
        self.joints
            .iter()
            .map(|joint| joint.location.y)
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), y| {
                (min.min(y), max.max(y))
            })
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
        let mut push_range = (Meters(1000.0), Meters(0.0));
        let mut pull_range = (Meters(1000.0), Meters(0.0));
        let mut push_count = 0;
        let mut push_total = Meters(0.0);
        let mut pull_count = 0;
        let mut pull_total = Meters(0.0);
        for interval_opt in self.intervals.iter().filter(|i| i.is_some()) {
            let interval = interval_opt.as_ref().unwrap();
            let length = Meters(interval.length(&self.joints));
            if !interval.has_role(Role::Support) {
                // Categorize by push-like vs pull-like behavior
                if interval.role == Role::Pushing {
                    push_count += 1;
                    push_total = push_total + length;
                    if *length < *push_range.0 {
                        push_range.0 = length;
                    }
                    if *length > *push_range.1 {
                        push_range.1 = length;
                    }
                } else if interval.role.is_pull_like() {
                    pull_count += 1;
                    pull_total = pull_total + length;
                    if *length < *pull_range.0 {
                        pull_range.0 = length;
                    }
                    if *length > *pull_range.1 {
                        pull_range.1 = length;
                    }
                }
                // Springy and other roles are ignored in stats
            }
        }
        let (_, max_y) = self.altitude_range();
        FabricStats {
            name: self.name.clone(),
            age: self.age,
            joint_count: self.joints.len(),
            height: Meters(max_y),
            push_count,
            push_range,
            push_total,
            pull_count,
            pull_range,
            pull_total,
        }
    }

    /// Calculate total mass from intervals using current physics
    /// This is done on-demand rather than cached, so it always reflects current physics.mass_scale
    fn calculate_total_mass(&self, physics: &Physics) -> Grams {
        let mut total_mass = Grams(0.0);

        // Add ambient mass for each joint
        total_mass += AMBIENT_MASS * self.joints.len() as f32;

        // Add mass from each interval
        for interval_opt in &self.intervals {
            if let Some(interval) = interval_opt {
                let alpha = &self.joints[interval.alpha_index];
                let omega = &self.joints[interval.omega_index];
                let real_length = Meters((&omega.location - &alpha.location).magnitude());
                let interval_mass = interval.material.linear_density(physics) * real_length;
                total_mass += interval_mass;
            }
        }

        total_mass
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Default, Hash, Eq, Ord, PartialOrd)]
pub struct UniqueId(pub usize);
