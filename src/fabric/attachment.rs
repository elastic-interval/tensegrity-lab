/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::fabric::{FabricDimensions, IntervalEnd, IntervalKey, JointKey, Joints};
use crate::units::{Degrees, Unit};
use glam::Vec3;

/// Number of attachment points at each end of a push interval
pub const ATTACHMENT_POINTS: usize = 10;

/// The 5 allowed hinge bending angles
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display)]
pub enum HingeBend {
    #[strum(serialize = "-60")]
    Neg60,
    #[strum(serialize = "-30")]
    Neg30,
    #[strum(serialize = "0")]
    Zero,
    #[strum(serialize = "+30")]
    Pos30,
    #[strum(serialize = "+60")]
    Pos60,
}

impl HingeBend {
    /// Get the angle in degrees as f32
    pub fn degrees(&self) -> f32 {
        match self {
            HingeBend::Neg60 => -60.0,
            HingeBend::Neg30 => -30.0,
            HingeBend::Zero => 0.0,
            HingeBend::Pos30 => 30.0,
            HingeBend::Pos60 => 60.0,
        }
    }

    /// Snap an ideal angle to the nearest HingeBend
    pub fn from_angle(angle: Degrees) -> Self {
        let deg = angle.0;
        if deg < -45.0 {
            HingeBend::Neg60
        } else if deg < -15.0 {
            HingeBend::Neg30
        } else if deg < 15.0 {
            HingeBend::Zero
        } else if deg < 45.0 {
            HingeBend::Pos30
        } else {
            HingeBend::Pos60
        }
    }

    /// Calculate the hinge endpoint position
    pub fn endpoint(
        &self,
        hinge_pos: Vec3,
        push_axis: Vec3,
        radial_direction: Vec3,
        hinge_length: f32,
    ) -> Vec3 {
        // At 0°, hinge points along radial_direction (away from ring center)
        // At +angle, it rotates toward push_axis (outward along bolt)
        // At -angle, it rotates away from push_axis (inward toward push interval)
        let angle_rad = self.degrees().to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        let hinge_direction = radial_direction * cos_a + push_axis * sin_a;
        hinge_pos + hinge_direction * hinge_length
    }
}

/// Represents an attachment point on a push interval
#[derive(Clone, Copy, Debug)]
pub struct AttachmentPoint {
    /// The position of the attachment point in 3D space
    pub position: Vec3,

    /// The index of this attachment point (0-5)
    pub index: usize,
}

/// Represents a connection between a pull interval and an attachment point
#[derive(Clone, Copy, Debug)]
pub struct PullConnection {
    /// The key of the pull interval that is attached
    pub pull_interval_key: IntervalKey,

    /// The attachment point index where the pull interval is connected
    pub attachment_index: usize,
}

/// Encapsulates the array of connections between intervals
#[derive(Clone, Debug)]
pub struct PullConnections {
    pub alpha: [Option<PullConnection>; ATTACHMENT_POINTS],
    pub omega: [Option<PullConnection>; ATTACHMENT_POINTS],
}

impl PullConnections {
    /// Creates a new empty set of connections
    pub fn new() -> Self {
        Self {
            alpha: [None; ATTACHMENT_POINTS],
            omega: [None; ATTACHMENT_POINTS],
        }
    }

    /// Returns the connections array for the specified end
    pub fn connections(&self, end: IntervalEnd) -> &[Option<PullConnection>; ATTACHMENT_POINTS] {
        match end {
            IntervalEnd::Alpha => &self.alpha,
            IntervalEnd::Omega => &self.omega,
        }
    }

