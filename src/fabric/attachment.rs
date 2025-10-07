/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::fabric::{IntervalEnd, UniqueId};
use cgmath::{InnerSpace, MetricSpace, Point3, Vector3};

/// Number of attachment points at each end of a push interval
pub const ATTACHMENT_POINTS: usize = 6;

/// Represents an attachment point on a push interval
#[derive(Clone, Copy, Debug)]
pub struct AttachmentPoint {
    /// The position of the attachment point in 3D space
    pub position: Point3<f32>,

    /// The index of this attachment point (0-5)
    pub index: usize,
}

/// Represents a connection between a pull interval and an attachment point
#[derive(Clone, Copy, Debug)]
pub struct PullConnection {
    /// The ID of the pull interval that is attached
    pub pull_interval_id: UniqueId,

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
        joint_positions: &[Point3<f32>],
        pull_intervals: &[(UniqueId, usize, usize)], // (pull_id, alpha_index, omega_index)
        pull_data: &[PullIntervalData],
        push_alpha_index: usize,
        push_omega_index: usize,
    ) {
        // Step 1: Collect all connections that need to be made
        let connections_to_make =
            self.collect_connections_to_make(pull_intervals, push_alpha_index, push_omega_index);

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

        // Step 4: Find optimal assignment for each end using moment minimization
        let optimized_alpha = find_optimal_assignment(
            &alpha_connections,
            alpha_attachment_points,
            pull_data,
            joint_positions,
            push_alpha_index,
        );

        let optimized_omega = find_optimal_assignment(
            &omega_connections,
            omega_attachment_points,
            pull_data,
            joint_positions,
            push_omega_index,
        );

        // Step 5: Assign connections using optimized order
        for (attach_idx, (end, pull_id, _joint_index)) in optimized_alpha.iter().enumerate() {
            if attach_idx < ATTACHMENT_POINTS {
                let connection = PullConnection {
                    pull_interval_id: *pull_id,
                    attachment_index: attach_idx,
                };
                match end {
                    IntervalEnd::Alpha => self.alpha[attach_idx] = Some(connection),
                    IntervalEnd::Omega => self.omega[attach_idx] = Some(connection),
                }
            }
        }

        for (attach_idx, (end, pull_id, _joint_index)) in optimized_omega.iter().enumerate() {
            if attach_idx < ATTACHMENT_POINTS {
                let connection = PullConnection {
                    pull_interval_id: *pull_id,
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
        pull_intervals: &[(UniqueId, usize, usize)],
        push_alpha_index: usize,
        push_omega_index: usize,
    ) -> Vec<(IntervalEnd, UniqueId, usize)> {
        let mut connections_to_make = Vec::new();

        for (pull_id, alpha_index, omega_index) in pull_intervals {
            // Check if pull's alpha end connects to this push interval
            if *alpha_index == push_alpha_index {
                connections_to_make.push((IntervalEnd::Alpha, *pull_id, *alpha_index));
            } else if *alpha_index == push_omega_index {
                connections_to_make.push((IntervalEnd::Omega, *pull_id, *alpha_index));
            }

            // Check if pull's omega end connects to this push interval
            if *omega_index == push_alpha_index {
                connections_to_make.push((IntervalEnd::Alpha, *pull_id, *omega_index));
            } else if *omega_index == push_omega_index {
                connections_to_make.push((IntervalEnd::Omega, *pull_id, *omega_index));
            }
        }

        connections_to_make
    }

    /// Assigns connections to attachment points
    fn assign_connections(
        &mut self,
        connections_to_make: Vec<(IntervalEnd, UniqueId, usize)>,
        alpha_attachment_points: &[AttachmentPoint],
        omega_attachment_points: &[AttachmentPoint],
        joint_positions: &[Point3<f32>],
        pull_intervals: &[(UniqueId, usize, usize)], // (pull_id, alpha_index, omega_index)
    ) {
        for (end, pull_id, _joint_index) in connections_to_make {
            let attachment_points = self.get_attachment_points_for_end(
                end,
                alpha_attachment_points,
                omega_attachment_points,
            );

            // Find the best available attachment point
            let best_idx = match end {
                IntervalEnd::Alpha => Self::find_best_attachment_point(
                    attachment_points,
                    joint_positions,
                    pull_intervals,
                    pull_id,
                    &self.alpha,
                ),
                IntervalEnd::Omega => Self::find_best_attachment_point(
                    attachment_points,
                    joint_positions,
                    pull_intervals,
                    pull_id,
                    &self.omega,
                ),
            };

            // Assign to the closest available attachment point
            if let Some(idx) = best_idx {
                let connection = PullConnection {
                    pull_interval_id: pull_id,
                    attachment_index: idx,
                };

                // Update the appropriate array
                match end {
                    IntervalEnd::Alpha => self.alpha[idx] = Some(connection),
                    IntervalEnd::Omega => self.omega[idx] = Some(connection),
                };
            }
        }
    }

    /// Gets the attachment points for a specific interval end
    fn get_attachment_points_for_end<'a>(
        &self,
        end: IntervalEnd,
        alpha_attachment_points: &'a [AttachmentPoint],
        omega_attachment_points: &'a [AttachmentPoint],
    ) -> &'a [AttachmentPoint] {
        match end {
            IntervalEnd::Alpha => alpha_attachment_points,
            IntervalEnd::Omega => omega_attachment_points,
        }
    }

    /// Finds the best available attachment point
    /// Prioritizes lower indices (first positions) to ensure connections fill from the beginning
    fn find_best_attachment_point(
        attachment_points: &[AttachmentPoint],
        _joint_positions: &[Point3<f32>],
        _pull_intervals: &[(UniqueId, usize, usize)], // (pull_id, alpha_index, omega_index)
        _pull_id: UniqueId,
        target_array: &[Option<PullConnection>; ATTACHMENT_POINTS],
    ) -> Option<usize> {
        // Find the first available (unoccupied) attachment point
        // This ensures connections are assigned starting from index 0
        for i in 0..attachment_points.len() {
            if target_array[i].is_none() {
                return Some(i);
            }
        }
        
        // No available attachment points
        None
    }

    /// Helper function to calculate the minimum distance between a joint and any attachment point
    fn calculate_min_distance(
        joint_index: usize,
        attachment_points: &[AttachmentPoint],
        joint_positions: &[Point3<f32>],
    ) -> f32 {
        if attachment_points.is_empty() {
            return f32::MAX;
        }

        let joint_position = joint_positions[joint_index];
        attachment_points
            .iter()
            .map(|point| joint_position.distance2(point.position))
            .min_by(|dist1, dist2| {
                dist1
                    .partial_cmp(dist2)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(f32::MAX)
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
    pub id: UniqueId,
    pub alpha_joint: usize,
    pub omega_joint: usize,
    pub strain: f32,
    pub unit: Vector3<f32>,
}

/// Helper function to find the nearest attachment point in a set of points
/// Returns the index of the nearest point and its squared distance
/// If the points array is empty, returns (0, f32::MAX) as a fallback
pub fn find_nearest_attachment_point(
    points: &[AttachmentPoint],
    position: Point3<f32>,
) -> (usize, f32) {
    if points.is_empty() {
        return (0, f32::MAX); // Fallback for empty arrays
    }

    points
        .iter()
        .enumerate()
        .map(|(i, point)| (i, position.distance2(point.position)))
        .min_by(|(_, dist1), (_, dist2)| {
            // Handle NaN values safely by considering them equal
            // This prevents unwrap failures on partial_cmp
            dist1
                .partial_cmp(dist2)
                .unwrap_or_else(|| std::cmp::Ordering::Equal)
        })
        .unwrap_or((0, f32::MAX)) // Additional safety in case min_by fails
}

/// Calculates the total rotational moment for a given assignment of pulls to attachment points
/// The first attachment point acts as a pivot (ball joint)
/// Returns the magnitude of the total moment vector
fn calculate_rotational_moment(
    assignment: &[(UniqueId, usize)], // (pull_id, attachment_point_index)
    attachment_points: &[AttachmentPoint],
    pull_data: &[PullIntervalData],
    joint_positions: &[Point3<f32>],
    push_joint_index: usize,
) -> f32 {
    if assignment.is_empty() || attachment_points.is_empty() {
        return 0.0;
    }

    // The first (closest) attachment point is the pivot
    let pivot_position = attachment_points[0].position;

    // Accumulate moment vector
    let mut total_moment = Vector3::new(0.0, 0.0, 0.0);

    for (pull_id, attach_idx) in assignment {
        // Find the pull interval data
        if let Some(pull) = pull_data.iter().find(|p| p.id == *pull_id) {
            // Get attachment point position
            let attach_pos = attachment_points[*attach_idx].position;

            // Calculate moment arm: vector from pivot to attachment point
            let moment_arm = Vector3::new(
                attach_pos.x - pivot_position.x,
                attach_pos.y - pivot_position.y,
                attach_pos.z - pivot_position.z,
            );

            // Determine which end of the pull connects to this push
            let connected_joint = if pull.alpha_joint == push_joint_index {
                pull.alpha_joint
            } else {
                pull.omega_joint
            };

            // Calculate force vector: strain * unit direction
            // Direction is from the attachment point toward the pull's other end
            let other_joint = if connected_joint == pull.alpha_joint {
                pull.omega_joint
            } else {
                pull.alpha_joint
            };

            let other_pos = joint_positions[other_joint];
            let pull_direction = Vector3::new(
                other_pos.x - attach_pos.x,
                other_pos.y - attach_pos.y,
                other_pos.z - attach_pos.z,
            )
            .normalize();

            let force = pull_direction * pull.strain;

            // Calculate moment: r × F (cross product)
            let moment = moment_arm.cross(force);
            total_moment += moment;
        }
    }

    // Return magnitude of total moment
    total_moment.magnitude()
}

/// Finds the optimal assignment of pull intervals to attachment points
/// that minimizes rotational moment
fn find_optimal_assignment(
    pulls: &[(IntervalEnd, UniqueId, usize)], // (end, pull_id, joint_index)
    attachment_points: &[AttachmentPoint],
    pull_data: &[PullIntervalData],
    joint_positions: &[Point3<f32>],
    push_joint_index: usize,
) -> Vec<(IntervalEnd, UniqueId, usize)> {
    if pulls.is_empty() {
        return Vec::new();
    }

    let n = pulls.len();
    
    // If there's only one pull, no optimization needed
    if n == 1 {
        return pulls.to_vec();
    }

    // For small numbers (≤ 6), evaluate all permutations
    // Generate all permutations and find the one with minimum moment
    let mut best_moment = f32::MAX;

    // Use Heap's algorithm to generate permutations
    let mut indices: Vec<usize> = (0..n).collect();
    let mut result = pulls.to_vec();

    // Helper to evaluate current permutation
    let evaluate = |perm: &[usize]| -> f32 {
        let assignment: Vec<(UniqueId, usize)> = perm
            .iter()
            .enumerate()
            .map(|(attach_idx, &pull_idx)| (pulls[pull_idx].1, attach_idx))
            .collect();
        calculate_rotational_moment(
            &assignment,
            attachment_points,
            pull_data,
            joint_positions,
            push_joint_index,
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
            // Store this permutation
            for (i, &pull_idx) in perm.iter().enumerate() {
                result[i] = pulls[pull_idx];
            }
        }
    });

    result
}

/// Generates the positions of attachment points at the end of a push interval
///
/// # Parameters
/// * `end_position` - The position of the end of the push interval
/// * `direction` - The direction vector of the push interval (points outward from interval)
/// * `radius` - The radius of the push interval (used as base spacing)
pub fn generate_attachment_points(
    end_position: Point3<f32>,
    direction: Vector3<f32>,
    radius: f32,
) -> [AttachmentPoint; ATTACHMENT_POINTS] {
    // Normalize the direction vector to get the axis
    let axis = direction.normalize();

    // Calculate spacing between attachment points
    // Spacing equals the diameter of the rendered spheres so they are tangent (touching)
    // Sphere radius in renderer is Role::Pulling.appearance().radius * 0.12
    // Adjusted slightly larger to prevent overlap
    let sphere_diameter = radius * 0.026; // Diameter of each sphere
    let sphere_radius = sphere_diameter * 0.5; // Radius of each sphere

    // Create array to hold all attachment points
    let mut points = [AttachmentPoint {
        position: end_position,
        index: 0,
    }; ATTACHMENT_POINTS];

    // Generate attachment points extending outwards along the axis
    for i in 0..ATTACHMENT_POINTS {
        // Calculate distance from the end position
        // First sphere starts at sphere_radius so it's tangent with the cylinder cap
        // Subsequent spheres are spaced by sphere_diameter to be tangent with each other
        let distance = sphere_radius + sphere_diameter * (i as f32);
        
        // Calculate offset vector
        let offset = axis * distance;

        // Set the position and index
        points[i] = AttachmentPoint {
            position: Point3::new(
                end_position.x + offset.x,
                end_position.y + offset.y,
                end_position.z + offset.z,
            ),
            index: i,
        };
    }

    points
}

/// Calculates attachment points for both ends of a push interval
pub fn calculate_interval_attachment_points(
    start: Point3<f32>,
    end: Point3<f32>,
    radius: f32,
) -> (
    [AttachmentPoint; ATTACHMENT_POINTS],
    [AttachmentPoint; ATTACHMENT_POINTS],
) {
    // Calculate direction vector from start to end
    let direction = Vector3::new(end.x - start.x, end.y - start.y, end.z - start.z);

    // Generate attachment points at both ends
    // Alpha end: points extend outward from start (opposite to interval direction)
    // Omega end: points extend outward from end (in interval direction)
    (
        generate_attachment_points(start, -direction, radius),
        generate_attachment_points(end, direction, radius),
    )
}
