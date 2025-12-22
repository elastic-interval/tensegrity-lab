/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::fabric::{IntervalEnd, IntervalKey, JointKey, Joints};
use crate::units::{Degrees, Meters};
use cgmath::{InnerSpace, MetricSpace, Point3, Vector3};

/// Number of attachment points at each end of a push interval
pub const ATTACHMENT_POINTS: usize = 10;

/// All physical dimensions for a fabric: structure size and interval geometry.
#[derive(Clone, Copy, Debug)]
pub struct FabricDimensions {
    pub altitude: Meters,
    pub scale: Meters,
    pub push_radius: Meters,
    pub pull_radius: Meters,
    pub ring_thickness: Meters,
    pub hinge_offset: Meters,
    pub hinge_length: Meters,
}

impl FabricDimensions {
    /// Full-size dimensions for real structures (scale 1.0m, altitude 7.5m)
    pub fn full_size() -> Self {
        Self {
            altitude: Meters(7.5),
            scale: Meters(1.0),
            push_radius: Meters(0.060),     // 60mm
            pull_radius: Meters(0.007),     // 7mm
            ring_thickness: Meters(0.012),  // 12mm
            hinge_offset: Meters(0.063),    // 63mm
            hinge_length: Meters(0.100),    // 100mm
        }
    }

    /// Model-size dimensions for small physical models (scale 0.056m, altitude 0.5m)
    pub fn model_size() -> Self {
        Self {
            altitude: Meters(0.5),
            scale: Meters(0.056),
            push_radius: Meters(0.003),     // 3mm
            pull_radius: Meters(0.0005),    // 0.5mm
            ring_thickness: Meters(0.001),  // 1mm
            hinge_offset: Meters(0.004),    // 4mm
            hinge_length: Meters(0.006),    // 6mm
        }
    }

    /// Set custom altitude
    pub fn with_altitude(mut self, altitude: Meters) -> Self {
        self.altitude = altitude;
        self
    }

    /// Set custom scale
    pub fn with_scale(mut self, scale: Meters) -> Self {
        self.scale = scale;
        self
    }

    /// Interval dimensions only (for backward compatibility)
    pub fn interval_dimensions(&self) -> IntervalDimensions {
        IntervalDimensions {
            push_radius: self.push_radius,
            pull_radius: self.pull_radius,
            ring_thickness: self.ring_thickness,
            hinge_offset: self.hinge_offset,
            hinge_length: self.hinge_length,
        }
    }
}

/// Interval geometry dimensions (subset of FabricDimensions for rendering/export)
#[derive(Clone, Copy, Debug)]
pub struct IntervalDimensions {
    pub push_radius: Meters,
    pub pull_radius: Meters,
    pub ring_thickness: Meters,
    pub hinge_offset: Meters,
    pub hinge_length: Meters,
}

impl Default for IntervalDimensions {
    fn default() -> Self {
        FabricDimensions::full_size().interval_dimensions()
    }
}

impl IntervalDimensions {
    pub fn for_scale(scale: f32) -> Self {
        Self::default().scaled(scale)
    }

    pub fn scaled(&self, scale: f32) -> Self {
        Self {
            push_radius: self.push_radius * scale,
            pull_radius: self.pull_radius * scale,
            ring_thickness: self.ring_thickness * scale,
            hinge_offset: self.hinge_offset * scale,
            hinge_length: self.hinge_length * scale,
        }
    }

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
        // Ring center position on the bolt (1x, 2x, 3x ring_thickness for slots 0, 1, 2)
        let axial_offset = *self.ring_thickness * (slot as f32 + 1.0);
        let ring_center = push_end + push_axis * axial_offset;

        // Direction from ring center toward the pull's other end, projected onto ring plane
        let to_pull = pull_other_end - ring_center;
        let axial_component = push_axis * to_pull.dot(push_axis);
        let radial_direction = to_pull - axial_component;

        let radial_unit = if radial_direction.magnitude2() < 1e-10 {
            let arbitrary = if push_axis.x.abs() < 0.9 {
                Vector3::new(1.0, 0.0, 0.0)
            } else {
                Vector3::new(0.0, 1.0, 0.0)
            };
            push_axis.cross(arbitrary).normalize()
        } else {
            radial_direction.normalize()
        };

        ring_center + radial_unit * *self.hinge_offset
    }

    /// Calculate the ideal hinge angle for a pull interval at its connection point
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
    /// Ideal hinge angle as Degrees (not snapped)
    pub fn hinge_angle(push_axis: Vector3<f32>, pull_direction: Vector3<f32>) -> Degrees {
        // The hinge angle is the angle between pull direction and the ring plane.
        // The ring plane is perpendicular to push_axis.
        // sin(hinge_angle) = dot(pull_direction, push_axis)
        let sin_angle = pull_direction.dot(push_axis);
        Degrees(sin_angle.asin().to_degrees())
    }

    /// Calculate hinge position, snapped angle, and endpoint for a pull interval connection
    ///
    /// # Returns
    /// (hinge_pos, hinge_bend, pull_end_pos)
    pub fn hinge_geometry(
        &self,
        push_end: Point3<f32>,
        push_axis: Vector3<f32>,
        slot: usize,
        pull_other_end: Point3<f32>,
    ) -> (Point3<f32>, HingeBend, Point3<f32>) {
        // Ring center position on the bolt
        let axial_offset = *self.ring_thickness * (slot as f32 + 1.0);
        let ring_center = push_end + push_axis * axial_offset;

        // Direction from ring center toward the pull's other end, projected onto ring plane
        let to_pull = pull_other_end - ring_center;
        let axial_component = push_axis * to_pull.dot(push_axis);
        let radial_direction = to_pull - axial_component;

        let radial_unit = if radial_direction.magnitude2() < 1e-10 {
            let arbitrary = if push_axis.x.abs() < 0.9 {
                Vector3::new(1.0, 0.0, 0.0)
            } else {
                Vector3::new(0.0, 1.0, 0.0)
            };
            push_axis.cross(arbitrary).normalize()
        } else {
            radial_direction.normalize()
        };

        let hinge_pos = ring_center + radial_unit * *self.hinge_offset;

        // Calculate ideal angle and snap to nearest HingeBend
        let pull_direction = (pull_other_end - hinge_pos).normalize();
        let ideal_angle = Self::hinge_angle(push_axis, pull_direction);
        let hinge_bend = HingeBend::from_angle(ideal_angle);

        // Calculate endpoint using snapped angle
        let pull_end_pos = hinge_bend.endpoint(
            hinge_pos,
            push_axis,
            radial_unit,
            *self.hinge_length,
        );

        (hinge_pos, hinge_bend, pull_end_pos)
    }
}

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
        hinge_pos: Point3<f32>,
        push_axis: Vector3<f32>,
        radial_direction: Vector3<f32>,
        hinge_length: f32,
    ) -> Point3<f32> {
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

/// Type alias for backward compatibility during migration
pub type ConnectorSpec = IntervalDimensions;

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
    joints: &Joints,
    push_joint_key: JointKey,
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
    push_joint_key: JointKey,
    push_axis: Vector3<f32>,
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
    push_axis: Vector3<f32>,
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
        let distance = *connector.ring_thickness * (i as f32 + 0.5);

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