    /// Reorders connections to ensure each pull interval is assigned to the most appropriate attachment point
    /// This optimizes the positions of connections to minimize rotational moment
    pub fn reorder_connections(
        &mut self,
        alpha_attachment_points: &[AttachmentPoint],
        omega_attachment_points: &[AttachmentPoint],
        joints: &Joints,
        pull_intervals: &[(IntervalKey, JointKey, JointKey)], // (pull_id, alpha_key, omega_key)
        pull_data: &[PullIntervalData],
        push_alpha_key: JointKey,
        push_omega_key: JointKey,
    ) {
        // Step 1: Collect all connections that need to be made
        let connections_to_make =
            self.collect_connections_to_make(pull_intervals, push_alpha_key, push_omega_key);

        // Step 2: Clear all existing connections
        self.alpha = [None; ATTACHMENT_POINTS];
        self.omega = [None; ATTACHMENT_POINTS];

        // Step 3: Separate connections by end
        let alpha_connections: Vec<_> = connections_to_make
            .iter()
            .filter(|(end, _, _)| matches!(end, IntervalEnd::Alpha))
            .copied()
            .collect();

        let omega_connections: Vec<_> = connections_to_make
            .iter()
            .filter(|(end, _, _)| matches!(end, IntervalEnd::Omega))
            .copied()
            .collect();

        // Calculate push axis: direction from alpha to omega
        let alpha_pos = joints[push_alpha_key].location;
        let omega_pos = joints[push_omega_key].location;
        let push_direction = (omega_pos - alpha_pos).normalize();

        // Step 4: Find optimal assignment for each end using moment minimization
        // Alpha end: push axis points outward (opposite to push direction)
        let optimized_alpha = find_optimal_assignment(
            &alpha_connections,
            alpha_attachment_points,
            pull_data,
            joints,
            push_alpha_key,
            -push_direction, // Outward from alpha end
        );

        // Omega end: push axis points outward (same as push direction)
        let optimized_omega = find_optimal_assignment(
            &omega_connections,
            omega_attachment_points,
            pull_data,
            joints,
            push_omega_key,
            push_direction, // Outward from omega end
        );

        // Step 5: Assign connections using optimized order
        for (attach_idx, (end, pull_id, _joint_key)) in optimized_alpha.iter().enumerate() {
            if attach_idx < ATTACHMENT_POINTS {
                let connection = PullConnection {
                    pull_interval_key: *pull_id,
                    attachment_index: attach_idx,
                };
                match end {
                    IntervalEnd::Alpha => self.alpha[attach_idx] = Some(connection),
                    IntervalEnd::Omega => self.omega[attach_idx] = Some(connection),
                }
            }
        }

        for (attach_idx, (end, pull_id, _joint_key)) in optimized_omega.iter().enumerate() {
            if attach_idx < ATTACHMENT_POINTS {
                let connection = PullConnection {
                    pull_interval_key: *pull_id,
                    attachment_index: attach_idx,
                };
                match end {
                    IntervalEnd::Alpha => self.alpha[attach_idx] = Some(connection),
                    IntervalEnd::Omega => self.omega[attach_idx] = Some(connection),
                }
            }
        }
    }

    /// Collects all connections that need to be made for a push interval
    fn collect_connections_to_make(
        &self,
        pull_intervals: &[(IntervalKey, JointKey, JointKey)],
        push_alpha_key: JointKey,
        push_omega_key: JointKey,
    ) -> Vec<(IntervalEnd, IntervalKey, JointKey)> {
        let mut connections_to_make = Vec::new();

        for (pull_id, alpha_key, omega_key) in pull_intervals {
            // Check if pull's alpha end connects to this push interval
            if *alpha_key == push_alpha_key {
                connections_to_make.push((IntervalEnd::Alpha, *pull_id, *alpha_key));
            } else if *alpha_key == push_omega_key {
                connections_to_make.push((IntervalEnd::Omega, *pull_id, *alpha_key));
            }

            // Check if pull's omega end connects to this push interval
            if *omega_key == push_alpha_key {
                connections_to_make.push((IntervalEnd::Alpha, *pull_id, *omega_key));
            } else if *omega_key == push_omega_key {
                connections_to_make.push((IntervalEnd::Omega, *pull_id, *omega_key));
            }
        }

        connections_to_make
    }

    /// Checks if a specific index is occupied at the specified end
    pub fn is_occupied(&self, end: IntervalEnd, index: usize) -> bool {
        if index < ATTACHMENT_POINTS {
            match end {
                IntervalEnd::Alpha => self.alpha[index].is_some(),
                IntervalEnd::Omega => self.omega[index].is_some(),
            }
        } else {
            false
        }
    }

    /// Gets a specific connection at the specified end
    pub fn get_connection(&self, end: IntervalEnd, index: usize) -> Option<&PullConnection> {
        if index < ATTACHMENT_POINTS {
            match end {
                IntervalEnd::Alpha => self.alpha[index].as_ref(),
                IntervalEnd::Omega => self.omega[index].as_ref(),
            }
        } else {
            None
        }
    }

    /// Gets a specific connection at the specified end as mutable
    pub fn get_connection_mut(
        &mut self,
        end: IntervalEnd,
        index: usize,
    ) -> Option<&mut PullConnection> {
        if index < ATTACHMENT_POINTS {
            match end {
                IntervalEnd::Alpha => self.alpha[index].as_mut(),
                IntervalEnd::Omega => self.omega[index].as_mut(),
            }
        } else {
            None
        }
    }

