/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::fabric::{IntervalEnd, UniqueId};
use cgmath::{InnerSpace, MetricSpace, Point3, Vector3};
use std::f32::consts::PI;

/// Number of attachment points at each end of a push interval
pub const ATTACHMENT_POINTS: usize = 10;

/// Represents an attachment point on a push interval
#[derive(Clone, Copy, Debug)]
pub struct AttachmentPoint {
    /// The position of the attachment point in 3D space
    pub position: Point3<f32>,

    /// The index of this attachment point (0-9)
    pub index: usize,
}

/// Represents a connection between a pull interval and an attachment point
#[derive(Clone, Copy, Debug)]
pub struct AttachmentConnection {
    /// The ID of the pull interval that is attached
    pub pull_interval_id: UniqueId,

    /// The attachment point index where the pull interval is connected
    pub attachment_index: usize,
}

/// Encapsulates the array of connections between intervals
#[derive(Clone, Debug)]
pub struct AttachmentConnections {
    pub alpha: [Option<AttachmentConnection>; ATTACHMENT_POINTS],
    pub omega: [Option<AttachmentConnection>; ATTACHMENT_POINTS],
}

impl AttachmentConnections {
    /// Creates a new empty set of connections
    pub fn new() -> Self {
        Self {
            alpha: [None; ATTACHMENT_POINTS],
            omega: [None; ATTACHMENT_POINTS],
        }
    }

    /// Returns the connections array for the specified end
    pub fn connections(
        &self,
        end: IntervalEnd,
    ) -> &[Option<AttachmentConnection>; ATTACHMENT_POINTS] {
        match end {
            IntervalEnd::Alpha => &self.alpha,
            IntervalEnd::Omega => &self.omega,
        }
    }

    /// Adds a connection to the next available slot at the specified end
    /// Panics if all slots are full
    pub fn add_connection(&mut self, end: IntervalEnd, connection: AttachmentConnection) {
        let array = match end {
            IntervalEnd::Alpha => &mut self.alpha,
            IntervalEnd::Omega => &mut self.omega,
        };

        for slot in array.iter_mut() {
            if slot.is_none() {
                *slot = Some(connection);
                return;
            }
        }
        panic!("No available {} connection slots", end.as_str());
    }

    /// Clears all connections
    pub fn clear(&mut self) {
        // Helper function to clear an array of connections
        let clear_array = |array: &mut [Option<AttachmentConnection>; ATTACHMENT_POINTS]| {
            for connection in array.iter_mut() {
                *connection = None;
            }
        };

        // Clear connections for both ends
        clear_array(&mut self.alpha);
        clear_array(&mut self.omega);
    }

    /// Clears connections for a specific end
    pub fn clear_end(&mut self, end: IntervalEnd) {
        let array = match end {
            IntervalEnd::Alpha => &mut self.alpha,
            IntervalEnd::Omega => &mut self.omega,
        };

        for connection in array.iter_mut() {
            *connection = None;
        }
    }

