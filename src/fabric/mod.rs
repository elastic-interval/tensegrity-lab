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
use crate::units::{Degrees, Grams, Meters, Percent, Seconds};
use crate::Age;
use cgmath::num_traits::zero;
use cgmath::{
    EuclideanSpace, InnerSpace, Matrix4, MetricSpace, Point3, Quaternion, Rotation, Transform,
    Vector3,
};
use slotmap::{new_key_type, SlotMap};
use std::fmt::Debug;
use std::ops::Deref;

/// All physical dimensions for a fabric: structure size and interval geometry.
#[derive(Clone, Copy, Debug)]
pub struct FabricDimensions {
    pub altitude: Meters,
    pub scale: Meters,
    pub push_radius: Meters, // A default 30mm
    pub pull_radius: Meters,
    pub push_radius_margin: Meters,       // B default 3mm
    pub disc_thickness: Meters,           // default 10mm, note that C is half disc_thickness
    pub disc_separator_thickness: Meters, // default 3mm
    pub hinge_extension: Meters,          // D
    pub hinge_hole_diameter: Meters,      // E
    pub push_length_increment: Option<Meters>,
    pub max_pretenst_strain: Option<f32>,
}

impl FabricDimensions {
    /// Full-size dimensions for real structures (scale 1.0m, altitude 7.5m)
    pub fn full_size() -> Self {
        Self {
            altitude: Meters(7.5),
            scale: Meters(1.0),
            push_radius: Meters(0.030),                 // A: 30mm
            pull_radius: Meters(0.007),                 // 7mm
            push_radius_margin: Meters(0.003),          // B: 3mm
            disc_thickness: Meters(0.010),              // 10mm (C = 5mm)
            disc_separator_thickness: Meters(0.003),    // 3mm
            hinge_extension: Meters(0.012),             // D: 12mm
            hinge_hole_diameter: Meters(0.017),         // E: 17mm
            push_length_increment: Some(Meters(0.025)), // 25mm holes
            max_pretenst_strain: Some(0.03), // 3% max - skip extension if 1 increment exceeds this
        }
    }

    /// Model-size dimensions for small physical models (scale 0.056m, altitude 0.5m)
    pub fn model_size() -> Self {
        Self {
            altitude: Meters(0.5),
            scale: Meters(0.056),
            push_radius: Meters(0.003),               // A: 3mm
            pull_radius: Meters(0.0005),              // 0.5mm
            push_radius_margin: Meters(0.0003),       // B: 0.3mm
            disc_thickness: Meters(0.001),            // 1mm (C = 0.5mm)
            disc_separator_thickness: Meters(0.0003), // 0.3mm
            hinge_extension: Meters(0.0012),          // D: 1.2mm
            hinge_hole_diameter: Meters(0.0017),      // E: 1.7mm
            push_length_increment: None,
            max_pretenst_strain: None, // No limit for models (continuous pretensing)
        }
    }

    /// Hinge offset from center: A + B + C (push_radius + push_radius_margin + half disc_thickness)
    pub fn hinge_offset(&self) -> Meters {
        Meters(*self.push_radius + *self.push_radius_margin + *self.disc_thickness / 2.0)
    }

    /// Hinge length: C + D + E (half disc_thickness + hinge_extension + hinge_hole_diameter)
    pub fn hinge_length(&self) -> Meters {
        Meters(*self.disc_thickness / 2.0 + *self.hinge_extension + *self.hinge_hole_diameter)
    }

    /// Set custom altitude
    pub fn with_altitude(mut self, altitude: Meters) -> Self {
        self.altitude = altitude;
        self
    }

    /// Set custom scale
    pub fn with_scale(mut self, scale: Meters) -> Self {
        self.scale = scale;
        self
    }

    /// Calculate the hinge position for a pull interval connection
    pub fn hinge_position(
        &self,
        push_end: Point3<f32>,
        push_axis: Vector3<f32>,
        slot: usize,
        pull_other_end: Point3<f32>,
    ) -> Point3<f32> {
        let axial_offset = *self.disc_thickness * (slot as f32 + 1.0);
        let ring_center = push_end + push_axis * axial_offset;

        let to_pull = pull_other_end - ring_center;
        let axial_component = push_axis * to_pull.dot(push_axis);
        let radial_direction = to_pull - axial_component;

        let radial_unit = if radial_direction.magnitude2() < 1e-10 {
            let arbitrary = if push_axis.x.abs() < 0.9 {
                Vector3::new(1.0, 0.0, 0.0)
            } else {
                Vector3::new(0.0, 1.0, 0.0)
            };
            push_axis.cross(arbitrary).normalize()
        } else {
            radial_direction.normalize()
        };

        ring_center + radial_unit * *self.hinge_offset()
    }

