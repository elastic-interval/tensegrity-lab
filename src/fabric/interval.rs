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
use crate::fabric::{Fabric, IntervalEnd, Progress, UniqueId};
use crate::units::Millimeters;
use crate::Appearance;
use cgmath::num_traits::zero;
use cgmath::{EuclideanSpace, InnerSpace, Point3, Vector3};
use fast_inv_sqrt::InvSqrt32;
use std::ops::Mul;

impl Fabric {
    /// Update the attachment connections for a specific push interval
    /// This finds all pull intervals connected to the push interval and assigns them
    /// to their nearest attachment points
    pub fn update_interval_attachment_connections(&mut self, push_interval_id: UniqueId) {
        // First collect information about the push interval and connected pull intervals
        let mut connected_pulls = Vec::new();
        let mut pull_data = Vec::new();

        // Get the push interval and check if it's a push interval
        if let Some(push_interval) = self.intervals[push_interval_id.0].as_ref() {
            if !push_interval.has_role(Pushing) {
                return; // Not a push interval, nothing to do
            }

            let push_alpha = push_interval.alpha_index;
            let push_omega = push_interval.omega_index;

            // Find all pull intervals connected to this push interval
            for (idx, interval_opt) in self.intervals.iter().enumerate() {
                if let Some(interval) = interval_opt {
                    // Only consider pull intervals
                    if !interval.has_role(Pulling) {
                        continue;
                    }

                    // Check if this pull interval is connected to the push interval
                    if interval.alpha_index == push_alpha
                        || interval.alpha_index == push_omega
                        || interval.omega_index == push_alpha
                        || interval.omega_index == push_omega
                    {
                        connected_pulls.push((
                            UniqueId(idx),
                            interval.alpha_index,
                            interval.omega_index,
                        ));

                        // Collect pull interval data for moment calculation
                        pull_data.push(PullIntervalData {
                            id: UniqueId(idx),
                            alpha_joint: interval.alpha_index,
                            omega_joint: interval.omega_index,
                            strain: interval.strain,
                            unit: interval.unit,
                        });
                    }
                }
            }
        }

        // Now update the attachment connections if we have a valid push interval
        if let Some(push_interval) = self.intervals[push_interval_id.0].as_mut() {
            // Use the new reorder_connections method to optimize attachment points
            let _ = push_interval.reorder_connections(&self.joints, &connected_pulls, &pull_data);
        }
    }

    /// Update attachment connections for all push intervals in the fabric
    /// This is typically called at the end of the pretenst phase
    pub fn update_all_attachment_connections(&mut self) {
        // Skip if there are no joints
        if self.joints.is_empty() {
            return;
        }

        // Find all push interval IDs
        let push_interval_ids: Vec<UniqueId> = self
            .intervals
            .iter()
            .enumerate()
            .filter_map(|(idx, interval_opt)| {
                if idx >= self.intervals.len() {
                    return None; // Safety check
                }

                interval_opt.as_ref().and_then(|interval| {
                    if interval.has_role(Pushing) {
                        Some(UniqueId(idx))
                    } else {
                        None
                    }
                })
            })
            .collect();

        // Update connections for each push interval
        for push_id in push_interval_ids {
            // Make sure the interval ID is valid
            if push_id.0 < self.intervals.len() {
                self.update_interval_attachment_connections(push_id);
            }
        }
    }

    /// Create an interval with a specified role
    /// The material is automatically determined from the role
    pub fn create_interval(
        &mut self,
        alpha_index: usize,
        omega_index: usize,
        ideal: f32,
        role: Role,
    ) -> UniqueId {
        let id = self.create_id();
        let interval = Interval::new(
            alpha_index,
            omega_index,
            role,
            Approaching {
                start_length: self.distance(alpha_index, omega_index),
                target_length: ideal,
            },
        );

        // Ensure the vector is large enough
        if id.0 >= self.intervals.len() {
            self.intervals.resize_with(id.0 + 1, || None);
        }

        self.intervals[id.0] = Some(interval);
        self.interval_count += 1;

        // If we added a pull-like interval, update connections for any push intervals it might connect to
        if role != Pushing && role != Springy {
            // Find all push intervals connected to this pull interval
            let mut push_intervals = Vec::new();
            for (idx, interval_opt) in self.intervals.iter().enumerate() {
                if let Some(interval) = interval_opt {
                    if interval.has_role(Pushing) {
                        if interval.touches(alpha_index) || interval.touches(omega_index) {
                            push_intervals.push(UniqueId(idx));
                        }
                    }
                }
            }

            // Update connections for each connected push interval
            for push_id in push_intervals {
                self.update_interval_attachment_connections(push_id);
            }
        }

        id
    }

