/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::build::dsl::brick_dsl::{BrickRole, FaceName};
use crate::fabric::face::Face;
use crate::fabric::interval::Span::Fixed;
use crate::fabric::interval::SpanTransition;
use crate::fabric::interval::{Interval, Role};
use crate::fabric::joint::{Joint, AMBIENT_MASS};
use crate::fabric::physics::Physics;
use crate::units::{Degrees, Grams, Meters, Unit};
use crate::Age;
use glam::{Mat4, Quat, Vec3};
use slotmap::{new_key_type, SlotMap};
use std::fmt::Debug;

/// Hinge geometry dimensions for physical construction.
#[derive(Clone, Copy, Debug)]
pub struct HingeDimensions {
    pub push_radius: Meters,
    pub push_radius_margin: Meters,
    pub disc_thickness: Meters,
    pub disc_separator_thickness: Meters,
    pub hinge_extension: Meters,
    pub hinge_hole_diameter: Meters,
}

impl Default for HingeDimensions {
    fn default() -> Self {
        Self {
            push_radius: Meters(0.030),
            push_radius_margin: Meters(0.003),
            disc_thickness: Meters(0.010),
            disc_separator_thickness: Meters(0.003),
            hinge_extension: Meters(0.012),
            hinge_hole_diameter: Meters(0.017),
        }
    }
}

impl HingeDimensions {

    pub fn offset(&self) -> Meters {
        self.push_radius + self.push_radius_margin + self.disc_thickness / 2.0
    }

    pub fn length(&self) -> Meters {
        self.disc_thickness / 2.0 + self.hinge_extension + self.hinge_hole_diameter
    }
}

const NEAR_PARALLEL_THRESHOLD: f32 = 1e-10;
const AXIS_ALIGNMENT_THRESHOLD: f32 = 0.9;

pub fn hinge_angle(push_axis: Vec3, pull_direction: Vec3) -> Degrees {
    let sin_angle = pull_direction.dot(push_axis);
    Degrees(sin_angle.asin().to_degrees())
}

fn radial_unit_from_axis(push_axis: Vec3, direction: Vec3) -> Vec3 {
    let axial_component = push_axis * direction.dot(push_axis);
    let radial_direction = direction - axial_component;

    if radial_direction.length_squared() < NEAR_PARALLEL_THRESHOLD {
        let arbitrary = if push_axis.x.abs() < AXIS_ALIGNMENT_THRESHOLD {
            Vec3::new(1.0, 0.0, 0.0)
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        };
        push_axis.cross(arbitrary).normalize()
    } else {
        radial_direction.normalize()
    }
}

/// All physical dimensions for a fabric: structure size and interval geometry.
#[derive(Clone, Copy, Debug)]
pub struct FabricDimensions {
    pub altitude: Meters,
    pub scale: Meters,
    pub pull_radius: Meters,
    pub hinge: HingeDimensions,
    pub push_length_increment: Option<Meters>,
    pub max_pretenst_strain: Option<f32>,
}

impl Default for FabricDimensions {
    fn default() -> Self {
        Self {
            altitude: Meters(7.5),
            scale: Meters(1.0),
            pull_radius: Meters(0.007),
            hinge: HingeDimensions::default(),
            push_length_increment: Some(Meters(0.025)),
            max_pretenst_strain: Some(0.03),
        }
    }
}

impl FabricDimensions {

    pub fn with_altitude(mut self, altitude: Meters) -> Self {
        self.altitude = altitude;
        self
    }

    pub fn with_scale(mut self, scale: Meters) -> Self {
        self.scale = scale;
        self
    }

    fn ring_center(&self, push_end: Vec3, push_axis: Vec3, slot: usize) -> Vec3 {
        let axial_offset = self.hinge.disc_thickness.f32() * (slot as f32 + 1.0);
        push_end + push_axis * axial_offset
    }

    pub fn hinge_position(
        &self,
        push_end: Vec3,
        push_axis: Vec3,
        slot: usize,
        pull_other_end: Vec3,
    ) -> Vec3 {
        let (hinge_pos, _, _) = self.hinge_geometry(push_end, push_axis, slot, pull_other_end);
        hinge_pos
    }

