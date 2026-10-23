/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::connector::{fork, ConnectorDimensions};
use crate::fabric::{IntervalEnd, IntervalKey, JointKey, Joints};
use crate::units::{Meters, Unit};
use glam::Vec3;

/// Number of attachment points at each end of a push interval
pub const ATTACHMENT_POINTS: usize = 10;

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

    /// How far this cable's boss reaches from the strut axis when it is on a
    /// custom disc; `None` for a standard ring.
    pub reach: Option<Meters>,

    /// Extra washers stacked below this cable's ring, counting every gap
    /// below it, beyond the one washer each gap has.
    pub lift: u8,
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

    /// Reorders connections so each pull interval lands on the best slot (see
    /// `find_optimal_assignment`): fewest colliding forks, then most arm
    /// clearance, then least rotational moment.
    pub fn reorder_connections(
        &mut self,
        alpha_attachment_points: &[AttachmentPoint],
        omega_attachment_points: &[AttachmentPoint],
        joints: &Joints,
        pull_intervals: &[(IntervalKey, JointKey, JointKey)], // (pull_id, alpha_key, omega_key)
        pull_data: &[PullIntervalData],
        push_alpha_key: JointKey,
        push_omega_key: JointKey,
        (alpha_pos, omega_pos): (Vec3, Vec3), // where the two stacks start
        connector: &ConnectorDimensions,
        ring_radius: f32,
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
        let push_direction = (omega_pos - alpha_pos).normalize();

        // Step 4: Find optimal assignment for each end.
        // Alpha end: push axis points outward (opposite to push direction)
        let optimized_alpha = find_optimal_assignment(
            &alpha_connections,
            alpha_attachment_points,
            pull_data,
            joints,
            push_alpha_key,
            alpha_pos,
            -push_direction, // Outward from alpha end
            connector,
            ring_radius,
        );

        // Omega end: push axis points outward (same as push direction)
        let optimized_omega = find_optimal_assignment(
            &omega_connections,
            omega_attachment_points,
            pull_data,
            joints,
            push_omega_key,
            omega_pos,
            push_direction, // Outward from omega end
            connector,
            ring_radius,
        );

        // Step 5: Assign connections using optimized order
        for (attach_idx, (end, pull_id, _joint_key)) in optimized_alpha.iter().enumerate() {
            if attach_idx < ATTACHMENT_POINTS {
                let connection = PullConnection {
                    pull_interval_key: *pull_id,
                    attachment_index: attach_idx,
                    reach: None,
                    lift: 0,
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
                    reach: None,
                    lift: 0,
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
pub fn find_nearest_attachment_point(points: &[AttachmentPoint], position: Vec3) -> (usize, f32) {
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

/// Finds the optimal assignment of pull intervals to attachment points,
/// trying every order of the cables on the stack. There are no ordering
/// rules: any cable may take any slot.
///
/// Objective:
///   Fewest pairs of forks whose capsule models (`connector::fork`) collide
///   — the same test that marks culprits.
///
/// Then:
///   Maximise the minimum 3D distance between any pair of connector arms
///   (modelled as line segments from the ring centre to the pivot pin).
///
/// Tie-break:
///   Rotational moment about the slot-0 ring centre.
fn find_optimal_assignment(
    pulls: &[(IntervalEnd, IntervalKey, JointKey)], // (end, pull_id, joint_key)
    attachment_points: &[AttachmentPoint],
    pull_data: &[PullIntervalData],
    joints: &Joints,
    push_joint_key: JointKey,
    push_end: Vec3,
    push_axis: Vec3,
    connector: &ConnectorDimensions,
    ring_radius: f32,
) -> Vec<(IntervalEnd, IntervalKey, JointKey)> {
    if pulls.is_empty() {
        return Vec::new();
    }
    if pulls.len() == 1 {
        return pulls.to_vec();
    }

    let n = pulls.len();

    // Each cable's far-end position.
    let other_ends: Vec<Vec3> = pulls
        .iter()
        .map(|pull| match pull_data.iter().find(|d| d.key == pull.1) {
            Some(data) => {
                let other_key = if data.alpha_key == push_joint_key {
                    data.omega_key
                } else {
                    data.alpha_key
                };
                joints[other_key].location
            }
            None => Vec3::ZERO,
        })
        .collect();

    // Precompute arm segments (ring_center, pivot_pos) per (cable, slot).
    // The pivot azimuth and ring centre both depend on slot, so we recompute
    // per slot. We only need slots 0..n (one slot per cable).
    let mut segments: Vec<Vec<(Vec3, Vec3)>> = Vec::with_capacity(n);
    for c in 0..n {
        let mut row = Vec::with_capacity(n);
        for k in 0..n {
            let ring_center = connector.ring_center(push_end, push_axis, k);
            let (pivot_pos, _elevation) =
                connector.pivot_geometry(push_end, push_axis, k, other_ends[c]);
            row.push((ring_center, pivot_pos));
        }
        segments.push(row);
    }
    let capsules: Vec<Vec<_>> = (0..n)
        .map(|c| {
            segments[c]
                .iter()
                .map(|&(ring_center, pivot)| {
                    let tangent = push_axis.cross((pivot - ring_center).normalize()).normalize();
                    fork::capsules(pivot, tangent, (other_ends[c] - pivot).normalize())
                        .with_arm(ring_center, pivot, ring_radius)
                })
                .collect()
        })
        .collect();

    // Score: (collisions, -min_clearance, moment).
    let mut best_collisions: u32 = u32::MAX;
    let mut best_neg_clearance: f32 = f32::MAX;
    let mut best_moment: f32 = f32::MAX;
    let mut best_order: Vec<usize> = (0..n).collect();

    let mut indices: Vec<usize> = (0..n).collect();

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
        let mut collisions: u32 = 0;
        for i in 0..n {
            for j in (i + 1)..n {
                if fork::assemblies_collide(&capsules[perm[i]][i], &capsules[perm[j]][j]) {
                    collisions += 1;
                }
            }
        }

        // Minimum 3D distance between any pair of connector arms.
        let mut min_clearance = f32::INFINITY;
        for i in 0..n {
            let (h_i, e_i) = segments[perm[i]][i];
            for j in (i + 1)..n {
                let (h_j, e_j) = segments[perm[j]][j];
                let d = segment_segment_distance(h_i, e_i, h_j, e_j);
                if d < min_clearance {
                    min_clearance = d;
                }
            }
        }
        let neg_clearance = -min_clearance;

        // Rotational moment as final tiebreak.
        let assignment: Vec<(IntervalKey, usize)> = perm
            .iter()
            .enumerate()
            .map(|(slot, &pull_idx)| (pulls[pull_idx].1, slot))
            .collect();
        let moment = calculate_rotational_moment(
            &assignment,
            attachment_points,
            pull_data,
            joints,
            push_joint_key,
        );

        // Lex comparison: (collisions, -clearance, moment)
        let better = if collisions != best_collisions {
            collisions < best_collisions
        } else if neg_clearance != best_neg_clearance {
            neg_clearance < best_neg_clearance
        } else {
            moment < best_moment
        };

        if better {
            best_collisions = collisions;
            best_neg_clearance = neg_clearance;
            best_moment = moment;
            best_order.clear();
            best_order.extend_from_slice(perm);
        }
    });

    best_order.into_iter().map(|i| pulls[i]).collect()
}

/// Minimum distance between two 3D line segments (closed-form).
/// Reference: Lumelsky 1985 / Eberly's "Geometric Tools" segment-segment routine.
pub(crate) fn segment_segment_distance(p1: Vec3, p2: Vec3, p3: Vec3, p4: Vec3) -> f32 {
    let d1 = p2 - p1;
    let d2 = p4 - p3;
    let r = p1 - p3;

    let a = d1.length_squared();
    let e = d2.length_squared();
    let f = d2.dot(r);

    const EPS: f32 = 1e-10;

    let (s, t) = if a <= EPS && e <= EPS {
        (0.0_f32, 0.0_f32)
    } else if a <= EPS {
        (0.0, (f / e).clamp(0.0, 1.0))
    } else {
        let c = d1.dot(r);
        if e <= EPS {
            ((-c / a).clamp(0.0, 1.0), 0.0)
        } else {
            let b = d1.dot(d2);
            let denom = a * e - b * b;
            let s0 = if denom.abs() > EPS {
                ((b * f - c * e) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let t0 = (b * s0 + f) / e;
            if t0 < 0.0 {
                ((-c / a).clamp(0.0, 1.0), 0.0)
            } else if t0 > 1.0 {
                (((b - c) / a).clamp(0.0, 1.0), 1.0)
            } else {
                (s0, t0)
            }
        }
    };

    let c1 = p1 + d1 * s;
    let c2 = p3 + d2 * t;
    c1.distance(c2)
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
    connector: &ConnectorDimensions,
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
        let distance = connector.ring_center_offset(i).f32();

        // Set the position and index
        points[i] = AttachmentPoint {
            position: end_position + axis * distance,
            index: i,
        };
    }

    points
}

/// Calculates attachment points for both ends of a push interval
pub fn calculate_interval_attachment_points(
    start: Vec3,
    end: Vec3,
    connector: &ConnectorDimensions,
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
        generate_attachment_points(start, -direction, connector),
        generate_attachment_points(end, direction, connector),
    )
}