    /// Reorders connections to ensure each pull interval is assigned to the most appropriate attachment point
    /// This optimizes the positions of connections based on joint positions
    pub fn reorder_connections(
        &mut self,
        alpha_attachment_points: &[AttachmentPoint],
        omega_attachment_points: &[AttachmentPoint],
        joint_positions: &[Point3<f32>],
        pull_intervals: &[(UniqueId, usize, usize)], // (pull_id, alpha_index, omega_index)
        push_alpha_index: usize,
        push_omega_index: usize,
    ) {
        // Step 1: Collect all connections that need to be made
        let mut connections_to_make = self.collect_connections_to_make(pull_intervals, push_alpha_index, push_omega_index);
        
        // Step 2: Clear all existing connections
        self.alpha = [None; ATTACHMENT_POINTS];
        self.omega = [None; ATTACHMENT_POINTS];
        
        // Step 3: Sort connections by distance to optimize placement
        self.sort_connections_by_distance(&mut connections_to_make, alpha_attachment_points, omega_attachment_points, joint_positions);
        
        // Step 4: Assign connections to attachment points
        self.assign_connections(connections_to_make, alpha_attachment_points, omega_attachment_points, joint_positions);
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
    
    /// Sorts connections by distance to optimize placement
    fn sort_connections_by_distance(
        &self,
        connections_to_make: &mut Vec<(IntervalEnd, UniqueId, usize)>,
        alpha_attachment_points: &[AttachmentPoint],
        omega_attachment_points: &[AttachmentPoint],
        joint_positions: &[Point3<f32>],
    ) {
        connections_to_make.sort_by(|(end_a, _, joint_a), (end_b, _, joint_b)| {
            // Get the attachment points for each end
            let points_a = self.get_attachment_points_for_end(*end_a, alpha_attachment_points, omega_attachment_points);
            let points_b = self.get_attachment_points_for_end(*end_b, alpha_attachment_points, omega_attachment_points);
            
            // Calculate the minimum distance for each connection
            let min_dist_a = Self::calculate_min_distance(*joint_a, points_a, joint_positions);
            let min_dist_b = Self::calculate_min_distance(*joint_b, points_b, joint_positions);
            
            // Sort by distance (closest first)
            min_dist_a.partial_cmp(&min_dist_b).unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    
    /// Assigns connections to attachment points
    fn assign_connections(
        &mut self,
        connections_to_make: Vec<(IntervalEnd, UniqueId, usize)>,
        alpha_attachment_points: &[AttachmentPoint],
        omega_attachment_points: &[AttachmentPoint],
        joint_positions: &[Point3<f32>],
    ) {
        for (end, pull_id, joint_index) in connections_to_make {
            let attachment_points = self.get_attachment_points_for_end(end, alpha_attachment_points, omega_attachment_points);
            
            // Find the best available attachment point
            let best_idx = match end {
                IntervalEnd::Alpha => Self::find_best_attachment_point(
                    attachment_points, joint_positions, joint_index, &self.alpha),
                IntervalEnd::Omega => Self::find_best_attachment_point(
                    attachment_points, joint_positions, joint_index, &self.omega),
            };
            
            // Assign to the closest available attachment point
            if let Some(idx) = best_idx {
                let connection = AttachmentConnection {
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
    fn find_best_attachment_point(
        attachment_points: &[AttachmentPoint],
        joint_positions: &[Point3<f32>],
        joint_index: usize,
        target_array: &[Option<AttachmentConnection>; ATTACHMENT_POINTS],
    ) -> Option<usize> {
        // Calculate distances to all attachment points
        let mut distances = Vec::new();
        for (i, point) in attachment_points.iter().enumerate() {
            if target_array[i].is_none() { // Only consider unoccupied points
                let joint_position = joint_positions[joint_index];
                let distance = joint_position.distance2(point.position);
                distances.push((i, distance));
            }
        }
        
        // Sort by distance (closest first)
        distances.sort_by(|(_, dist1), (_, dist2)| {
            dist1.partial_cmp(dist2).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Return the closest available attachment point
        distances.first().map(|(idx, _)| *idx)
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
            .min_by(|dist1, dist2| dist1.partial_cmp(dist2).unwrap_or(std::cmp::Ordering::Equal))
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
    pub fn get_connection(&self, end: IntervalEnd, index: usize) -> Option<&AttachmentConnection> {
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
    ) -> Option<&mut AttachmentConnection> {
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
        connection: Option<AttachmentConnection>,
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

/// Calculates the positions of attachment points at the end of a push interval
pub fn calculate_attachment_points(
    end_position: Point3<f32>,
    direction: Vector3<f32>,
    radius: f32,
    up_vector: Vector3<f32>,
) -> [AttachmentPoint; ATTACHMENT_POINTS] {
    // Normalize the direction vector
    let axis = direction.normalize();

    // Create a perpendicular vector to serve as the reference for attachment point 0
    // We use the up vector as a reference to ensure consistent orientation
    let reference = if up_vector.magnitude2() < 0.001 {
        // If up vector is too small, use a default
        let default_up = Vector3::new(0.0, 1.0, 0.0);
        if axis.dot(default_up).abs() > 0.9 {
            // If axis is nearly parallel to default up, use a different reference
            Vector3::new(1.0, 0.0, 0.0)
        } else {
            default_up
        }
    } else {
        up_vector.normalize()
    };

    // Create a vector perpendicular to both the axis and the reference
    // This will be the starting point for our circle of attachment points
    let perpendicular = axis.cross(reference).normalize();

    // Create the second perpendicular vector to complete the basis
    let perpendicular2 = axis.cross(perpendicular).normalize();

    // Calculate positions for all attachment points
    let mut points = [AttachmentPoint {
        position: end_position,
        index: 0,
    }; ATTACHMENT_POINTS];

    // The attachment points should appear at the same radius as the push interval
    // Based on testing, we need to use a small multiplier to match the visual radius
    // The original calculation made the attachment points appear at about double the radius
    let bar_radius = radius * 0.04; // Reduced to match the push interval's visual radius

    for i in 0..ATTACHMENT_POINTS {
        // Calculate angle for this attachment point
        let angle = 2.0 * PI * (i as f32) / (ATTACHMENT_POINTS as f32);

        // Calculate offset from center to place points exactly at the edge of the bar
        let offset =
            perpendicular * angle.cos() * bar_radius + perpendicular2 * angle.sin() * bar_radius;

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

    // Use world up vector as reference for consistent orientation
    let up_vector = Vector3::new(0.0, 1.0, 0.0);

    // Calculate attachment points at both ends
    // Note: We use the same direction vector for both ends to ensure
    // the attachment points are not rotated relative to each other
    let start_points = calculate_attachment_points(start, direction, radius, up_vector);
    let end_points = calculate_attachment_points(end, direction, radius, up_vector);

    (start_points, end_points)
}
