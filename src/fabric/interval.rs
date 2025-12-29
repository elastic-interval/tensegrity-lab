/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::fabric::attachment::{
    calculate_interval_attachment_points, find_nearest_attachment_point, AttachmentPoint,
    PullConnection, PullConnections, PullIntervalData, ATTACHMENT_POINTS,
};
use crate::fabric::error::FabricError;
use crate::fabric::interval::Role::*;
use crate::fabric::interval::Span::*;
use crate::fabric::joint::Joint;
use crate::fabric::material::Material;
use crate::fabric::physics::Physics;
use crate::fabric::vulcanize::VulcanizeMode;
use crate::fabric::FabricDimensions;
use crate::fabric::{Fabric, IntervalEnd, IntervalKey, JointKey, Joints};
use crate::units::{Meters, NewtonsPerMeter, Percent, Seconds, Unit};
use crate::Age;
use crate::Appearance;
use fast_inv_sqrt::InvSqrt32;
use glam::Vec3;

impl Fabric {
    /// Update the attachment connections for a specific push interval
    /// This finds all pull intervals connected to the push interval and assigns them
    /// to their nearest attachment points
    pub fn update_interval_attachment_connections(&mut self, push_interval_key: IntervalKey) {
        // First collect information about the push interval and connected pull intervals
        let mut connected_pulls = Vec::new();
        let mut pull_data = Vec::new();

        // Get the push interval and check if it's a push interval
        if let Some(push_interval) = self.intervals.get(push_interval_key) {
            if !push_interval.has_role(Pushing) {
                return; // Not a push interval, nothing to do
            }

            let push_alpha = push_interval.alpha_key;
            let push_omega = push_interval.omega_key;

            // Find all pull intervals connected to this push interval
            for (key, interval) in self.intervals.iter() {
                // Only consider pull-like intervals (all tension-only types)
                if !interval.role.is_pull_like() {
                    continue;
                }

                // Check if this pull interval is connected to the push interval
                if interval.alpha_key == push_alpha
                    || interval.alpha_key == push_omega
                    || interval.omega_key == push_alpha
                    || interval.omega_key == push_omega
                {
                    connected_pulls.push((key, interval.alpha_key, interval.omega_key));

                    // Collect pull interval data for moment calculation
                    pull_data.push(PullIntervalData {
                        key,
                        alpha_key: interval.alpha_key,
                        omega_key: interval.omega_key,
                        strain: interval.strain,
                        unit: interval.unit,
                    });
                }
            }
        }

        // Now update the attachment connections if we have a valid push interval
        if let Some(push_interval) = self.intervals.get_mut(push_interval_key) {
            // Use the new reorder_connections method to optimize attachment points
            let _ = push_interval.reorder_connections(
                &self.joints,
                &connected_pulls,
                &pull_data,
                &self.dimensions,
            );
        }
    }

    /// Update attachment connections for all push intervals in the fabric
    /// This is typically called at the end of the pretenst phase
    pub fn update_all_attachment_connections(&mut self) {
        // Skip if there are no joints
        if self.joints.is_empty() {
            return;
        }

        // Find all push interval keys
        let push_interval_keys: Vec<IntervalKey> = self
            .intervals
            .iter()
            .filter_map(|(key, interval)| {
                if interval.has_role(Pushing) {
                    Some(key)
                } else {
                    None
                }
            })
            .collect();

        // Update connections for each push interval
        for push_key in push_interval_keys {
            self.update_interval_attachment_connections(push_key);
        }
    }

