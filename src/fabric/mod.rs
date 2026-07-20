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
use crate::units::{Grams, Meters, Percent, Seconds, Unit};
use crate::Age;
use glam::{Mat4, Quat, Vec3};
use slotmap::{new_key_type, SlotMap};
use std::fmt::Debug;
use std::sync::Arc;

/// Plug-in for rendering joint identifiers as human-meaningful labels.
/// Implementations may inspect the fabric and a joint key; returning `Some`
/// supplies the label. Returning `None` defers to the default formatting
/// (the `JointPath`'s `Display` impl). Installed on a `Fabric` only when its
/// construction path knows how to provide one — algorithmic fabrics with no
/// installed labeller use the default formatting throughout.
pub trait JointLabeller: Debug + Send + Sync {
    fn label(&self, fabric: &Fabric, key: JointKey) -> Option<String>;
}

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

pub mod physics_tester;

// Re-export so `crate::fabric::ConnectorDimensions` and `crate::fabric::FabricDimensions`
// keep working from outside this module. Connector code lives in `crate::connector`.
pub use crate::connector::{
    attachment, bend_optimizer, tab_angle, ConnectorDimensions, ConnectorSystem,
};
pub use dimensions::FabricDimensions;

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
    /// Physical connector hardware, present only when a large-scale build is
    /// intended. `None` means connectors play no role anywhere.
    pub connector: Option<ConnectorSystem>,
    pub labeller: Option<Arc<dyn JointLabeller>>,

    cached_bounding_radius: f32,
    approaching_count: usize,
    /// Age at which the fabric's max velocity first fell below the
    /// quiet-freeze threshold. Reset to `None` whenever motion exceeds the
    /// threshold or any interval is still approaching its target length.
    /// When the elapsed time since this anchor reaches
    /// `QUIET_FREEZE_DURATION_SECS`, the fabric is auto-frozen.
    quiet_since: Option<Age>,
}

impl Fabric {
    pub fn new(name: String) -> Self {
        let mut dimensions = FabricDimensions::default();
        let connector = dimensions.connector.take().map(ConnectorSystem::new);
        Self {
            name,
            age: Age::default(),
            joints: SlotMap::with_key(),
            intervals: SlotMap::with_key(),
            faces: SlotMap::with_key(),
            frozen: false,
            stats: IterationStats::default(),
            cached_bounding_radius: 0.0,
            dimensions,
            connector,
            labeller: None,
            approaching_count: 0,
            quiet_since: None,
        }
    }

    pub fn with_dimensions(mut self, mut dimensions: FabricDimensions) -> Self {
        self.connector = dimensions.connector.take().map(ConnectorSystem::new);
        self.dimensions = dimensions;
        self
    }

    pub fn scale(&self) -> f32 {
        self.dimensions.scale.f32()
    }

    pub fn joint_label(&self, joint_key: JointKey) -> String {
        let Some(joint) = self.joints.get(joint_key) else {
            return String::new();
        };
        if let Some(labeller) = &self.labeller {
            if let Some(label) = labeller.label(self, joint_key) {
                return label;
            }
        }
        joint.path.to_string()
    }

    pub fn ambient_mass(&self) -> Grams {
        self.dimensions.joint_mass
    }

    /// Update the connector's bend magnitudes with the K-center optimal set
    /// for this fabric's cable ends. No-op without a connector, when locked,
    /// K=0, or no pulls.
    pub fn recompute_bend_magnitudes(&mut self) {
        let Some(mut connector) = self.connector.take() else {
            return;
        };
        connector.recompute_bend_magnitudes(self);
        self.connector = Some(connector);
    }

    /// Rebuild the connector's slot assignments for every push interval from
    /// current geometry. No-op without a connector.
    pub fn update_all_attachment_connections(&mut self) {
        let Some(mut connector) = self.connector.take() else {
            return;
        };
        connector.update_all_connections(self);
        self.connector = Some(connector);
    }

