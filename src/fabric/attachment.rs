/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::fabric::{IntervalEnd, IntervalKey};
use crate::units::Degrees;
use cgmath::{InnerSpace, MetricSpace, Point3, Vector3};

/// Number of attachment points at each end of a push interval
pub const ATTACHMENT_POINTS: usize = 10;

/// Connector geometry specification for ring connectors
/// Each pull interval connects via a ring stacked on a bolt extending from the push interval end.
/// The hinge (connection point) is at the edge of the ring, offset radially from the bolt axis.
/// All dimensions scale proportionally with the fabric scale.
pub struct ConnectorSpec {
    /// Distance between ring centers along the bolt (thickness of each ring)
    pub ring_thickness: f32,
    /// Radial distance from bolt axis to hinge point (just outside interval radius)
    pub hinge_offset: f32,
}

impl ConnectorSpec {
    /// Create a connector spec scaled appropriately for the fabric
    /// Base dimensions are for scale 1.0 (1 meter structures)
    /// Push interval radius is 0.04 * 1.2 * scale = 0.048 * scale
    pub fn for_scale(scale: f32) -> Self {
        Self {
            ring_thickness: 0.016 * scale, // 16mm at scale 1.0 (doubled from 8mm)
            hinge_offset: 0.04 * 1.2 * scale, // Match disc radius exactly (48mm at scale 1.0)
        }
    }
}

impl Default for ConnectorSpec {
    fn default() -> Self {
        Self::for_scale(1.0)
    }
}

impl ConnectorSpec {
    /// Calculate the hinge position for a pull interval connection
    ///
    /// # Parameters
    /// * `push_end` - Position of the push interval end (where the bolt starts)
    /// * `push_axis` - Outward unit vector along the bolt (away from push interval)
    /// * `slot` - Which ring slot (0 = closest to push end)
    /// * `pull_other_end` - Position of the other end of the pull interval
    ///
    /// # Returns
    /// The 3D position of the hinge point
    pub fn hinge_position(
        &self,
        push_end: Point3<f32>,
        push_axis: Vector3<f32>,
        slot: usize,
        pull_other_end: Point3<f32>,
    ) -> Point3<f32> {
        // Ring center position on the bolt
        let axial_offset = self.ring_thickness * (slot as f32 + 0.5);
        let ring_center = push_end + push_axis * axial_offset;

        // Direction from ring center toward the pull's other end, projected onto ring plane
        let to_pull = pull_other_end - ring_center;
        // Remove the axial component to get the radial direction
        let axial_component = push_axis * to_pull.dot(push_axis);
        let radial_direction = to_pull - axial_component;

        // If the radial direction is zero (pull is exactly along axis), pick an arbitrary direction
        let radial_unit = if radial_direction.magnitude2() < 1e-10 {
            // Find any vector perpendicular to push_axis
            let arbitrary = if push_axis.x.abs() < 0.9 {
                Vector3::new(1.0, 0.0, 0.0)
            } else {
                Vector3::new(0.0, 1.0, 0.0)
            };
            push_axis.cross(arbitrary).normalize()
        } else {
            radial_direction.normalize()
        };

        // Hinge point is at radial offset from the ring center
        ring_center + radial_unit * self.hinge_offset
    }

