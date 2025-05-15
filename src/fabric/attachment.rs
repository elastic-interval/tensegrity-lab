/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use cgmath::{InnerSpace, Point3, Vector3};
use std::f32::consts::PI;
use crate::fabric::UniqueId;

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
        let offset = perpendicular * angle.cos() * bar_radius + perpendicular2 * angle.sin() * bar_radius;
        
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
) -> ([AttachmentPoint; ATTACHMENT_POINTS], [AttachmentPoint; ATTACHMENT_POINTS]) {
    // Calculate direction vector from start to end
    let direction = Vector3::new(
        end.x - start.x,
        end.y - start.y,
        end.z - start.z,
    );
    
    // Use world up vector as reference for consistent orientation
    let up_vector = Vector3::new(0.0, 1.0, 0.0);
    
    // Calculate attachment points at both ends
    // Note: We use the same direction vector for both ends to ensure
    // the attachment points are not rotated relative to each other
    let start_points = calculate_attachment_points(start, direction, radius, up_vector);
    let end_points = calculate_attachment_points(end, direction, radius, up_vector);
    
    (start_points, end_points)
}