    /// Get an interval by its ID, returning a Result
    pub fn interval_result(&self, id: UniqueId) -> Result<&Interval, FabricError> {
        if id.0 >= self.intervals.len() {
            return Err(FabricError::IntervalNotFound);
        }
        self.intervals[id.0]
            .as_ref()
            .ok_or(FabricError::IntervalNotFound)
    }

    /// Get an interval by its ID
    /// This maintains backward compatibility with existing code
    pub fn interval(&self, id: UniqueId) -> &Interval {
        self.interval_result(id).expect("Interval not found")
    }

    /// Get an interval snapshot by its ID, returning a Result
    pub fn interval_snapshot_result(&self, id: UniqueId) -> Result<IntervalSnapshot, FabricError> {
        let interval = self.interval_result(id)?;

        // Make sure joint indices are valid
        if interval.alpha_index >= self.joints.len() || interval.omega_index >= self.joints.len() {
            return Err(FabricError::InvalidJointIndices);
        }

        let alpha = self.joints[interval.alpha_index];
        let omega = self.joints[interval.omega_index];

        Ok(IntervalSnapshot {
            interval: interval.clone(),
            alpha,
            omega,
        })
    }

    /// Get an interval snapshot by its ID
    /// This maintains backward compatibility with existing code
    pub fn interval_snapshot(&self, id: UniqueId) -> IntervalSnapshot {
        self.interval_snapshot_result(id)
            .expect("Failed to get interval snapshot")
    }

    pub fn remove_interval(&mut self, id: UniqueId) -> Interval {
        match self.intervals[id.0].take() {
            None => panic!("Removing nonexistent interval {:?}", id),
            Some(removed) => {
                self.interval_count -= 1;
                removed
            }
        }
    }

    pub fn find_push_at(&self, index: usize) -> Option<UniqueId> {
        self.intervals
            .iter()
            .enumerate()
            .find_map(|(id, interval_opt)| {
                interval_opt.as_ref().and_then(|interval| {
                    (interval.is_push_interval() && interval.touches(index)).then_some(UniqueId(id))
                })
            })
    }

    pub fn joining(&self, pair: (usize, usize)) -> Option<UniqueId> {
        self.intervals
            .iter()
            .enumerate()
            .filter_map(|(index, interval_opt)| {
                interval_opt.as_ref().and_then(|interval| {
                    if interval.touches(pair.0) && interval.touches(pair.1) {
                        Some(UniqueId(index))
                    } else {
                        None
                    }
                })
            })
            .next()
    }

    pub fn interval_values(&self) -> impl Iterator<Item = &Interval> {
        self.intervals
            .iter()
            .filter_map(|interval_opt| interval_opt.as_ref())
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Span {
    Fixed {
        length: f32,
    },
    Pretenst {
        target_length: f32,
        start_length: f32,
        rest_length: f32,
        finished: bool,
    },
    Approaching {
        target_length: f32,
        start_length: f32,
    },
    Muscle {
        rest_length: f32,
        contracted_length: f32,
        reverse: bool,
    },
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
    North = 7,
    South = 8,
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
            Pulling
                | Circumference
                | BowTie
                | FaceRadial
                | Support
                | North
                | South
                | GuyLine
                | PrismPull
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
            North => "north",
            South => "south",
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
            "north" => Some(North),
            "south" => Some(South),
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
            Pulling | BowTie | Support | GuyLine => 0.2,
            Springy => 0.7,
            Circumference => 0.25,
            FaceRadial => 0.15,
            North | South => 0.15,
            PrismPull => 0.2,
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
            North => [0.2, 0.2, 0.25, 1.0],
            South => [0.25, 0.2, 0.2, 1.0],
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
            North => [0.3, 0.8, 0.9, 1.0],         // Cyan
            South => [0.9, 0.3, 0.7, 1.0],         // Magenta
            GuyLine => [0.7, 0.7, 0.3, 1.0],       // Olive
            PrismPull => [0.6, 0.4, 0.8, 1.0],     // Purple
        }
    }
}

#[derive(Clone, Debug)]
pub struct Interval {
    pub alpha_index: usize,
    pub omega_index: usize,
    pub role: Role,
    pub material: Material,
    pub span: Span,
    pub unit: Vector3<f32>,
    pub strain: f32,

    // Connections structure that contains both alpha and omega connections
    // Only allocated for push intervals and boxed to reduce memory footprint
    pub connections: Option<Box<PullConnections>>,
}

impl Interval {
    /// Check if the interval has the specified role
    pub fn has_role(&self, role: Role) -> bool {
        self.role.is(role)
    }

