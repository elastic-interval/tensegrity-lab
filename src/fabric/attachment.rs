/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::fabric::dimensions::radial_unit_from_axis;
use crate::fabric::{FabricDimensions, IntervalEnd, IntervalKey, JointKey, Joints};
use crate::units::Unit;
use glam::Vec3;
use std::fmt;

/// Number of attachment points at each end of a push interval
pub const ATTACHMENT_POINTS: usize = 10;

/// Signed bend angle (degrees) at a cable end. Continuous ideal during
/// build, snapped to the optimised magnitude set after Viewing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TabBend(pub f32);

impl TabBend {
    pub fn degrees(&self) -> f32 {
        self.0
    }

    /// 0° = radial; +angle tilts toward push_axis (outward), −angle inward.
    pub fn endpoint(
        &self,
        tab_pos: Vec3,
        push_axis: Vec3,
        radial_direction: Vec3,
        tab_length: f32,
    ) -> Vec3 {
        let angle_rad = self.degrees().to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        let tab_direction = radial_direction * cos_a + push_axis * sin_a;
        tab_pos + tab_direction * tab_length
    }
}

impl fmt::Display for TabBend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let deg = self.0;
        if deg.abs() < 0.05 {
            write!(f, "0")
        } else if (deg - deg.round()).abs() < 0.05 {
            write!(f, "{:+}", deg.round() as i32)
        } else {
            write!(f, "{:+.1}", deg)
        }
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

    /// Reorders connections so each pull interval lands on the best slot:
    ///   - Hard rule: an outward-pulling cable may not occupy the topmost slot
    ///     (the final nut at the strut tip would otherwise carry the full axial load).
    ///   - Soft objective: among permutations satisfying the hard rule, pick the one
    ///     that maximises the minimum 3D distance between any pair of bent tab arms.
    ///   - Tie-break: rotational moment about slot 0, as before.
    pub fn reorder_connections(
        &mut self,
        alpha_attachment_points: &[AttachmentPoint],
        omega_attachment_points: &[AttachmentPoint],
        joints: &Joints,
        pull_intervals: &[(IntervalKey, JointKey, JointKey)], // (pull_id, alpha_key, omega_key)
        pull_data: &[PullIntervalData],
        push_alpha_key: JointKey,
        push_omega_key: JointKey,
        dimensions: &FabricDimensions,
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
            dimensions,
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
            dimensions,
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

/// Finds the optimal assignment of pull intervals to attachment points.
///
/// Hard rule:
///   An outward-pulling cable (positive dot with the outward strut axis) may
///   not occupy the topmost (highest-index) slot. If the only feasible
///   arrangement violates this, a warning is logged.
///
/// Soft rules (preferred over clearance), in priority order:
///   1. Lid choice: when the joint-end has any outward-pulling cable, the
///      cable whose tab-arm radial direction is most opposite (around the
///      strut axis) to the outward cable's should sit directly above it.
///      Their arms project on opposite sides of the strut, so the cover
///      disc never fouls the outward arm.
///   2. Outward placement: the topmost outward-pulling cable should sit at
///      slot n−2 (second from the top), leaving only the lid above it. This
///      keeps the outward arm's axial reach clear of the rest of the stack.
///
/// Soft objective:
///   Among permutations satisfying the above, pick the one that maximises
///   the minimum 3D distance between any pair of bent tab arms (modelled
///   as line segments from `tab_pos` to `pull_end_pos`).
///
/// Tie-break:
///   Rotational moment about the slot-0 ring centre, as before.
fn find_optimal_assignment(
    pulls: &[(IntervalEnd, IntervalKey, JointKey)], // (end, pull_id, joint_key)
    attachment_points: &[AttachmentPoint],
    pull_data: &[PullIntervalData],
    joints: &Joints,
    push_joint_key: JointKey,
    push_end: Vec3,
    push_axis: Vec3,
    dimensions: &FabricDimensions,
) -> Vec<(IntervalEnd, IntervalKey, JointKey)> {
    if pulls.is_empty() {
        return Vec::new();
    }
    if pulls.len() == 1 {
        return pulls.to_vec();
    }

    let n = pulls.len();

    // Per-cable metadata: far-end position, is_outward, and the radial unit
    // vector around the strut axis (slot-independent — only the radial
    // component of (far_end - ring_centre) determines it).
    let push_pos = joints[push_joint_key].location;
    let mut other_ends: Vec<Vec3> = Vec::with_capacity(n);
    let mut is_outward: Vec<bool> = Vec::with_capacity(n);
    let mut radials: Vec<Vec3> = Vec::with_capacity(n);
    for pull in pulls {
        if let Some(data) = pull_data.iter().find(|d| d.key == pull.1) {
            let other_key = if data.alpha_key == push_joint_key {
                data.omega_key
            } else {
                data.alpha_key
            };
            let other_pos = joints[other_key].location;
            let axial = (other_pos - push_pos).normalize().dot(push_axis);
            other_ends.push(other_pos);
            is_outward.push(axial > 0.0);
            radials.push(radial_unit_from_axis(push_axis, other_pos - push_pos));
        } else {
            other_ends.push(Vec3::ZERO);
            is_outward.push(false);
            radials.push(Vec3::ZERO);
        }
    }

    // For each potential outward cable o, the best "lid" is the cable whose
    // radial direction is most opposite to o's (smallest dot product). The
    // lookup is per-cable because the outward cable's identity depends on
    // the permutation we're scoring.
    let best_lid_for: Vec<usize> = (0..n)
        .map(|o| {
            (0..n)
                .filter(|&c| c != o)
                .min_by(|&a, &b| {
                    let da = radials[a].dot(radials[o]);
                    let db = radials[b].dot(radials[o]);
                    da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap_or(o)
        })
        .collect();
    let any_outward = is_outward.iter().any(|&o| o);

    // Precompute arm segments (tab_pos, pull_end_pos) per (cable, slot).
    // Bend angle and ring centre both depend on slot, so we recompute per slot.
    // We only need slots 0..n (one slot per cable).
    let mut segments: Vec<Vec<(Vec3, Vec3)>> = Vec::with_capacity(n);
    for c in 0..n {
        let mut row = Vec::with_capacity(n);
        for k in 0..n {
            let (tab_pos, _bend, pull_end_pos, _ideal) =
                dimensions.tab_geometry(push_end, push_axis, k, other_ends[c]);
            row.push((tab_pos, pull_end_pos));
        }
        segments.push(row);
    }

    // Score: (outward_at_top, lid_miss, outward_low_miss, -min_clearance, moment).
    let mut best_outward_top: u32 = u32::MAX;
    let mut best_lid_miss: u32 = u32::MAX;
    let mut best_outward_low_miss: u32 = u32::MAX;
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
        // Hard rule: outward cable at topmost slot.
        let outward_at_top: u32 = if is_outward[perm[n - 1]] { 1 } else { 0 };

        let top_outward_slot: Option<usize> =
            if any_outward { (0..n).rev().find(|&s| is_outward[perm[s]]) } else { None };

        // Soft rule 1: the cable whose radial direction is most opposite the
        // topmost outward cable's should sit directly above it.
        let lid_miss: u32 = match top_outward_slot {
            Some(s) if s + 1 < n => {
                let outward_cable = perm[s];
                if perm[s + 1] == best_lid_for[outward_cable] { 0 } else { 1 }
            }
            _ => 0, // None, or outward at top (latter caught by outward_at_top)
        };

        // Soft rule 2: the topmost outward cable should sit at slot n-2.
        let outward_low_miss: u32 = match top_outward_slot {
            Some(s) if s == n - 2 => 0,
            None => 0,
            _ => 1,
        };

        // Minimum 3D distance between any pair of bent arms.
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

        // Lex comparison: (outward_at_top, lid_miss, outward_low_miss, -clearance, moment)
        let better = if outward_at_top != best_outward_top {
            outward_at_top < best_outward_top
        } else if lid_miss != best_lid_miss {
            lid_miss < best_lid_miss
        } else if outward_low_miss != best_outward_low_miss {
            outward_low_miss < best_outward_low_miss
        } else if neg_clearance != best_neg_clearance {
            neg_clearance < best_neg_clearance
        } else {
            moment < best_moment
        };

        if better {
            best_outward_top = outward_at_top;
            best_lid_miss = lid_miss;
            best_outward_low_miss = outward_low_miss;
            best_neg_clearance = neg_clearance;
            best_moment = moment;
            best_order.clear();
            best_order.extend_from_slice(perm);
        }
    });

    if best_outward_top > 0 {
        eprintln!(
            "warning: disc-ordering at push joint {:?}: outward-pulling cable forced \
             to topmost slot (no other arrangement available).",
            push_joint_key
        );
    }

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
        let distance = dimensions.connector.disc_center_offset(i).f32();

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