    pub fn hinge_geometry(
        &self,
        push_end: Vec3,
        push_axis: Vec3,
        slot: usize,
        pull_other_end: Vec3,
    ) -> (Vec3, attachment::HingeBend, Vec3) {
        let ring_center = self.ring_center(push_end, push_axis, slot);
        let to_pull = pull_other_end - ring_center;
        let radial_unit = radial_unit_from_axis(push_axis, to_pull);

        let hinge_pos = ring_center + radial_unit * self.hinge.offset().f32();

        let pull_direction = (pull_other_end - hinge_pos).normalize();
        let sin_angle = pull_direction.dot(push_axis);
        let ideal_angle = Degrees(sin_angle.asin().to_degrees());
        let hinge_bend = attachment::HingeBend::from_angle(ideal_angle);

        let pull_end_pos = hinge_bend.endpoint(hinge_pos, push_axis, radial_unit, self.hinge.length().f32());

        (hinge_pos, hinge_bend, pull_end_pos)
    }

    /// Snap a length to the nearest increment (minimum 1 increment).
    pub fn snap_push_length(&self, length: f32) -> f32 {
        match self.push_length_increment {
            Some(increment) => {
                let inc = increment.f32();
                let snapped = (length / inc).round() * inc;
                snapped.max(inc)
            }
            None => length,
        }
    }

    /// Calculate discrete target length for pretensing based on target strain.
    ///
    /// If `max_pretenst_strain` is set and even 1 increment would exceed that
    /// strain for this strut, returns rest_length (no extension).
    pub fn discrete_pretenst_target(&self, rest_length: f32, target_strain: f32) -> f32 {
        match self.push_length_increment {
            Some(increment) => {
                let inc = increment.f32();

                // Check if even 1 increment would exceed max strain
                if let Some(max_strain) = self.max_pretenst_strain {
                    let one_increment_strain = inc / rest_length;
                    if one_increment_strain > max_strain {
                        // Skip extension for this short strut
                        return rest_length;
                    }
                }

                let min_extension = rest_length * target_strain;
                let num_increments = (min_extension / inc).ceil().max(0.0) as u32;
                rest_length + (num_increments as f32) * inc
            }
            None => rest_length * (1.0 + target_strain),
        }
    }
}

new_key_type! {
    /// Key for joints in the fabric's SlotMap
    pub struct JointKey;
}

new_key_type! {
    /// Key for intervals in the fabric's SlotMap
    pub struct IntervalKey;
}

new_key_type! {
    /// Key for faces in the fabric's SlotMap
    pub struct FaceKey;
}

pub mod attachment;
pub mod brick;
pub mod error;
pub mod fabric_sampler;
pub mod face;
pub mod interval;
pub mod joint;
pub mod joint_path;
pub mod material;
pub mod physics;
pub mod vulcanize;

pub mod csv_export;
pub mod physics_test;

// Type aliases for SlotMap containers
pub type Joints = SlotMap<JointKey, Joint>;
pub type Intervals = SlotMap<IntervalKey, Interval>;
pub type Faces = SlotMap<FaceKey, Face>;

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    pub joints: SlotMap<JointKey, Joint>,
    pub intervals: SlotMap<IntervalKey, Interval>,
    pub faces: SlotMap<FaceKey, Face>,
    pub frozen: bool,
    pub stats: IterationStats,
    pub dimensions: FabricDimensions,

    cached_bounding_radius: f32,
    scale: f32,
    approaching_count: usize,
}

impl Fabric {
    pub fn new(name: String) -> Self {
        Self {
            name,
            age: Age::default(),
            joints: SlotMap::with_key(),
            intervals: SlotMap::with_key(),
            faces: SlotMap::with_key(),
            frozen: false,
            stats: IterationStats::default(),
            cached_bounding_radius: 0.0,
            scale: 1.0,
            dimensions: FabricDimensions::default(),
            approaching_count: 0,
        }
    }

    pub fn with_dimensions(mut self, dimensions: FabricDimensions) -> Self {
        self.dimensions = dimensions;
        self
    }

    /// Returns the fabric's scale factor (set during construction)
    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn ambient_mass(&self) -> Grams {
        // Mass scales with scale^3.5: volume (scale³) plus slight reduction for small structures
        Grams(AMBIENT_MASS.f32() * self.scale.powf(3.5))
    }

    pub fn apply_matrix4(&mut self, matrix: Mat4) {
        for joint in self.joints.values_mut() {
            joint.location = matrix.transform_point3(joint.location);
            joint.velocity = matrix.transform_vector3(joint.velocity);
        }
    }

