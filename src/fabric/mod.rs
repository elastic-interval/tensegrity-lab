/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::build::dsl::brick_dsl::{BrickRole, FaceName};
use crate::fabric::face::Face;
use crate::fabric::interval::Span::Fixed;
use crate::fabric::interval::SpanTransition;
use crate::fabric::interval::{Interval, Role};
use crate::fabric::joint::Joint;
use crate::fabric::physics::Physics;
use crate::units::{Grams, Meters, Unit};
use crate::Age;
use glam::{Mat4, Quat, Vec3};
use slotmap::{new_key_type, SlotMap};
use std::fmt::Debug;

#[derive(Clone, Debug)]
pub struct IntervalReading {
    pub interval_key: IntervalKey,
    pub role: Role,
    pub strain: f32,
    pub actual_length: Meters,
    pub ideal_length: Meters,
    pub unit_vector: Vec3,
    pub alpha_position: Vec3,
    pub alpha_velocity: Vec3,
    pub omega_position: Vec3,
    pub omega_velocity: Vec3,
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
pub mod bend_optimizer;
pub mod brick;
pub mod dimensions;
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
pub mod physics_tester;

// Re-export so `crate::fabric::HingeDimensions` and `crate::fabric::FabricDimensions`
// keep working from outside this module.
pub use dimensions::{hinge_angle, FabricDimensions, HingeDimensions};

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
    pub mass_kg: f32,
    pub max_pull_force_kn: f32,
    pub push_strain_range: (f32, f32),
    pub pull_strain_range: (f32, f32),
    pub slack_pull_count: usize,
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
            dimensions: FabricDimensions::default(),
            approaching_count: 0,
        }
    }

    pub fn with_dimensions(mut self, dimensions: FabricDimensions) -> Self {
        self.dimensions = dimensions;
        self
    }

    pub fn scale(&self) -> f32 {
        self.dimensions.scale.f32()
    }

    pub fn ambient_mass(&self) -> Grams {
        self.dimensions.joint_mass
    }

    /// Update `self.dimensions.hinge.bend_magnitudes` with the K-center
    /// optimal set for this fabric's cable ends. No-op when locked, K=0, or no pulls.
    pub fn recompute_bend_magnitudes(&mut self) {
        if self.dimensions.hinge.bend_magnitudes_locked {
            return;
        }
        let k = self.dimensions.hinge.bend_count;
        if k == 0 {
            return;
        }
        let ideals = self.collect_ideal_bend_angles();
        if ideals.is_empty() {
            return;
        }
        self.dimensions.hinge.bend_magnitudes =
            bend_optimizer::optimize_magnitudes(&ideals, k);
    }

    /// Continuous ideal bend angle (degrees) at every cable end.
    pub fn collect_ideal_bend_angles(&self) -> Vec<f32> {
        let mut ideals = Vec::new();
        for (_key, push_interval) in self.intervals.iter() {
            if !push_interval.has_role(Role::Pushing) {
                continue;
            }
            let alpha_pos = self.joints[push_interval.alpha_key].location;
            let omega_pos = self.joints[push_interval.omega_key].location;
            let push_dir = (omega_pos - alpha_pos).normalize();

            for interval_end in [IntervalEnd::Alpha, IntervalEnd::Omega] {
                let (end_pos, axis_dir, end_key) = match interval_end {
                    IntervalEnd::Alpha => (alpha_pos, -push_dir, push_interval.alpha_key),
                    IntervalEnd::Omega => (omega_pos, push_dir, push_interval.omega_key),
                };
                let Some(connections) = push_interval.connections(interval_end) else {
                    continue;
                };
                for (slot_idx, conn_opt) in connections.iter().enumerate() {
                    let Some(connection) = conn_opt else { continue };
                    let Some(pull_interval) = self.intervals.get(connection.pull_interval_key)
                    else {
                        continue;
                    };
                    let pull_other_end = if pull_interval.alpha_key == end_key {
                        self.joints[pull_interval.omega_key].location
                    } else {
                        self.joints[pull_interval.alpha_key].location
                    };
                    let (_, _, _, ideal_deg) = self.dimensions.hinge_geometry(
                        end_pos,
                        axis_dir,
                        slot_idx,
                        pull_other_end,
                    );
                    ideals.push(ideal_deg);
                }
            }
        }
        ideals
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
        let down: Vec3 = downward_normals.into_iter().sum::<Vec3>().normalize();
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
    /// Push intervals are snapped first, then pulls have ideal length extended.
    /// `pull_lengthening` extends pull ideal lengths (e.g., 0.01 = 1% longer), giving slack room.
    pub fn slacken(&mut self, pull_lengthening: f32) {
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
        // Second pass: set pull intervals with slack allowance
        // Lengthen ideal so pulls start slack; push adjustments will take up the slack
        for interval in self.intervals.values_mut() {
            if !interval.has_role(Role::Pushing) && !interval.has_role(Role::Support) {
                let current_length = interval.fast_length(&self.joints);
                let ideal_length = current_length * (1.0 + pull_lengthening);
                interval.span = Fixed {
                    length: Meters(ideal_length),
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

    /// Velocity Verlet integration using kick-drift-kick formulation
    pub fn iterate(&mut self, physics: &Physics) -> f32 {
        if self.frozen {
            return 0.0;
        }

        use crate::units::EARTH_GRAVITY;

        self.stats.reset();
        let dt = Age::iteration_duration();
        let has_gravity = physics.surface.is_some();

        // 1. First half-kick: v += 0.5 * a * dt (using forces from previous iteration)
        //    Note: On first iteration, forces are zero, so this is a no-op
        for joint in self.joints.values_mut() {
            joint.half_kick(dt);
        }

        // 2. Drift: x += v * dt (position update)
        for joint in self.joints.values_mut() {
            joint.drift(dt);
        }

        // 3. Reset forces and recalculate at new positions
        let ambient_mass = self.ambient_mass();
        for joint in self.joints.values_mut() {
            joint.reset_with_mass(ambient_mass);
        }

        // Calculate interval forces (also adds interval mass to joints)
        let age = self.age;
        let dimensions = &self.dimensions;
        for interval in self.intervals.values_mut() {
            if interval.iterate(&mut self.joints, age, physics, dimensions) == SpanTransition::ApproachCompleted
            {
                self.approaching_count = self.approaching_count.saturating_sub(1);
            }
            self.stats.accumulate_strain(interval.strain);
        }

        // Apply gravity force AFTER interval.iterate so accumulated_mass includes interval mass
        if has_gravity {
            let g = EARTH_GRAVITY.f32();
            for joint in self.joints.values_mut() {
                let mass = joint.accumulated_mass.f32();
                joint.force += Vec3::new(0.0, -mass * g, 0.0);
            }
        }

        // 4. Second half-kick: v += 0.5 * a * dt (using new forces)
        //    Also apply damping and surface interaction after the velocity update
        const MAX_SPEED_SQUARED: f32 = 1000.0 * 1000.0; // (m/s)²
        let mut max_speed_squared = 0.0;

        for joint in self.joints.values_mut() {
            joint.half_kick(dt);
            joint.apply_damping_and_surface(physics, dt);

            let speed_squared = joint.velocity.length_squared();
            let mass = joint.accumulated_mass.f32();
            self.stats.accumulate_joint(mass, speed_squared);
            self.stats.update_max_speed_squared(speed_squared);
            if speed_squared > max_speed_squared {
                max_speed_squared = speed_squared;
            }
        }

        let elapsed = self.age.tick();
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

    /// Calculate elastic potential energy stored in all intervals
    /// E = 0.5 * k * x² where x = strain * ideal_length
    pub fn potential_energy(&self, physics: &Physics) -> f32 {
        self.intervals
            .values()
            .map(|interval| {
                let strain = interval.strain;
                let ideal = interval.ideal();
                let k = interval.material.spring_constant(ideal, physics);
                // Extension in meters
                let extension = Meters(strain * ideal.f32());
                // E = 0.5 * k * x²
                0.5 * k.f32() * extension.f32() * extension.f32()
            })
            .sum()
    }

    /// Total mechanical energy (kinetic + potential)
    pub fn total_energy(&self, physics: &Physics) -> f32 {
        self.kinetic_energy() + self.potential_energy(physics)
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

    pub fn fabric_stats(&self, physics: &Physics) -> FabricStats {
        let mut push_range = (Meters(1000.0), Meters(0.0));
        let mut pull_range = (Meters(1000.0), Meters(0.0));
        let mut push_count = 0;
        let mut push_total = Meters(0.0);
        let mut pull_count = 0;
        let mut pull_total = Meters(0.0);
        let mut push_strain_min = f32::MAX;
        let mut push_strain_max = f32::MIN;
        let mut pull_strain_min = f32::MAX;
        let mut pull_strain_max = f32::MIN;
        let mut max_pull_force_kn = 0.0f32;
        let mut slack_pull_count = 0usize;

        const SLACK_THRESHOLD: f32 = 0.0001;

        for interval in self.intervals.values() {
            let length = Meters(interval.length(&self.joints));
            if !interval.has_role(Role::Support) {
                if interval.role == Role::Pushing {
                    push_count += 1;
                    push_total = push_total + length;
                    if length < push_range.0 {
                        push_range.0 = length;
                    }
                    if length > push_range.1 {
                        push_range.1 = length;
                    }
                    push_strain_min = push_strain_min.min(interval.strain);
                    push_strain_max = push_strain_max.max(interval.strain);
                } else if interval.role.is_pull_like() {
                    pull_count += 1;
                    pull_total = pull_total + length;
                    if length < pull_range.0 {
                        pull_range.0 = length;
                    }
                    if length > pull_range.1 {
                        pull_range.1 = length;
                    }
                    pull_strain_min = pull_strain_min.min(interval.strain);
                    pull_strain_max = pull_strain_max.max(interval.strain);
                    if interval.strain < SLACK_THRESHOLD {
                        slack_pull_count += 1;
                    }
                    let k_real = interval.material.real_spring_constant_at_1m();
                    let force_kn = k_real.0 * interval.strain.abs() / 1000.0;
                    max_pull_force_kn = max_pull_force_kn.max(force_kn);
                }
            }
        }
        let (_, max_y) = self.altitude_range();
        let mass_kg = self.calculate_total_mass(physics).0 / 1000.0;

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
            mass_kg,
            max_pull_force_kn,
            push_strain_range: (push_strain_min, push_strain_max),
            pull_strain_range: (pull_strain_min, pull_strain_max),
            slack_pull_count,
        }
    }

    /// Calculate total mass from intervals using current physics
    /// This is done on-demand rather than cached, so it always reflects current physics.mass_scale
    fn calculate_total_mass(&self, physics: &Physics) -> Grams {
        let mut total_mass = Grams(0.0);

        // Connector head + per-joint hardware share, once per joint.
        total_mass += self.dimensions.joint_mass * self.joints.len() as f32;

        let mut pulling_count: usize = 0;
        for interval in self.intervals.values() {
            let alpha = &self.joints[interval.alpha_key];
            let omega = &self.joints[interval.omega_key];
            let real_length = Meters((omega.location - alpha.location).length());
            total_mass +=
                self.dimensions.linear_density(interval.material, physics) * real_length;

            // Telescoping inner tubes inside each push strut: total length
            // ≈ one full outer-tube length per strut. Reported here (not
            // included in the integrator's per-interval mass).
            if interval.role == Role::Pushing {
                total_mass += self.dimensions.inner_push_density * real_length;
            }
            if interval.role == Role::Pulling {
                pulling_count += 1;
            }
        }

        // Cable terminations: one fork-and-thread at each end of every cable.
        total_mass += self.dimensions.pull_end_mass * (2.0 * pulling_count as f32);

        total_mass
    }

    /// Find the interval connecting two joints, if one exists.
    pub fn interval_between(&self, a: JointKey, b: JointKey) -> Option<(IntervalKey, &Interval)> {
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

    pub fn interval_reading(&self, key: IntervalKey) -> Option<IntervalReading> {
        let interval = self.intervals.get(key)?;
        let alpha = self.joints.get(interval.alpha_key)?;
        let omega = self.joints.get(interval.omega_key)?;
        let ideal = interval.ideal().0;
        let actual = (omega.location - alpha.location).length();
        let strain = if ideal > 0.0 {
            (actual - ideal) / ideal
        } else {
            0.0
        };
        let unit = if actual > 0.0 {
            (omega.location - alpha.location) / actual
        } else {
            Vec3::Y
        };
        Some(IntervalReading {
            interval_key: key,
            role: interval.role,
            strain,
            actual_length: Meters(actual),
            ideal_length: Meters(ideal),
            unit_vector: unit,
            alpha_position: alpha.location,
            alpha_velocity: alpha.velocity,
            omega_position: omega.location,
            omega_velocity: omega.velocity,
        })
    }
}

#[cfg(test)]
mod hinge_geometry_tests {
    use super::*;
    use glam::Vec3;

    const MM: f32 = 1000.0;
    const TOL: f32 = 0.001; // 1 micron tolerance

    fn assert_mm(label: &str, actual_m: f32, expected_mm: f32) {
        let actual_mm = actual_m * MM;
        let diff = (actual_mm - expected_mm).abs();
        assert!(
            diff < TOL * MM,
            "{}: expected {:.3}mm, got {:.3}mm (diff {:.4}mm)",
            label, expected_mm, actual_mm, diff
        );
    }

    /// Test that HingeDimensions formulas produce the correct derived values.
    /// These are the numbers shown in the CSV header as "Afgeleide waarden".
    #[test]
    fn hinge_dimension_formulas() {
        let h = HingeDimensions::default();
        let a = h.push_radius.f32();      // 25mm
        let b = h.push_radius_margin.f32(); // 2mm
        let t1 = h.disc_thickness.f32();   // 6mm
        let t2 = h.disc_separator_thickness.f32(); // 1mm
        let cap = h.cap_thickness.f32();   // 6mm
        let d = h.hinge_extension.f32();   // 14mm
        let e = h.hinge_hole_diameter.f32(); // 12mm
        let c = t1 / 2.0;                 // 3mm

        // offset() = A + B + C (radial distance from tube axis to hinge bolt)
        assert_mm("offset = A+B+C", h.offset().f32(), (a + b + c) * MM);

        // length() = C + D + E (hinge length from disc center to cable endpoint)
        assert_mm("length = C+D+E", h.length().f32(), (c + d + e) * MM);

        // disc_center_offset(0) = cap + t2 + t1/2
        assert_mm(
            "disc_center_offset(0) = cap+t2+t1/2",
            h.disc_center_offset(0).f32(),
            (cap + t2 + c) * MM,
        );

        // disc_center_offset(1) = disc_center_offset(0) + t1 + t2
        assert_mm(
            "disc_center_offset(1) - offset(0) = t1+t2",
            h.disc_center_offset(1).f32() - h.disc_center_offset(0).f32(),
            (t1 + t2) * MM,
        );

        // Print summary for engineer verification
        println!("\n=== Hinge dimension check (mm) ===");
        println!("A  (push_radius):       {:.1}", a * MM);
        println!("B  (margin):            {:.1}", b * MM);
        println!("C  (t1/2):              {:.1}", c * MM);
        println!("D  (hinge_extension):   {:.1}", d * MM);
        println!("E  (hole_diameter):     {:.1}", e * MM);
        println!("t1 (disc_thickness):    {:.1}", t1 * MM);
        println!("t2 (disc_separator):    {:.1}", t2 * MM);
        println!("cap_thickness:          {:.1}", cap * MM);
        println!();
        println!("A + B + C = offset():           {:.1}", h.offset().f32() * MM);
        println!("C + D + E = length():           {:.1}", h.length().f32() * MM);
        println!("t1 + t2:                        {:.1}", (t1 + t2) * MM);
        println!("disc_center_offset(0):          {:.1}", h.disc_center_offset(0).f32() * MM);
        println!("disc_center_offset(1):          {:.1}", h.disc_center_offset(1).f32() * MM);
    }

    /// Test that the 3D positions produced by ring_center / hinge_geometry
    /// have the exact distances the engineer expects to measure between them.
    #[test]
    fn hinge_geometry_distances() {
        let dims = FabricDimensions::default();
        let h = &dims.hinge;

        // Synthetic push interval along +Z axis
        let push_end = Vec3::ZERO;
        let push_axis = Vec3::Z;
        // Pull cable going roughly radially outward in +X
        let pull_other_end = Vec3::new(1.0, 0.0, 0.2);

        // --- Axial distances (along push axis) ---

        let rc0 = dims.ring_center(push_end, push_axis, 0);
        let rc1 = dims.ring_center(push_end, push_axis, 1);
        let rc2 = dims.ring_center(push_end, push_axis, 2);

        // Push end to first disc center
        let axial_0 = (rc0 - push_end).length();
        assert_mm("push_end → ring_center(0)", axial_0, h.disc_center_offset(0).f32() * MM);

        // Between consecutive disc centers = t1 + t2
        let disc_step = (rc1 - rc0).length();
        assert_mm("ring_center(0) → ring_center(1) = t1+t2", disc_step,
                  (h.disc_thickness.f32() + h.disc_separator_thickness.f32()) * MM);

        let disc_step_2 = (rc2 - rc1).length();
        assert_mm("ring_center(1) → ring_center(2) = t1+t2", disc_step_2,
                  (h.disc_thickness.f32() + h.disc_separator_thickness.f32()) * MM);

        // --- Radial distance (ring center to hinge bolt) ---

        let (hinge_pos, _bend, pull_end_pos, _ideal) =
            dims.hinge_geometry(push_end, push_axis, 0, pull_other_end);

        let radial_dist = (hinge_pos - rc0).length();
        assert_mm("ring_center → hinge_pos = offset() = A+B+C", radial_dist,
                  h.offset().f32() * MM);

        // --- Hinge length (hinge bolt to cable endpoint) = C + D + E ---

        let hinge_len = (pull_end_pos - hinge_pos).length();
        assert_mm("hinge_pos → pull_end_pos = length() = C+D+E", hinge_len,
                  h.length().f32() * MM);

        // Print summary for engineer
        println!("\n=== Geometry distance check (mm) ===");
        println!("push_end → ring_center(0):     {:.3}", axial_0 * MM);
        println!("ring_center(0) → ring_center(1): {:.3} (= t1+t2)", disc_step * MM);
        println!("ring_center → hinge_pos:       {:.3} (= A+B+C = offset)", radial_dist * MM);
        println!("hinge_pos → pull_end_pos:      {:.3} (= C+D+E = length)", hinge_len * MM);
    }
}