    fn create_interval(
        &mut self,
        alpha_key: JointKey,
        omega_key: JointKey,
        role: Role,
        span: Span,
    ) -> IntervalKey {
        let interval = Interval::new(alpha_key, omega_key, role, span);

        let key = self.intervals.insert(interval);

        // If we added a pull-like interval, update connections for any push intervals it might connect to
        if role != Pushing && role != Springy {
            // Find all push intervals connected to this pull interval
            let push_intervals: Vec<IntervalKey> = self
                .intervals
                .iter()
                .filter_map(|(k, interval)| {
                    if interval.has_role(Pushing)
                        && (interval.touches(alpha_key) || interval.touches(omega_key))
                    {
                        Some(k)
                    } else {
                        None
                    }
                })
                .collect();

            // Update connections for each connected push interval
            for push_key in push_intervals {
                self.update_interval_attachment_connections(push_key);
            }
        }

        key
    }

    /// Create an interval that approaches a target length over a duration
    pub fn create_approaching_interval(
        &mut self,
        alpha_key: JointKey,
        omega_key: JointKey,
        target_length: Meters,
        role: Role,
        duration: Seconds,
    ) -> IntervalKey {
        let start_length = self.distance(alpha_key, omega_key);
        self.approaching_count += 1;
        self.create_interval(
            alpha_key,
            omega_key,
            role,
            Approaching {
                start_length,
                target_length,
                start_age: self.age,
                duration,
            },
        )
    }

    /// Extend an existing interval to a new target length
    pub fn extend_interval(&mut self, key: IntervalKey, target_length: Meters, duration: Seconds) {
        if let Some(interval) = self.intervals.get_mut(key) {
            let current_length = interval.ideal();
            interval.span = Approaching {
                start_length: current_length,
                target_length,
                start_age: self.age,
                duration,
            };
            self.approaching_count += 1;
        }
    }

    /// Create an interval with a Fixed span at a specific length
    pub fn create_fixed_interval(
        &mut self,
        alpha_key: JointKey,
        omega_key: JointKey,
        role: Role,
        length: Meters,
    ) -> IntervalKey {
        self.create_interval(alpha_key, omega_key, role, Fixed { length })
    }

    /// Create an interval with a Fixed span at length calculated from strain
    /// Formula: strain = (actual - ideal) / ideal, so ideal = actual / (1 + strain)
    pub fn create_strained_interval(
        &mut self,
        alpha_key: JointKey,
        omega_key: JointKey,
        role: Role,
        strain: f32,
    ) -> IntervalKey {
        let distance = self.distance(alpha_key, omega_key);
        let length = Meters(distance.f32() / (1.0 + strain));
        self.create_fixed_interval(alpha_key, omega_key, role, length)
    }

    /// Create a measuring interval (for vulcanize bow ties)
    pub fn create_measuring_interval(
        &mut self,
        alpha_key: JointKey,
        omega_key: JointKey,
        role: Role,
        contraction: f32,
        mode: VulcanizeMode,
    ) -> IntervalKey {
        let baseline = self.distance(alpha_key, omega_key);
        self.create_interval(
            alpha_key,
            omega_key,
            role,
            Measuring {
                baseline,
                contraction,
                mode,
            },
        )
    }

    /// Create an interval at its current slack length (Fixed span at current distance)
    pub fn create_slack_interval(
        &mut self,
        alpha_key: JointKey,
        omega_key: JointKey,
        role: Role,
    ) -> IntervalKey {
        self.create_fixed_interval(
            alpha_key,
            omega_key,
            role,
            self.distance(alpha_key, omega_key),
        )
    }

    /// Get an interval by its key, returning a Result
    pub fn interval_result(&self, key: IntervalKey) -> Result<&Interval, FabricError> {
        self.intervals.get(key).ok_or(FabricError::IntervalNotFound)
    }

    /// Get an interval by its key
    pub fn interval(&self, key: IntervalKey) -> &Interval {
        self.interval_result(key).expect("Interval not found")
    }