    /// Calculate the translation needed to centralize the fabric
    pub fn centralize_translation(&self, altitude: Option<f32>) -> Vec3 {
        let mut midpoint: Vec3 = Vec3::ZERO;
        for joint in self.joints.values() {
            midpoint += joint.location;
        }
        midpoint /= self.joints.len() as f32;
        midpoint.y = 0.0;

        let mut total_translation = -midpoint;

        // Calculate altitude adjustment if specified
        if let Some(altitude) = altitude {
            let min_y = self
                .joints
                .values()
                .map(|joint| joint.location.y)
                .min_by(|a, b| a.partial_cmp(b).unwrap());
            if let Some(min_y) = min_y {
                let altitude_adjustment = min_y - altitude;
                total_translation.y -= altitude_adjustment;
            }
        }

        total_translation
    }

    /// Apply a translation to all joints
    pub fn apply_translation(&mut self, translation: Vec3) {
        for joint in self.joints.values_mut() {
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
        let s = scale.f32();
        self.scale = s;
        let mass_scale = s.powf(3.5); // scale^3.5: volume plus slight reduction for small structures
                                      // Scale all joint positions, velocities, and mass
        for joint in self.joints.values_mut() {
            joint.location.x *= s;
            joint.location.y *= s;
            joint.location.z *= s;
            joint.velocity.x *= s;
            joint.velocity.y *= s;
            joint.velocity.z *= s;
            // Forces will be recalculated on next iteration
            joint.force = Vec3::ZERO;
            // Scale mass with volume (thinner intervals at small scale)
            joint.accumulated_mass = Grams(joint.accumulated_mass.f32() * mass_scale);
        }
        // Scale all interval ideal lengths
        for interval in self.intervals.values_mut() {
            interval.scale_lengths(s);
        }
        // Scale face scales
        for face in self.faces.values_mut() {
            face.scale *= s;
        }
        // Scale cached bounding radius
        self.cached_bounding_radius *= s;
    }

    /// Get the rotation matrix to orient the fabric so faces with Downwards(n) point down
    pub fn down_rotation(&self, brick_role: BrickRole) -> Mat4 {
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
        let down: Vec3 = downward_normals
            .into_iter()
            .sum::<Vec3>()
            .normalize();
        Mat4::from_quat(Quat::from_rotation_arc(down, -Vec3::Y))
    }

    /// Zero out all joint velocities and forces
    /// Useful when freezing the fabric to prevent accumulated velocity artifacts
    pub fn zero_velocities(&mut self) {
        for joint in self.joints.values_mut() {
            joint.velocity = Vec3::ZERO;
            joint.force = Vec3::ZERO;
        }
    }

    /// Slacken all intervals by setting their span to Fixed at their current length.
    /// Push intervals are snapped first, then pulls are measured to compensate.
    pub fn slacken(&mut self) {
        // First pass: snap push intervals to discrete lengths
        for interval in self.intervals.values_mut() {
            if interval.has_role(Role::Pushing) {
                let current_length = interval.fast_length(&self.joints);
                let snapped_length = self.dimensions.snap_push_length(current_length);
                interval.span = Fixed {
                    length: Meters(snapped_length),
                };
            }
        }
        // Second pass: set pull intervals to their current measured length
        for interval in self.intervals.values_mut() {
            if !interval.has_role(Role::Pushing) && !interval.has_role(Role::Support) {
                let current_length = interval.fast_length(&self.joints);
                interval.span = Fixed {
                    length: Meters(current_length),
                };
            }
        }
        for joint in self.joints.values_mut() {
            joint.force = Vec3::ZERO;
            joint.velocity = Vec3::ZERO;
        }
    }

    pub fn max_velocity(&self) -> f32 {
        self.joints
            .values()
            .map(|joint| joint.velocity.length_squared())
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|speed_sq| speed_sq.sqrt())
            .unwrap_or(0.0)
    }