    pub fn new(alpha_index: usize, omega_index: usize, role: Role, span: Span) -> Interval {
        // Only allocate connections for push intervals
        let is_push = role == Pushing;
        let connections = is_push.then_some(Box::new(PullConnections::new()));

        Interval {
            alpha_index,
            omega_index,
            role,
            material: role.material(),
            span,
            unit: zero(),
            strain: 0.0,
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
        joints: &[Joint],
        pull_intervals: &[(UniqueId, usize, usize)],
        pull_data: &[PullIntervalData],
    ) -> Result<(), FabricError> {
        // Only push intervals have connections to reorder
        if self.role != Pushing {
            return Err(FabricError::NotPushInterval);
        }

        // Get attachment points
        let attachment_points = self.attachment_points(joints)?;

        // Create a vector of joint positions
        let joint_positions: Vec<Point3<f32>> = joints.iter().map(|joint| joint.location).collect();

        // Reorder connections
        if let Some(conn) = &mut self.connections {
            conn.reorder_connections(
                &attachment_points.0,
                &attachment_points.1,
                &joint_positions,
                pull_intervals,
                pull_data,
                self.alpha_index,
                self.omega_index,
            );
        }

        Ok(())
    }

    /// Get attachment points for a push interval at both ends
    /// Returns (alpha_end_points, omega_end_points) as arrays of AttachmentPoint
    /// Returns an error if this is not a push interval
    pub fn attachment_points(
        &self,
        joints: &[Joint],
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

        // Calculate the radius based on the role
        let radius = self.role.appearance().radius;

        // Calculate attachment points at both ends of the interval
        Ok(calculate_interval_attachment_points(
            *alpha_location,
            *omega_location,
            radius,
        ))
    }

    /// Get a specific attachment point by its index and end
    /// Returns an error if this is not a push interval or if the index is out of bounds
    pub fn get_attachment_point(
        &self,
        joints: &[Joint],
        end: IntervalEnd,
        index: usize,
    ) -> Result<AttachmentPoint, FabricError> {
        if index >= ATTACHMENT_POINTS {
            return Err(FabricError::InvalidAttachmentIndex);
        }

        self.attachment_points(joints)
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
        joints: &[Joint],
        position: Point3<f32>,
    ) -> Result<(IntervalEnd, AttachmentPoint), FabricError> {
        let (alpha_points, omega_points) = self.attachment_points(joints)?;

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
        joints: &[Joint],
        end: IntervalEnd,
        index: usize,
    ) -> Result<AttachmentPoint, FabricError> {
        if index >= ATTACHMENT_POINTS {
            return Err(FabricError::InvalidAttachmentIndex);
        }

        let points = self.attachment_points(joints)?;

        // Use the opposite() method from IntervalEnd
        let opposite_end = end.opposite();

        Ok(self.get_point_from_end(points, opposite_end, index))
    }

    pub fn key(&self) -> (usize, usize) {
        if self.alpha_index < self.omega_index {
            (self.alpha_index, self.omega_index)
        } else {
            (self.omega_index, self.alpha_index)
        }
    }

    pub fn joint_removed(&mut self, index: usize) {
        // Helper function to update an index if needed
        let update_index = |current: &mut usize| {
            if *current > index {
                *current -= 1;
            }
        };

        // Update both alpha and omega indices
        update_index(&mut self.alpha_index);
        update_index(&mut self.omega_index);
    }

    /// Get the joint index for a specific end of the interval
    pub fn end_index(&self, end: IntervalEnd) -> usize {
        match end {
            IntervalEnd::Alpha => self.alpha_index,
            IntervalEnd::Omega => self.omega_index,
        }
    }

    /// Get the joint location for a specific end of the interval
    pub fn end_location<'a>(&self, joints: &'a [Joint], end: IntervalEnd) -> &'a Point3<f32> {
        &joints[self.end_index(end)].location
    }

    pub fn locations<'a>(&self, joints: &'a [Joint]) -> (&'a Point3<f32>, &'a Point3<f32>) {
        (
            self.end_location(joints, IntervalEnd::Alpha),
            self.end_location(joints, IntervalEnd::Omega),
        )
    }

    pub fn midpoint(&self, joints: &[Joint]) -> Point3<f32> {
        let (alpha, omega) = self.locations(joints);
        Point3::from_vec((alpha.to_vec() + omega.to_vec()) / 2f32)
    }

    pub fn fast_length(&mut self, joints: &[Joint]) -> f32 {
        let (alpha_location, omega_location) = self.locations(joints);
        self.unit = omega_location - alpha_location;
        let magnitude_squared = self.unit.magnitude2();
        if magnitude_squared < 0.00001 {
            return 0.00001;
        }
        let inverse_square_root = magnitude_squared.inv_sqrt32();
        self.unit *= inverse_square_root;
        1.0 / inverse_square_root
    }

    pub fn length(&self, joints: &[Joint]) -> f32 {
        let (alpha_location, omega_location) = self.locations(joints);
        let tween = omega_location - alpha_location;
        let magnitude_squared = tween.magnitude2();
        if magnitude_squared < 0.00001 {
            return 0.00001;
        }
        magnitude_squared.sqrt()
    }

    pub fn ideal(&self) -> f32 {
        match self.span {
            Fixed { length, .. }
            | Pretenst {
                target_length: length,
                ..
            }
            | Approaching {
                target_length: length,
                ..
            } => length,
            Muscle {
                rest_length,
                contracted_length,
                ..
            } => (rest_length + contracted_length) / 2.0,
        }
    }

    pub fn iterate(
        &mut self,
        joints: &mut [Joint],
        progress: &Progress,
        muscle_nuance: f32,
        physics: &Physics,
        scale: f32,
    ) {
        let ideal = match self.span {
            Fixed { length } => length,
            Pretenst {
                start_length,
                target_length,
                finished,
                ..
            } => {
                if finished {
                    target_length
                } else {
                    let progress_nuance = progress.nuance();
                    start_length * (1.0 - progress_nuance) + target_length * progress_nuance
                }
            }
            Approaching {
                start_length,
                target_length,
                ..
            } => {
                let progress_nuance = progress.nuance();
                start_length * (1.0 - progress_nuance) + target_length * progress_nuance
            }
            Muscle {
                rest_length,
                contracted_length,
                reverse,
            } => {
                let nuance = if reverse {
                    1.0 - muscle_nuance
                } else {
                    muscle_nuance
                };
                contracted_length * (1.0 - nuance) + rest_length * nuance
            }
        };
        let real_length = self.fast_length(joints);
        let real_length_mm = Millimeters(real_length * scale);

        // Check if interval is slack (push stretched or pull compressed)
        let is_slack = (self.is_push_interval() && real_length > ideal)
            || (self.is_pull_interval() && real_length < ideal);

        // Calculate strain, zeroing it out when slack
        self.strain = if is_slack {
            0.0
        } else {
            (real_length - ideal) / ideal
        };

        // Force: strain × material_stiffness × phase_softening
        let material_stiffness = self.material.stiffness();
        let force = self.strain * material_stiffness * physics.stiffness_factor;
        let force_vector: Vector3<f32> = self.unit * force / 2.0;

        // Apply forces to both ends
        let alpha_idx = self.end_index(IntervalEnd::Alpha);
        let omega_idx = self.end_index(IntervalEnd::Omega);
        joints[alpha_idx].force += force_vector;
        joints[omega_idx].force -= force_vector;

        // Mass from linear density × length
        let interval_mass = self.material.linear_density() * real_length_mm;
        let half_mass = interval_mass / 2.0;
        joints[alpha_idx].accumulated_mass += half_mass;
        joints[omega_idx].accumulated_mass += half_mass;
    }

    /// Check if this interval touches a specific joint
    pub fn touches(&self, joint: usize) -> bool {
        self.end_index(IntervalEnd::Alpha) == joint || self.end_index(IntervalEnd::Omega) == joint
    }

    /// Check if a specific end of this interval touches a joint
    pub fn end_touches(&self, end: IntervalEnd, joint: usize) -> bool {
        self.end_index(end) == joint
    }

    /// Get the end of the interval that corresponds to a given joint index
    pub fn joint_end(&self, joint_index: usize) -> IntervalEnd {
        if self.end_index(IntervalEnd::Alpha) == joint_index {
            IntervalEnd::Alpha
        } else if self.end_index(IntervalEnd::Omega) == joint_index {
            IntervalEnd::Omega
        } else {
            panic!("Joint index {} is not part of this interval", joint_index)
        }
    }

    /// Get the ray direction from a joint
    pub fn ray_from(&self, joint_index: usize) -> Vector3<f32> {
        match self.joint_end(joint_index) {
            IntervalEnd::Alpha => self.unit,
            IntervalEnd::Omega => self.unit.mul(-1.0),
        }
    }

    /// Get the index of the joint at the opposite end
    pub fn other_joint(&self, joint_index: usize) -> usize {
        let end = self.joint_end(joint_index);
        self.end_index(end.opposite())
    }

    pub fn joint_with(
        &self,
        Interval {
            alpha_index,
            omega_index,
            ..
        }: &Interval,
    ) -> Option<usize> {
        if self.alpha_index == *alpha_index || self.alpha_index == *omega_index {
            Some(self.alpha_index)
        } else if self.omega_index == *alpha_index || self.omega_index == *omega_index {
            Some(self.omega_index)
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
    pub fn end_index(&self, end: &IntervalEnd) -> usize {
        match end {
            IntervalEnd::Alpha => self.interval.alpha_index,
            IntervalEnd::Omega => self.interval.omega_index,
        }
    }
}