    /// Calculate the ideal hinge angle for a pull interval at its connection point
    pub fn hinge_angle(push_axis: Vector3<f32>, pull_direction: Vector3<f32>) -> Degrees {
        let sin_angle = pull_direction.dot(push_axis);
        Degrees(sin_angle.asin().to_degrees())
    }

    /// Calculate hinge position, snapped angle, and endpoint for a pull interval connection
    pub fn hinge_geometry(
        &self,
        push_end: Point3<f32>,
        push_axis: Vector3<f32>,
        slot: usize,
        pull_other_end: Point3<f32>,
    ) -> (Point3<f32>, attachment::HingeBend, Point3<f32>) {
        let axial_offset = *self.disc_thickness * (slot as f32 + 1.0);
        let ring_center = push_end + push_axis * axial_offset;

        let to_pull = pull_other_end - ring_center;
        let axial_component = push_axis * to_pull.dot(push_axis);
        let radial_direction = to_pull - axial_component;

        let radial_unit = if radial_direction.magnitude2() < 1e-10 {
            let arbitrary = if push_axis.x.abs() < 0.9 {
                Vector3::new(1.0, 0.0, 0.0)
            } else {
                Vector3::new(0.0, 1.0, 0.0)
            };
            push_axis.cross(arbitrary).normalize()
        } else {
            radial_direction.normalize()
        };

        let hinge_pos = ring_center + radial_unit * *self.hinge_offset();

        let pull_direction = (pull_other_end - hinge_pos).normalize();
        let ideal_angle = Self::hinge_angle(push_axis, pull_direction);
        let hinge_bend = attachment::HingeBend::from_angle(ideal_angle);

        let pull_end_pos =
            hinge_bend.endpoint(hinge_pos, push_axis, radial_unit, *self.hinge_length());

        (hinge_pos, hinge_bend, pull_end_pos)
    }