    pub fn failed_intervals(&self, strain_limit: f32) -> Vec<IntervalKey> {
        self.intervals
            .iter()
            .filter_map(|(key, interval)| {
                if interval.strain > strain_limit {
                    Some(key)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn iterate(&mut self, physics: &Physics) -> f32 {
        if self.frozen {
            return 0.0;
        }
        self.stats.reset();
        let ambient_mass = self.ambient_mass();
        for joint in self.joints.values_mut() {
            joint.reset_with_mass(ambient_mass);
        }
        let age = self.age;
        for interval in self.intervals.values_mut() {
            if interval.iterate(&mut self.joints, age, physics) == SpanTransition::ApproachCompleted {
                self.approaching_count = self.approaching_count.saturating_sub(1);
            }
            self.stats.accumulate_strain(interval.strain);
        }
        let elapsed = self.age.tick();

        // Check for excessive speed and accumulate velocity/energy stats
        const MAX_SPEED_SQUARED: f32 = 1000.0 * 1000.0; // (m/s)²
        let mut max_speed_squared = 0.0;

        for joint in self.joints.values_mut() {
            joint.iterate(physics);
            let speed_squared = joint.velocity.length_squared();
            let mass = joint.accumulated_mass.f32();
            self.stats.accumulate_joint(mass, speed_squared);
            self.stats.update_max_speed_squared(speed_squared);
            if speed_squared > max_speed_squared {
                max_speed_squared = speed_squared;
            }
        }
        self.stats.finalize();
        if max_speed_squared > MAX_SPEED_SQUARED || max_speed_squared.is_nan() {
            eprintln!(
                "Excessive speed detected: {:.2} m/s - freezing fabric",
                max_speed_squared.sqrt()
            );
            self.zero_velocities();
            self.frozen = true;
            return 0.0;
        }

        elapsed.as_micros() as f32
    }

    /// Check if any intervals are still approaching their target length
    pub fn has_approaching_intervals(&self) -> bool {
        self.approaching_count > 0
    }

    pub fn kinetic_energy(&self) -> f32 {
        self.joints
            .values()
            .map(|joint| {
                let speed_squared = joint.velocity.length_squared();
                0.5 * joint.accumulated_mass.f32() * speed_squared
            })
            .sum()
    }

    pub fn centroid(&self) -> Vec3 {
        let mut centroid: Vec3 = Vec3::ZERO;
        for joint in self.joints.values() {
            centroid += joint.location;
        }
        let denominator = if self.joints.is_empty() {
            1
        } else {
            self.joints.len()
        } as f32;
        centroid / denominator
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
        let centroid = self.centroid();

        let max_distance_squared = self
            .joints
            .values()
            .map(|joint| joint.location.distance_squared(centroid))
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
            .values()
            .map(|joint| joint.location.y)
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), y| {
                (min.min(y), max.max(y))
            })
    }

    pub fn check_orphan_joints(&self) {
        for (joint_key, _) in self.joints.iter() {
            let touching = self
                .interval_values()
                .any(|interval| interval.touches(joint_key));
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
        for interval in self.intervals.values() {
            let length = Meters(interval.length(&self.joints));
            if !interval.has_role(Role::Support) {
                // Categorize by push-like vs pull-like behavior
                if interval.role == Role::Pushing {
                    push_count += 1;
                    push_total = push_total + length;
                    if length < push_range.0 {
                        push_range.0 = length;
                    }
                    if length > push_range.1 {
                        push_range.1 = length;
                    }
                } else if interval.role.is_pull_like() {
                    pull_count += 1;
                    pull_total = pull_total + length;
                    if length < pull_range.0 {
                        pull_range.0 = length;
                    }
                    if length > pull_range.1 {
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
        for interval in self.intervals.values() {
            let alpha = &self.joints[interval.alpha_key];
            let omega = &self.joints[interval.omega_key];
            let real_length = Meters((omega.location - alpha.location).length());
            let interval_mass = interval.material.linear_density(physics) * real_length;
            total_mass += interval_mass;
        }

        total_mass
    }

    /// Find the interval connecting two joints, if one exists.
    pub fn interval_between(
        &self,
        a: JointKey,
        b: JointKey,
    ) -> Option<(IntervalKey, &Interval)> {
        self.intervals
            .iter()
            .find(|(_, interval)| interval.connects(a, b))
    }

    /// Find the strut (push interval) connected to a joint, if one exists.
    pub fn push_at(&self, joint: JointKey) -> Option<IntervalKey> {
        self.intervals
            .iter()
            .find(|(_, interval)| interval.role == Role::Pushing && interval.touches(joint))
            .map(|(key, _)| key)
    }
}

/// Geometric analysis results for buildability assessment
#[derive(Debug, Clone, Default)]
pub struct GeometricAnalysis {
    /// Average pull connections per joint
    pub avg_pull_connections: f32,
    /// Minimum pull connections on any joint
    pub min_pull_connections: usize,
    /// Maximum pull connections on any joint
    pub max_pull_connections: usize,
    /// Number of overpopulated joints (>5 pulls)
    pub overpopulated_joints: usize,
    /// Number of underpopulated joints (<3 pulls)
    pub underpopulated_joints: usize,
    /// Minimum distance between non-adjacent push pairs (meters)
    pub min_push_distance: f32,
    /// Number of crossing push pairs (distance < crossing_threshold)
    pub crossing_count: usize,
    /// Number of near-miss push pairs (distance < near_miss_threshold but >= crossing_threshold)
    pub near_miss_count: usize,
}

impl GeometricAnalysis {
    /// Score from 0.0 (poor) to 1.0 (excellent) based on buildability.
    pub fn buildability_score(&self) -> f32 {
        // Crossing penalty: each crossing halves the score
        let crossing_penalty = 0.5_f32.powi(self.crossing_count as i32);

        // Overpopulation penalty: each overpopulated joint reduces by 10%
        let overpop_penalty = 0.9_f32.powi(self.overpopulated_joints as i32);

        // Underpopulation penalty: each underpopulated joint reduces by 10%
        let underpop_penalty = 0.9_f32.powi(self.underpopulated_joints as i32);

        crossing_penalty * overpop_penalty * underpop_penalty
    }
}

/// Geometric analysis methods for buildability assessment
impl Fabric {
    /// Count pull connections per joint, returning a map from joint key to count.
    pub fn pull_connections_per_joint(&self) -> std::collections::HashMap<JointKey, usize> {
        use std::collections::HashMap;
        let mut counts: HashMap<JointKey, usize> = HashMap::new();

        for joint_key in self.joints.keys() {
            counts.insert(joint_key, 0);
        }

        for interval in self.intervals.values() {
            if interval.role.is_pull_like() {
                *counts.entry(interval.alpha_key).or_insert(0) += 1;
                *counts.entry(interval.omega_key).or_insert(0) += 1;
            }
        }

        counts
    }

    /// Calculate minimum distances between all non-adjacent push pairs.
    /// Returns distances in meters, sorted ascending.
    pub fn push_distances(&self) -> Vec<f32> {
        use crate::build::evo::geometry::segment_min_distance;

        let push_intervals: Vec<_> = self
            .intervals
            .iter()
            .filter(|(_, interval)| interval.has_role(Role::Pushing))
            .collect();

        let mut distances = Vec::new();

        for i in 0..push_intervals.len() {
            for j in (i + 1)..push_intervals.len() {
                let (_, int1) = push_intervals[i];
                let (_, int2) = push_intervals[j];

                // Skip if they share a joint (adjacent pushes)
                if int1.alpha_key == int2.alpha_key
                    || int1.alpha_key == int2.omega_key
                    || int1.omega_key == int2.alpha_key
                    || int1.omega_key == int2.omega_key
                {
                    continue;
                }

                let p1 = self.location(int1.alpha_key);
                let q1 = self.location(int1.omega_key);
                let p2 = self.location(int2.alpha_key);
                let q2 = self.location(int2.omega_key);

                let distance = segment_min_distance(p1, q1, p2, q2);
                distances.push(distance);
            }
        }

        distances.sort_by(|a, b| a.partial_cmp(b).unwrap());
        distances
    }

    /// Perform full geometric analysis for buildability assessment.
    ///
    /// Parameters:
    /// - `crossing_threshold`: Distance below which pushes are crossing (meters), default 0.01 (10mm)
    /// - `near_miss_threshold`: Distance below which pushes are dangerously close (meters), default 0.05 (50mm)
    pub fn geometric_analysis(
        &self,
        crossing_threshold: f32,
        near_miss_threshold: f32,
    ) -> GeometricAnalysis {
        let pull_counts = self.pull_connections_per_joint();

        let total: usize = pull_counts.values().sum();
        let avg = if pull_counts.is_empty() {
            0.0
        } else {
            total as f32 / pull_counts.len() as f32
        };
        let min = *pull_counts.values().min().unwrap_or(&0);
        let max = *pull_counts.values().max().unwrap_or(&0);

        let overpopulated = pull_counts.values().filter(|&&c| c > 5).count();
        let underpopulated = pull_counts.values().filter(|&&c| c < 3).count();

        let distances = self.push_distances();
        let min_distance = distances.first().copied().unwrap_or(f32::MAX);

        let crossing_count = distances.iter().filter(|&&d| d < crossing_threshold).count();
        let near_miss_count = distances
            .iter()
            .filter(|&&d| d >= crossing_threshold && d < near_miss_threshold)
            .count();

        GeometricAnalysis {
            avg_pull_connections: avg,
            min_pull_connections: min,
            max_pull_connections: max,
            overpopulated_joints: overpopulated,
            underpopulated_joints: underpopulated,
            min_push_distance: min_distance,
            crossing_count,
            near_miss_count,
        }
    }

    /// Convenience method for geometric analysis with default thresholds.
    /// Uses 10mm for crossing, 50mm for near-miss.
    pub fn analyze_buildability(&self) -> GeometricAnalysis {
        self.geometric_analysis(0.010, 0.050)
    }
}