    /// Continuous ideal bend angle (degrees) at every cable end.
    /// Empty without a connector.
    pub fn collect_ideal_bend_angles(&self) -> Vec<f32> {
        self.connector
            .as_ref()
            .map(|connector| connector.collect_ideal_bend_angles(self))
            .unwrap_or_default()
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

    /// Allow iterations to resume after the fabric was frozen — either by
    /// the excessive-speed guard or by the quiet-time auto-freeze. Call this
    /// when transitioning into a state that needs the physics step to run
    /// again (animation, physics testing, jump-style relocations).
    pub fn unfreeze(&mut self) {
        self.frozen = false;
        self.quiet_since = None;
    }

    /// Zero out all joint velocities and forces
    /// Useful when freezing the fabric to prevent accumulated velocity artifacts
    pub fn zero_velocities(&mut self) {
        for joint in self.joints.values_mut() {
            joint.velocity = Vec3::ZERO;
            joint.force = Vec3::ZERO;
        }
    }

    /// Freeze every non-Support interval at its current geometric length
    /// (zero strain everywhere) and clear all joint forces/velocities. The
    /// subsequent `set_pretenst` call grows push rest-lengths from here.
    pub fn slacken(&mut self) {
        use crate::fabric::interval::Span;
        let mut still_approaching = 0usize;
        for interval in self.intervals.values_mut() {
            if !interval.has_role(Role::Support) {
                if matches!(interval.span, Span::Approaching { .. }) {
                    still_approaching += 1;
                }
                interval.span = Fixed {
                    length: Meters(interval.fast_length(&self.joints)),
                };
            }
        }
        self.approaching_count = self.approaching_count.saturating_sub(still_approaching);
        for joint in self.joints.values_mut() {
            joint.force = Vec3::ZERO;
            joint.velocity = Vec3::ZERO;
        }
    }

    /// Begin pretensing: each push interval extends from its current rest
    /// length to `length × (1 + percent_as_factor)` smoothly over `seconds`
    /// of fabric time, via an `Approaching` span. Pulls stretch passively
    /// as the joints separate, building tension.
    pub fn set_pretenst(&mut self, pretenst: Percent, seconds: Seconds) {
        use crate::fabric::interval::Span::Approaching;
        let factor = pretenst.as_factor();
        let start_age = self.age;
        for interval in self.intervals.values_mut() {
            if !interval.has_role(Role::Pushing) {
                continue;
            }
            if let Fixed { length } = interval.span {
                interval.span = Approaching {
                    start_length: length,
                    target_length: length * (1.0 + factor),
                    start_age,
                    duration: seconds,
                };
                self.approaching_count += 1;
            }
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

        // Auto-freeze when the structure has been quiet for a sustained span
        // of fabric time, so background simulation in Viewing (etc.) doesn't
        // keep grinding once equilibrium is reached. `Approaching` intervals
        // are still progressing toward their target length, so we hold off
        // while any of those are in flight. The 50 mm/s threshold is chosen
        // above the residual max-velocity floor a meter-scale OpenClaw
        // exhibits in Viewing physics (~20-35 mm/s of joint jitter from
        // imperfectly cancelled interval forces); see commit notes.
        const QUIET_MAX_SPEED_SQ: f32 = 2.5e-3; // (5e-2 m/s)² = (50 mm/s)²
        const QUIET_FREEZE_DURATION_SECS: f32 = 10.0;
        if self.approaching_count > 0 || max_speed_squared > QUIET_MAX_SPEED_SQ {
            self.quiet_since = None;
        } else {
            match self.quiet_since {
                None => self.quiet_since = Some(self.age),
                Some(start) => {
                    if self.age.elapsed_since(start).0 >= QUIET_FREEZE_DURATION_SECS {
                        eprintln!(
                            "Fabric quiet for {}s of fabric time — freezing iterations",
                            QUIET_FREEZE_DURATION_SECS as u32
                        );
                        self.frozen = true;
                    }
                }
            }
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