    /// Get an interval snapshot by its key, returning a Result
    pub fn interval_snapshot_result(
        &self,
        key: IntervalKey,
    ) -> Result<IntervalSnapshot, FabricError> {
        let interval = self.interval_result(key)?;

        // Get joints by key - SlotMap returns None if key is invalid
        let alpha = self
            .joints
            .get(interval.alpha_key)
            .ok_or(FabricError::InvalidJointIndices)?
            .clone();
        let omega = self
            .joints
            .get(interval.omega_key)
            .ok_or(FabricError::InvalidJointIndices)?
            .clone();

        Ok(IntervalSnapshot {
            interval: interval.clone(),
            alpha,
            omega,
        })
    }

    /// Get an interval snapshot by its key
    pub fn interval_snapshot(&self, key: IntervalKey) -> IntervalSnapshot {
        self.interval_snapshot_result(key)
            .expect("Failed to get interval snapshot")
    }

    pub fn remove_interval(&mut self, key: IntervalKey) -> Interval {
        self.intervals
            .remove(key)
            .expect("Removing nonexistent interval")
    }

    pub fn find_push_at(&self, joint_key: JointKey) -> Option<IntervalKey> {
        self.intervals.iter().find_map(|(key, interval)| {
            (interval.is_push_interval() && interval.touches(joint_key)).then_some(key)
        })
    }

    pub fn joining(&self, pair: (JointKey, JointKey)) -> Option<IntervalKey> {
        self.intervals.iter().find_map(|(key, interval)| {
            if interval.touches(pair.0) && interval.touches(pair.1) {
                Some(key)
            } else {
                None
            }
        })
    }

