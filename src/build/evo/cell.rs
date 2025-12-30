use crate::fabric::interval::Role;
use crate::fabric::{Fabric, IntervalKey, JointKey};
use crate::fabric::IntervalEnd;
use glam::Vec3;

/// A Cell wraps a push interval and manages its connections.
///
/// In the evolutionary tensegrity system, push intervals are the autonomous
/// "cells" that can extend pull intervals to nearby endpoints.
pub struct Cell {
    /// The push interval this cell represents
    pub push_key: IntervalKey,
    /// The alpha (start) endpoint joint
    pub alpha_joint: JointKey,
    /// The omega (end) endpoint joint
    pub omega_joint: JointKey,
    /// Pull intervals connected to the alpha end
    alpha_pulls: Vec<IntervalKey>,
    /// Pull intervals connected to the omega end
    omega_pulls: Vec<IntervalKey>,
}

impl Cell {
    /// Create a new cell with a push interval at the given location and direction.
    pub fn new(fabric: &mut Fabric, location: Vec3, direction: Vec3, length: f32) -> Self {
        let half = direction.normalize() * length / 2.0;
        let alpha = fabric.create_joint(location - half);
        let omega = fabric.create_joint(location + half);
        let push_key = fabric.create_slack_interval(alpha, omega, Role::Pushing);

        Self {
            push_key,
            alpha_joint: alpha,
            omega_joint: omega,
            alpha_pulls: Vec::new(),
            omega_pulls: Vec::new(),
        }
    }

    /// Create a cell from an existing push interval.
    pub fn from_existing(
        push_key: IntervalKey,
        alpha_joint: JointKey,
        omega_joint: JointKey,
    ) -> Self {
        Self {
            push_key,
            alpha_joint,
            omega_joint,
            alpha_pulls: Vec::new(),
            omega_pulls: Vec::new(),
        }
    }

    /// Get the joint key for a given end.
    pub fn joint_at(&self, end: IntervalEnd) -> JointKey {
        match end {
            IntervalEnd::Alpha => self.alpha_joint,
            IntervalEnd::Omega => self.omega_joint,
        }
    }

    /// Get all endpoints as (joint_key, end) pairs.
    pub fn endpoints(&self) -> [(JointKey, IntervalEnd); 2] {
        [
            (self.alpha_joint, IntervalEnd::Alpha),
            (self.omega_joint, IntervalEnd::Omega),
        ]
    }

    /// Get the location of an endpoint.
    pub fn endpoint_location(&self, fabric: &Fabric, end: IntervalEnd) -> Vec3 {
        fabric.location(self.joint_at(end))
    }

    /// Count pull intervals connected to a specific end.
    pub fn pull_count(&self, end: IntervalEnd) -> usize {
        match end {
            IntervalEnd::Alpha => self.alpha_pulls.len(),
            IntervalEnd::Omega => self.omega_pulls.len(),
        }
    }

    /// Check if this end needs more pulls (target: at least 3).
    pub fn needs_more_pulls(&self, end: IntervalEnd) -> bool {
        self.pull_count(end) < 3
    }

    /// Check if this end can accept more pulls (max: 6).
    pub fn can_accept_pull(&self, end: IntervalEnd) -> bool {
        self.pull_count(end) < 6
    }

    /// Add a pull interval to a specific end.
    pub fn add_pull(&mut self, end: IntervalEnd, pull_key: IntervalKey) {
        match end {
            IntervalEnd::Alpha => self.alpha_pulls.push(pull_key),
            IntervalEnd::Omega => self.omega_pulls.push(pull_key),
        }
    }

    /// Get all pull intervals at a specific end.
    pub fn pulls_at(&self, end: IntervalEnd) -> &[IntervalKey] {
        match end {
            IntervalEnd::Alpha => &self.alpha_pulls,
            IntervalEnd::Omega => &self.omega_pulls,
        }
    }

    /// Get total number of pull intervals connected to this cell.
    pub fn total_pulls(&self) -> usize {
        self.alpha_pulls.len() + self.omega_pulls.len()
    }

    /// Check if this cell is well-connected (both ends have at least 3 pulls).
    pub fn is_well_connected(&self) -> bool {
        !self.needs_more_pulls(IntervalEnd::Alpha) && !self.needs_more_pulls(IntervalEnd::Omega)
    }
}