    /// Calculate the hinge angle for a pull interval at its connection point
    ///
    /// The hinge angle is the angle between the pull direction and the ring plane.
    /// - 0° means pulling in the ring plane (perpendicular to bolt)
    /// - +90° means pulling straight outward along the bolt
    /// - -90° means pulling straight inward toward the push interval
    ///
    /// # Parameters
    /// * `push_axis` - Outward unit vector along the bolt
    /// * `pull_direction` - Unit vector of pull direction (toward the other end of pull interval)
    ///
    /// # Returns
    /// Hinge angle as Degrees
    pub fn hinge_angle(push_axis: Vector3<f32>, pull_direction: Vector3<f32>) -> Degrees {
        // The hinge angle is the angle between pull direction and the ring plane.
        // The ring plane is perpendicular to push_axis.
        // sin(hinge_angle) = dot(pull_direction, push_axis)
        let sin_angle = pull_direction.dot(push_axis);
        Degrees(sin_angle.asin().to_degrees())
    }
}

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
        joint_positions: &[Point3<f32>],
        pull_intervals: &[(IntervalKey, usize, usize)], // (pull_id, alpha_index, omega_index)
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

        // Calculate push axis: direction from alpha to omega
        let alpha_pos = joint_positions[push_alpha_index];
        let omega_pos = joint_positions[push_omega_index];
        let push_direction = (omega_pos - alpha_pos).normalize();

        // Step 4: Find optimal assignment for each end using moment minimization
        // Alpha end: push axis points outward (opposite to push direction)
        let optimized_alpha = find_optimal_assignment(
            &alpha_connections,
            alpha_attachment_points,
            pull_data,
            joint_positions,
            push_alpha_index,
            -push_direction, // Outward from alpha end
        );

        // Omega end: push axis points outward (same as push direction)
        let optimized_omega = find_optimal_assignment(
            &omega_connections,
            omega_attachment_points,
            pull_data,
            joint_positions,
            push_omega_index,
            push_direction, // Outward from omega end
        );

        // Step 5: Assign connections using optimized order
        for (attach_idx, (end, pull_id, _joint_index)) in optimized_alpha.iter().enumerate() {
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

        for (attach_idx, (end, pull_id, _joint_index)) in optimized_omega.iter().enumerate() {
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
        pull_intervals: &[(IntervalKey, usize, usize)],
        push_alpha_index: usize,
        push_omega_index: usize,
    ) -> Vec<(IntervalEnd, IntervalKey, usize)> {
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

    for (pull_key, attach_idx) in assignment {
        // Find the pull interval data
        if let Some(pull) = pull_data.iter().find(|p| p.key == *pull_key) {
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

/// Checks if a pull interval is "outward-pulling" (positive dot product with push axis)
/// These must be assigned to the lowest slot (slot 0)
fn is_outward_pulling(
    pull: &PullIntervalData,
    push_joint_index: usize,
    push_axis: Vector3<f32>,
    joint_positions: &[Point3<f32>],
) -> bool {
    // Determine which end of the pull connects to this push joint
    let other_joint = if pull.alpha_joint == push_joint_index {
        pull.omega_joint
    } else {
        pull.alpha_joint
    };

    // Pull direction: from push joint toward the other end
    let push_pos = joint_positions[push_joint_index];
    let other_pos = joint_positions[other_joint];
    let pull_direction = (other_pos - push_pos).normalize();

    // Positive dot product means pulling outward along the push axis
    pull_direction.dot(push_axis) > 0.0
}

/// Finds the optimal assignment of pull intervals to attachment points
/// that minimizes rotational moment.
/// Outward-pulling intervals (positive dot with push axis) are forced to slot 0.
fn find_optimal_assignment(
    pulls: &[(IntervalEnd, IntervalKey, usize)], // (end, pull_id, joint_index)
    attachment_points: &[AttachmentPoint],
    pull_data: &[PullIntervalData],
    joint_positions: &[Point3<f32>],
    push_joint_index: usize,
    push_axis: Vector3<f32>,
) -> Vec<(IntervalEnd, IntervalKey, usize)> {
    if pulls.is_empty() {
        return Vec::new();
    }

    // Separate outward-pulling intervals from inward-pulling ones
    let mut outward_pulls = Vec::new();
    let mut inward_pulls = Vec::new();

    for pull in pulls {
        if let Some(data) = pull_data.iter().find(|d| d.key == pull.1) {
            if is_outward_pulling(data, push_joint_index, push_axis, joint_positions) {
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
/// * `connector` - The connector spec with scaled dimensions
pub fn generate_attachment_points(
    end_position: Point3<f32>,
    direction: Vector3<f32>,
    connector: &ConnectorSpec,
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
        // Ring center is at slot index * ring_thickness + half ring thickness
        let distance = connector.ring_thickness * (i as f32 + 0.5);

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
    connector: &ConnectorSpec,
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
        generate_attachment_points(start, -direction, connector),
        generate_attachment_points(end, direction, connector),
    )
}