    pub fn interval_values(&self) -> impl Iterator<Item = &Interval> {
        self.intervals.values()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Span {
    Fixed {
        length: Meters,
    },
    Approaching {
        start_length: Meters,
        target_length: Meters,
        start_age: Age,
        duration: Seconds,
    },
    Measuring {
        baseline: Meters,
        contraction: f32,
        mode: VulcanizeMode,
    },
}

impl Span {
    pub fn scale(&mut self, factor: f32) {
        match self {
            Span::Fixed { length } => {
                *length = *length * factor;
            }
            Span::Approaching {
                start_length,
                target_length,
                ..
            } => {
                *start_length = *start_length * factor;
                *target_length = *target_length * factor;
            }
            Span::Measuring { baseline, .. } => {
                *baseline = *baseline * factor;
            }
        }
    }
}

impl Unit for Span {
    fn f32(self) -> f32 {
        match self {
            Span::Fixed { length } => length.f32(),
            Span::Approaching { target_length, .. } => target_length.f32(),
            Span::Measuring { baseline, .. } => baseline.f32(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SpanTransition {
    Unchanged,
    ApproachCompleted,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Role {
    Pushing = 0,
    Pulling = 1,
    Springy = 2,
    Circumference = 3,
    BowTie = 4,
    FaceRadial = 5,
    Support = 6,
    GuyLine = 9,
    PrismPull = 11,
}

impl Role {
    /// Check if this role is equal to another role
    pub fn is(&self, other: Role) -> bool {
        *self == other
    }

    /// Check if this role behaves like a pull interval (tension-only)
    pub fn is_pull_like(&self) -> bool {
        matches!(
            self,
            Pulling | Circumference | BowTie | FaceRadial | Support | GuyLine | PrismPull
        )
    }

    /// Get a label string for this role (for serialization)
    pub fn label(&self) -> &'static str {
        match self {
            Pushing => "push",
            Pulling => "pull",
            Springy => "spring",
            Circumference => "circumference",
            BowTie => "bow-tie",
            FaceRadial => "face-radial",
            Support => "support",
            GuyLine => "guy-line",
            PrismPull => "prism-pull",
        }
    }

    /// Get role from a label string (for deserialization)
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "push" => Some(Pushing),
            "pull" => Some(Pulling),
            "spring" => Some(Springy),
            "circumference" => Some(Circumference),
            "bow-tie" => Some(BowTie),
            "face-radial" => Some(FaceRadial),
            "support" => Some(Support),
            "guy-line" => Some(GuyLine),
            "prism-pull" => Some(PrismPull),
            _ => None,
        }
    }

    /// Get the material for this role
    pub fn material(&self) -> Material {
        match self {
            Pushing => Material::Push,
            Springy => Material::Spring,
            _ => Material::Pull,
        }
    }

    pub fn appearance(&self) -> Appearance {
        Appearance {
            radius: self.radius(),
            color: self.gray_color(),
        }
    }

    pub fn radius(&self) -> f32 {
        match self {
            Pushing => 1.2,
            Pulling | BowTie | Support | GuyLine => 0.14, // 30% thinner than push intervals
            Springy => 0.7,
            Circumference => 0.18, // Also reduced proportionally
            FaceRadial => 0.1,     // Also reduced proportionally
            PrismPull => 0.14,     // Match pulling radius
        }
    }

    fn gray_color(&self) -> [f32; 4] {
        match self {
            Pushing => [0.3, 0.3, 0.3, 1.0],
            Pulling => [0.2, 0.2, 0.2, 1.0],
            Springy => [0.4, 0.4, 0.4, 1.0],
            Circumference => [0.25, 0.25, 0.35, 1.0],
            BowTie => [0.2, 0.25, 0.2, 1.0],
            FaceRadial => [0.15, 0.15, 0.15, 1.0],
            Support => [0.25, 0.2, 0.2, 1.0],
            GuyLine => [0.22, 0.22, 0.22, 1.0],
            PrismPull => [0.2, 0.2, 0.2, 1.0],
        }
    }

    /// Get a distinct color for this role (for color-by-role rendering)
    pub fn color(&self) -> [f32; 4] {
        match self {
            Pushing => [1.0, 1.0, 1.0, 1.0],       // White
            Pulling => [0.3, 0.6, 0.9, 1.0],       // Blue
            Springy => [0.9, 0.7, 0.2, 1.0],       // Yellow/Gold
            Circumference => [1.0, 0.5, 0.5, 1.0], // Pastel bright red
            BowTie => [0.2, 0.8, 0.3, 1.0],        // Green
            FaceRadial => [0.5, 0.5, 0.5, 1.0],    // Gray
            Support => [0.9, 0.5, 0.2, 1.0],       // Orange
            GuyLine => [0.7, 0.7, 0.3, 1.0],       // Olive
            PrismPull => [0.6, 0.4, 0.8, 1.0],     // Purple
        }
    }
}

#[derive(Clone, Debug)]
pub struct Interval {
    pub alpha_key: JointKey,
    pub omega_key: JointKey,
    pub role: Role,
    pub material: Material,
    pub span: Span,
    pub unit: Vec3,
    pub strain: f32,
    pub stiffness: Percent,
    pub connections: Option<Box<PullConnections>>,
}

impl Interval {
    /// Check if the interval has the specified role
    pub fn has_role(&self, role: Role) -> bool {
        self.role.is(role)
    }

    pub fn new(alpha_key: JointKey, omega_key: JointKey, role: Role, span: Span) -> Interval {
        let is_push = role == Pushing;
        let connections = is_push.then_some(Box::new(PullConnections::new()));

        Interval {
            alpha_key,
            omega_key,
            role,
            material: role.material(),
            span,
            unit: Vec3::ZERO,
            strain: 0.0,
            stiffness: Percent(100.0),
            connections,
        }
    }

    /// Check if this is a push interval
    pub fn is_push_interval(&self) -> bool {
        self.has_role(Pushing)
    }

    pub fn is_pull_interval(&self) -> bool {
        self.role.is_pull_like()
    }

    /// Scale all length values by the given factor
    /// Used when converting from internal units to meters
    pub fn scale_lengths(&mut self, factor: f32) {
        self.span.scale(factor);
    }

    /// Get connections for a specific end if this is a push interval
    pub fn connections(
        &self,
        end: IntervalEnd,
    ) -> Option<&[Option<PullConnection>; ATTACHMENT_POINTS]> {
        self.connections.as_ref().map(|conn| conn.connections(end))
    }

    /// Reorder connections to optimize attachment points
    /// This extracts existing connections and reassigns them to optimal attachment points
    pub fn reorder_connections(
        &mut self,
        joints: &Joints,
        pull_intervals: &[(IntervalKey, JointKey, JointKey)],
        pull_data: &[PullIntervalData],
        dimensions: &FabricDimensions,
    ) -> Result<(), FabricError> {
        // Only push intervals have connections to reorder
        if self.role != Pushing {
            return Err(FabricError::NotPushInterval);
        }

        // Get attachment points
        let attachment_points = self.attachment_points(joints, dimensions)?;

        // Reorder connections
        if let Some(conn) = &mut self.connections {
            conn.reorder_connections(
                &attachment_points.0,
                &attachment_points.1,
                joints,
                pull_intervals,
                pull_data,
                self.alpha_key,
                self.omega_key,
            );
        }

        Ok(())
    }

    /// Get attachment points for a push interval at both ends
    /// Returns (alpha_end_points, omega_end_points) as arrays of AttachmentPoint
    /// Returns an error if this is not a push interval
    pub fn attachment_points(
        &self,
        joints: &Joints,
        dimensions: &FabricDimensions,
    ) -> Result<
        (
            [AttachmentPoint; ATTACHMENT_POINTS],
            [AttachmentPoint; ATTACHMENT_POINTS],
        ),
        FabricError,
    > {
        // Only push intervals have attachment points
        if self.role != Pushing {
            return Err(FabricError::NotPushInterval);
        }

        let (alpha_location, omega_location) = self.locations(joints);

        // Calculate attachment points at both ends of the interval
        Ok(calculate_interval_attachment_points(
            alpha_location,
            omega_location,
            dimensions,
        ))
    }

    /// Get a specific attachment point by its index and end
    /// Returns an error if this is not a push interval or if the index is out of bounds
    pub fn get_attachment_point(
        &self,
        joints: &Joints,
        end: IntervalEnd,
        index: usize,
        dimensions: &FabricDimensions,
    ) -> Result<AttachmentPoint, FabricError> {
        if index >= ATTACHMENT_POINTS {
            return Err(FabricError::InvalidAttachmentIndex);
        }

        self.attachment_points(joints, dimensions)
            .map(|points| self.get_point_from_end(points, end, index))
    }

    /// Helper method to get a point from a specific end of the interval
    fn get_point_from_end(
        &self,
        points: (
            [AttachmentPoint; ATTACHMENT_POINTS],
            [AttachmentPoint; ATTACHMENT_POINTS],
        ),
        end: IntervalEnd,
        index: usize,
    ) -> AttachmentPoint {
        let (alpha_points, omega_points) = points;
        match end {
            IntervalEnd::Alpha => alpha_points[index],
            IntervalEnd::Omega => omega_points[index],
        }
    }

    /// Find the nearest attachment point to a given position
    /// Returns an error if this is not a push interval
    pub fn nearest_attachment_point(
        &self,
        joints: &Joints,
        position: Vec3,
        dimensions: &FabricDimensions,
    ) -> Result<(IntervalEnd, AttachmentPoint), FabricError> {
        let (alpha_points, omega_points) = self.attachment_points(joints, dimensions)?;

        // Find the nearest point from each end using the standalone function
        let (alpha_nearest_idx, alpha_nearest_dist) =
            find_nearest_attachment_point(&alpha_points, position);
        let (omega_nearest_idx, omega_nearest_dist) =
            find_nearest_attachment_point(&omega_points, position);

        // Return the nearest point from either end
        if alpha_nearest_dist <= omega_nearest_dist {
            Ok((IntervalEnd::Alpha, alpha_points[alpha_nearest_idx]))
        } else {
            Ok((IntervalEnd::Omega, omega_points[omega_nearest_idx]))
        }
    }

    /// Get the attachment point that is directly opposite to the given attachment point
    /// This would be the point with the same index but at the opposite end
    pub fn opposite_attachment_point(
        &self,
        joints: &Joints,
        end: IntervalEnd,
        index: usize,
        dimensions: &FabricDimensions,
    ) -> Result<AttachmentPoint, FabricError> {
        if index >= ATTACHMENT_POINTS {
            return Err(FabricError::InvalidAttachmentIndex);
        }

        let points = self.attachment_points(joints, dimensions)?;

        // Use the opposite() method from IntervalEnd
        let opposite_end = end.opposite();

        Ok(self.get_point_from_end(points, opposite_end, index))
    }

    pub fn key(&self) -> (JointKey, JointKey) {
        if self.alpha_key < self.omega_key {
            (self.alpha_key, self.omega_key)
        } else {
            (self.omega_key, self.alpha_key)
        }
    }

    /// Get the joint key for a specific end of the interval
    pub fn end_key(&self, end: IntervalEnd) -> JointKey {
        match end {
            IntervalEnd::Alpha => self.alpha_key,
            IntervalEnd::Omega => self.omega_key,
        }
    }

    /// Get the joint location for a specific end of the interval
    pub fn end_location(&self, joints: &Joints, end: IntervalEnd) -> Vec3 {
        joints[self.end_key(end)].location
    }

    pub fn locations(&self, joints: &Joints) -> (Vec3, Vec3) {
        (
            self.end_location(joints, IntervalEnd::Alpha),
            self.end_location(joints, IntervalEnd::Omega),
        )
    }

    pub fn midpoint(&self, joints: &Joints) -> Vec3 {
        let (alpha, omega) = self.locations(joints);
        (alpha + omega) / 2.0
    }

    pub fn fast_length(&mut self, joints: &Joints) -> f32 {
        let (alpha_location, omega_location) = self.locations(joints);
        self.unit = omega_location - alpha_location;
        let magnitude_squared = self.unit.length_squared();
        if magnitude_squared < 0.00001 {
            return 0.00001;
        }
        let inverse_square_root = magnitude_squared.inv_sqrt32();
        self.unit *= inverse_square_root;
        1.0 / inverse_square_root
    }

    pub fn length(&self, joints: &Joints) -> f32 {
        let (alpha_location, omega_location) = self.locations(joints);
        let tween = omega_location - alpha_location;
        let magnitude_squared = tween.length_squared();
        if magnitude_squared < 0.00001 {
            return 0.00001;
        }
        magnitude_squared.sqrt()
    }

    pub fn ideal(&self) -> Meters {
        match self.span {
            Fixed { length, .. }
            | Approaching {
                target_length: length,
                ..
            }
            | Measuring {
                baseline: length, ..
            } => length,
        }
    }

    pub fn iterate(&mut self, joints: &mut Joints, age: Age, physics: &Physics) -> SpanTransition {
        if matches!(self.span, Measuring { .. }) {
            return SpanTransition::Unchanged;
        }
        let mut transition = SpanTransition::Unchanged;
        let ideal = match self.span {
            Fixed { length } => length,
            Approaching {
                start_length,
                target_length,
                start_age,
                duration,
            } => {
                let elapsed = age.elapsed_since(start_age);
                let completion = (elapsed.0 / duration.0).min(1.0);
                if completion >= 1.0 {
                    self.span = Fixed {
                        length: target_length,
                    };
                    transition = SpanTransition::ApproachCompleted;
                    target_length
                } else {
                    Meters(start_length.f32() * (1.0 - completion) + target_length.f32() * completion)
                }
            }
            Measuring { .. } => unreachable!(),
        };
        let real_length = self.fast_length(joints);
        let actual_length = Meters(real_length);

        // Check if interval is slack (push stretched or pull compressed)
        let is_slack = (self.is_push_interval() && real_length > ideal.f32())
            || (self.is_pull_interval() && real_length < ideal.f32());

        // Calculate strain (dimensionless)
        self.strain = if is_slack {
            0.0
        } else {
            (real_length - ideal.f32()) / ideal.f32()
        };

        // Force: F = k × ΔL where ΔL = strain × L₀
        // Spring constant scales with 1/L for proper physics
        // Stiffness percentage allows softer intervals (e.g., actuators at 10%)
        let k = self.material.spring_constant(ideal, physics);
        let k_adjusted = NewtonsPerMeter(k.f32() * self.stiffness.as_factor());
        let extension = Meters(self.strain * ideal.f32());
        let force = k_adjusted * extension;
        let force_vector: Vec3 = self.unit * force.f32() / 2.0;

        // Apply forces to both ends
        let alpha_key = self.end_key(IntervalEnd::Alpha);
        let omega_key = self.end_key(IntervalEnd::Omega);
        joints[alpha_key].force += force_vector;
        joints[omega_key].force -= force_vector;

        // Mass from linear density × length
        let interval_mass = self.material.linear_density(physics) * actual_length;
        let half_mass = interval_mass / 2.0;
        joints[alpha_key].accumulated_mass += half_mass;
        joints[omega_key].accumulated_mass += half_mass;

        transition
    }

    /// Check if this interval touches a specific joint
    pub fn touches(&self, joint_key: JointKey) -> bool {
        self.end_key(IntervalEnd::Alpha) == joint_key
            || self.end_key(IntervalEnd::Omega) == joint_key
    }

    /// Check if this interval connects two specific joints
    pub fn connects(&self, a: JointKey, b: JointKey) -> bool {
        (self.alpha_key == a && self.omega_key == b) || (self.alpha_key == b && self.omega_key == a)
    }

    /// Check if a specific end of this interval touches a joint
    pub fn end_touches(&self, end: IntervalEnd, joint_key: JointKey) -> bool {
        self.end_key(end) == joint_key
    }

    /// Get the end of the interval that corresponds to a given joint key
    pub fn joint_end(&self, joint_key: JointKey) -> IntervalEnd {
        if self.end_key(IntervalEnd::Alpha) == joint_key {
            IntervalEnd::Alpha
        } else if self.end_key(IntervalEnd::Omega) == joint_key {
            IntervalEnd::Omega
        } else {
            panic!("Joint key {:?} is not part of this interval", joint_key)
        }
    }

    /// Get the ray direction from a joint
    pub fn ray_from(&self, joint_key: JointKey) -> Vec3 {
        match self.joint_end(joint_key) {
            IntervalEnd::Alpha => self.unit,
            IntervalEnd::Omega => -self.unit,
        }
    }

    /// Get the key of the joint at the opposite end
    pub fn other_joint(&self, joint_key: JointKey) -> JointKey {
        let end = self.joint_end(joint_key);
        self.end_key(end.opposite())
    }

    pub fn joint_with(
        &self,
        Interval {
            alpha_key,
            omega_key,
            ..
        }: &Interval,
    ) -> Option<JointKey> {
        if self.alpha_key == *alpha_key || self.alpha_key == *omega_key {
            Some(self.alpha_key)
        } else if self.omega_key == *alpha_key || self.omega_key == *omega_key {
            Some(self.omega_key)
        } else {
            None
        }
    }
}

pub struct IntervalSnapshot {
    pub interval: Interval,
    pub alpha: Joint,
    pub omega: Joint,
}

impl IntervalSnapshot {
    pub fn end_key(&self, end: &IntervalEnd) -> JointKey {
        match end {
            IntervalEnd::Alpha => self.interval.alpha_key,
            IntervalEnd::Omega => self.interval.omega_key,
        }
    }
}