    /// Snap a length to the nearest increment (minimum 1 increment).
    pub fn snap_push_length(&self, length: f32) -> f32 {
        match self.push_length_increment {
            Some(increment) => {
                let inc = *increment;
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
                let inc = *increment;

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
pub mod joint_incident;
pub mod material;
pub mod physics;
pub mod progress;
pub mod vulcanize;

pub mod csv_export;
pub mod physics_test;

// Type aliases for physics quantities - improves readability and provides
// hook for future unit-aware types without cluttering code with f32 generics
pub type Location = Point3<f32>;
pub type Velocity = Vector3<f32>;
pub type Force = Vector3<f32>;

// Type aliases for SlotMap containers
pub type Joints = SlotMap<JointKey, Joint>;
pub type Intervals = SlotMap<IntervalKey, Interval>;
pub type Faces = SlotMap<FaceKey, Face>;

/// A numerical identifier for joints, used for display (e.g., "J0", "J1")
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct JointId(pub usize);

impl std::fmt::Display for JointId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "J{}", self.0)
    }
}

impl Deref for JointId {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

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
    pub progress: Progress,
    pub joints: SlotMap<JointKey, Joint>,
    /// Maps JointId (numerical index) to JointKey for DSL resolution
    pub joint_by_id: Vec<JointKey>,
    pub intervals: SlotMap<IntervalKey, Interval>,
    pub faces: SlotMap<FaceKey, Face>,
    pub frozen: bool,
    pub stats: IterationStats,
    cached_bounding_radius: f32,
    scale: f32,
    pub dimensions: FabricDimensions,
}

impl Fabric {
    pub fn new(name: String) -> Self {
        Self {
            name,
            age: Age::default(),
            progress: Progress::default(),
            joints: SlotMap::with_key(),
            joint_by_id: Vec::new(),
            intervals: SlotMap::with_key(),
            faces: SlotMap::with_key(),
            frozen: false,
            stats: IterationStats::default(),
            cached_bounding_radius: 0.0,
            scale: 1.0,
            dimensions: FabricDimensions::model_size(),
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
        Grams(*AMBIENT_MASS * self.scale.powf(3.5))
    }

    pub fn apply_matrix4(&mut self, matrix: Matrix4<f32>) {
        for joint in self.joints.values_mut() {
            joint.location = matrix.transform_point(joint.location);
            joint.velocity = matrix.transform_vector(joint.velocity);
        }
    }

    /// Calculate the translation needed to centralize the fabric
    pub fn centralize_translation(&self, altitude: Option<f32>) -> Vector3<f32> {
        let mut midpoint: Vector3<f32> = zero();
        for joint in self.joints.values() {
            midpoint += joint.location.to_vec();
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
    pub fn apply_translation(&mut self, translation: Vector3<f32>) {
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
        let s = *scale;
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
            joint.force = cgmath::num_traits::zero();
            // Scale mass with volume (thinner intervals at small scale)
            joint.accumulated_mass = Grams(*joint.accumulated_mass * mass_scale);
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
        let down = downward_normals
            .into_iter()
            .sum::<Vector3<f32>>()
            .normalize();
        Matrix4::from(Quaternion::between_vectors(down, -Vector3::unit_y()))
    }

    /// Zero out all joint velocities and forces
    /// Useful when freezing the fabric to prevent accumulated velocity artifacts
    pub fn zero_velocities(&mut self) {
        for joint in self.joints.values_mut() {
            joint.velocity = zero();
            joint.force = zero();
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
                    length: snapped_length,
                };
            }
        }
        // Second pass: set pull intervals to their current measured length
        for interval in self.intervals.values_mut() {
            if !interval.has_role(Role::Pushing) && !interval.has_role(Role::Support) {
                let current_length = interval.fast_length(&self.joints);
                interval.span = Fixed {
                    length: current_length,
                };
            }
        }
        for joint in self.joints.values_mut() {
            joint.force = zero();
            joint.velocity = zero();
        }
    }

    /// Set pretensing target for push intervals (non-holistic, uses fabric dimensions).
    pub fn set_pretenst(&mut self, pretenst: Percent, seconds: Seconds) {
        let target_strain = pretenst.as_factor();

        for interval in self.intervals.values_mut() {
            if !interval.has_role(Role::Support) {
                let is_pushing = interval.has_role(Role::Pushing);
                match interval.span {
                    Fixed {
                        length: rest_length,
                    } => {
                        if is_pushing {
                            let target_length = self
                                .dimensions
                                .discrete_pretenst_target(rest_length, target_strain);
                            interval.span = Pretensing {
                                start_length: rest_length,
                                target_length,
                                rest_length,
                                finished: false,
                            };
                        }
                    }
                    Pretensing {
                        target_length: current_target,
                        rest_length,
                        ..
                    } => {
                        let target_length = self
                            .dimensions
                            .discrete_pretenst_target(rest_length, target_strain);
                        interval.span = Pretensing {
                            start_length: current_target,
                            target_length,
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
        self.joints
            .values()
            .map(|joint| joint.velocity.magnitude2())
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
        for interval in self.intervals.values_mut() {
            interval.iterate(&mut self.joints, &self.progress, physics);
            self.stats.accumulate_strain(interval.strain);
        }
        let elapsed = self.age.tick();

        // Check for excessive speed and accumulate velocity/energy stats
        // Now in meters: 1000 m/s max speed (still very high, for safety)
        const MAX_SPEED_SQUARED: f32 = 1000.0 * 1000.0; // (m per tick)²
        let mut max_speed_squared = 0.0;

        for joint in self.joints.values_mut() {
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
            for interval in self.intervals.values_mut() {
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
            .values()
            .map(|joint| {
                let speed_squared = joint.velocity.magnitude2();
                0.5 * *joint.accumulated_mass * speed_squared
            })
            .sum()
    }

    pub fn midpoint(&self) -> Point3<f32> {
        let mut midpoint: Point3<f32> = Point3::origin();
        for joint in self.joints.values() {
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
            .values()
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
        for interval in self.intervals.values() {
            let alpha = &self.joints[interval.alpha_key];
            let omega = &self.joints[interval.omega_key];
            let real_length = Meters((&omega.location - &alpha.location).magnitude());
            let interval_mass = interval.material.linear_density(physics) * real_length;
            total_mass += interval_mass;
        }

        total_mass
    }
}