    /// Sets a specific connection at the specified end
    pub fn set_connection(
        &mut self,
        end: IntervalEnd,
        index: usize,
        connection: Option<PullConnection>,
    ) -> bool {
        if index < ATTACHMENT_POINTS {
            match end {
                IntervalEnd::Alpha => self.alpha[index] = connection,
                IntervalEnd::Omega => self.omega[index] = connection,
            }
            true
        } else {
            false
        }
    }
}

/// Data about a pull interval needed for moment calculation
#[derive(Clone, Copy, Debug)]
pub struct PullIntervalData {
    pub key: IntervalKey,
    pub alpha_key: JointKey,
    pub omega_key: JointKey,
    pub strain: f32,
    pub unit: Vec3,
}

/// Helper function to find the nearest attachment point in a set of points
/// Returns the index of the nearest point and its squared distance
/// If the points array is empty, returns (0, f32::MAX) as a fallback
pub fn find_nearest_attachment_point(
    points: &[AttachmentPoint],
    position: Vec3,
) -> (usize, f32) {
    if points.is_empty() {
        return (0, f32::MAX); // Fallback for empty arrays
    }

    points
        .iter()
        .enumerate()
        .map(|(i, point)| (i, position.distance_squared(point.position)))
        .min_by(|(_, dist1), (_, dist2)| {
            // Handle NaN values safely by considering them equal
            // This prevents unwrap failures on partial_cmp
            dist1
                .partial_cmp(dist2)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or((0, f32::MAX)) // Additional safety in case min_by fails
}

/// Calculates the total rotational moment for a given assignment of pulls to attachment points
/// The first attachment point acts as a pivot (ball joint)
/// Returns the magnitude of the total moment vector
fn calculate_rotational_moment(
    assignment: &[(IntervalKey, usize)], // (pull_id, attachment_point_index)
    attachment_points: &[AttachmentPoint],
    pull_data: &[PullIntervalData],
    joints: &Joints,
    push_joint_key: JointKey,
) -> f32 {
    if assignment.is_empty() || attachment_points.is_empty() {
        return 0.0;
    }

    // The first (closest) attachment point is the pivot
    let pivot_position = attachment_points[0].position;

    // Accumulate moment vector
    let mut total_moment = Vec3::ZERO;

    for (pull_key, attach_idx) in assignment {
        // Skip if attachment index is out of bounds
        if *attach_idx >= attachment_points.len() {
            continue;
        }
        // Find the pull interval data
        if let Some(pull) = pull_data.iter().find(|p| p.key == *pull_key) {
            // Get attachment point position
            let attach_pos = attachment_points[*attach_idx].position;

            // Calculate moment arm: vector from pivot to attachment point
            let moment_arm = attach_pos - pivot_position;

            // Determine which end of the pull connects to this push
            let connected_key = if pull.alpha_key == push_joint_key {
                pull.alpha_key
            } else {
                pull.omega_key
            };

            // Calculate force vector: strain * unit direction
            // Direction is from the attachment point toward the pull's other end
            let other_key = if connected_key == pull.alpha_key {
                pull.omega_key
            } else {
                pull.alpha_key
            };

            let other_pos = joints[other_key].location;
            let pull_direction = (other_pos - attach_pos).normalize();

            let force = pull_direction * pull.strain;

            // Calculate moment: r × F (cross product)
            let moment = moment_arm.cross(force);
            total_moment += moment;
        }
    }

    // Return magnitude of total moment
    total_moment.length()
}

/// Checks if a pull interval is "outward-pulling" (positive dot product with push axis)
/// These must be assigned to the lowest slot (slot 0)
fn is_outward_pulling(
    pull: &PullIntervalData,
    push_joint_key: JointKey,
    push_axis: Vec3,
    joints: &Joints,
) -> bool {
    // Determine which end of the pull connects to this push joint
    let other_key = if pull.alpha_key == push_joint_key {
        pull.omega_key
    } else {
        pull.alpha_key
    };

    // Pull direction: from push joint toward the other end
    let push_pos = joints[push_joint_key].location;
    let other_pos = joints[other_key].location;
    let pull_direction = (other_pos - push_pos).normalize();

    // Positive dot product means pulling outward along the push axis
    pull_direction.dot(push_axis) > 0.0
}

/// Finds the optimal assignment of pull intervals to attachment points
/// that minimizes rotational moment.
/// Outward-pulling intervals (positive dot with push axis) are forced to slot 0.
fn find_optimal_assignment(
    pulls: &[(IntervalEnd, IntervalKey, JointKey)], // (end, pull_id, joint_key)
    attachment_points: &[AttachmentPoint],
    pull_data: &[PullIntervalData],
    joints: &Joints,
    push_joint_key: JointKey,
    push_axis: Vec3,
) -> Vec<(IntervalEnd, IntervalKey, JointKey)> {
    if pulls.is_empty() {
        return Vec::new();
    }

    // Separate outward-pulling intervals from inward-pulling ones
    let mut outward_pulls = Vec::new();
    let mut inward_pulls = Vec::new();

    for pull in pulls {
        if let Some(data) = pull_data.iter().find(|d| d.key == pull.1) {
            if is_outward_pulling(data, push_joint_key, push_axis, joints) {
                outward_pulls.push(*pull);
            } else {
                inward_pulls.push(*pull);
            }
        } else {
            inward_pulls.push(*pull);
        }
    }

    // Build result: outward pulls get lowest slots, then inward pulls
    let mut result = Vec::with_capacity(pulls.len());

    // Add outward-pulling intervals first (they get the lowest slots)
    for pull in &outward_pulls {
        result.push(*pull);
    }

    // If there are no inward pulls to optimize, we're done
    if inward_pulls.is_empty() {
        return result;
    }

    // If there's only one inward pull, no optimization needed
    if inward_pulls.len() == 1 {
        result.push(inward_pulls[0]);
        return result;
    }

    // Optimize the inward pulls using moment minimization
    let n = inward_pulls.len();
    let start_slot = outward_pulls.len(); // Inward pulls start after outward slots

    let mut best_moment = f32::MAX;
    let mut indices: Vec<usize> = (0..n).collect();
    let mut best_order = inward_pulls.clone();

    // Helper to evaluate current permutation
    let evaluate = |perm: &[usize]| -> f32 {
        let assignment: Vec<(IntervalKey, usize)> = perm
            .iter()
            .enumerate()
            .map(|(i, &pull_idx)| (inward_pulls[pull_idx].1, start_slot + i))
            .collect();
        calculate_rotational_moment(
            &assignment,
            attachment_points,
            pull_data,
            joints,
            push_joint_key,
        )
    };

    // Heap's algorithm for generating permutations
    fn heap_permute<F>(k: usize, indices: &mut [usize], callback: &mut F)
    where
        F: FnMut(&[usize]),
    {
        if k == 1 {
            callback(indices);
        } else {
            heap_permute(k - 1, indices, callback);
            for i in 0..k - 1 {
                if k % 2 == 0 {
                    indices.swap(i, k - 1);
                } else {
                    indices.swap(0, k - 1);
                }
                heap_permute(k - 1, indices, callback);
            }
        }
    }

    heap_permute(n, &mut indices, &mut |perm| {
        let moment = evaluate(perm);
        if moment < best_moment {
            best_moment = moment;
            for (i, &pull_idx) in perm.iter().enumerate() {
                best_order[i] = inward_pulls[pull_idx];
            }
        }
    });

    // Add the optimized inward pulls
    result.extend(best_order);

    result
}

/// Generates the positions of attachment points at the end of a push interval
///
/// # Parameters
/// * `end_position` - The position of the end of the push interval
/// * `direction` - The direction vector of the push interval (points outward from interval)
/// * `dimensions` - The fabric dimensions with scaled values
pub fn generate_attachment_points(
    end_position: Vec3,
    direction: Vec3,
    dimensions: &FabricDimensions,
) -> [AttachmentPoint; ATTACHMENT_POINTS] {
    // Normalize the direction vector to get the axis
    let axis = direction.normalize();

    // Create array to hold all attachment points
    let mut points = [AttachmentPoint {
        position: end_position,
        index: 0,
    }; ATTACHMENT_POINTS];

    // Generate attachment points extending outwards along the axis
    // Each point represents the center of a ring at that slot
    for i in 0..ATTACHMENT_POINTS {
        // Ring center is at slot index * disc_thickness + half disc thickness
        let distance = dimensions.hinge.disc_thickness.f32() * (i as f32 + 0.5);

        // Calculate offset vector
        let offset = axis * distance;

        // Set the position and index
        points[i] = AttachmentPoint {
            position: end_position + offset,
            index: i,
        };
    }

    points
}

/// Calculates attachment points for both ends of a push interval
pub fn calculate_interval_attachment_points(
    start: Vec3,
    end: Vec3,
    dimensions: &FabricDimensions,
) -> (
    [AttachmentPoint; ATTACHMENT_POINTS],
    [AttachmentPoint; ATTACHMENT_POINTS],
) {
    // Calculate direction vector from start to end
    let direction = end - start;

    // Generate attachment points at both ends
    // Alpha end: points extend outward from start (opposite to interval direction)
    // Omega end: points extend outward from end (in interval direction)
    (
        generate_attachment_points(start, -direction, dimensions),
        generate_attachment_points(end, direction, dimensions),
    )
}
