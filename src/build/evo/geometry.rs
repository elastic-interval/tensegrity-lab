//! Geometry utilities for evolution, particularly for detecting push crossings.

use crate::fabric::interval::Role;
use crate::fabric::Fabric;
use glam::Vec3;

/// Calculate the minimum distance between two 3D line segments.
///
/// Uses the algorithm from "Real-Time Collision Detection" by Christer Ericson.
/// Returns the minimum distance between any point on segment (p1, q1) and any point on segment (p2, q2).
pub fn segment_min_distance(p1: Vec3, q1: Vec3, p2: Vec3, q2: Vec3) -> f32 {
    let d1 = q1 - p1; // Direction vector of segment 1
    let d2 = q2 - p2; // Direction vector of segment 2
    let r = p1 - p2;

    let a = d1.dot(d1); // Squared length of segment 1
    let e = d2.dot(d2); // Squared length of segment 2
    let f = d2.dot(r);

    const EPSILON: f32 = 1e-10;

    // Check if either or both segments degenerate into points
    if a <= EPSILON && e <= EPSILON {
        // Both segments are points
        return (p1 - p2).length();
    }

    let (mut s, mut t);

    if a <= EPSILON {
        // First segment degenerates into a point
        s = 0.0;
        t = (f / e).clamp(0.0, 1.0);
    } else {
        let c = d1.dot(r);
        if e <= EPSILON {
            // Second segment degenerates into a point
            t = 0.0;
            s = (-c / a).clamp(0.0, 1.0);
        } else {
            // General case: both segments have non-zero length
            let b = d1.dot(d2);
            let denom = a * e - b * b; // Always non-negative

            // If segments not parallel, compute closest point on L1 to L2
            if denom.abs() > EPSILON {
                s = ((b * f - c * e) / denom).clamp(0.0, 1.0);
            } else {
                // Segments are parallel, pick arbitrary s
                s = 0.0;
            }

            // Compute point on L2 closest to S1(s)
            t = (b * s + f) / e;

            // If t outside [0,1], clamp and recompute s
            if t < 0.0 {
                t = 0.0;
                s = (-c / a).clamp(0.0, 1.0);
            } else if t > 1.0 {
                t = 1.0;
                s = ((b - c) / a).clamp(0.0, 1.0);
            }
        }
    }

    let c1 = p1 + d1 * s; // Closest point on segment 1
    let c2 = p2 + d2 * t; // Closest point on segment 2

    (c1 - c2).length()
}

/// Result of analyzing push crossings in a fabric.
#[derive(Debug, Clone, Default)]
pub struct CrossingAnalysis {
    /// Number of push pairs that cross (distance < threshold)
    pub crossing_count: usize,
    /// Number of push pairs that are dangerously close
    pub near_miss_count: usize,
    /// Minimum distance found between any non-adjacent push pair (in meters)
    pub min_distance: f32,
    /// All distances for statistical analysis
    pub distances: Vec<f32>,
}

impl CrossingAnalysis {
    /// Score from 0.0 (many crossings) to 1.0 (no crossings).
    /// Uses exponential decay: each crossing halves the score.
    pub fn score(&self) -> f32 {
        0.5_f32.powi(self.crossing_count as i32)
    }
}

/// Analyze push crossings in a fabric.
///
/// Parameters:
/// - `fabric`: The fabric to analyze
/// - `crossing_threshold`: Distance below which pushes are considered crossing (meters)
/// - `near_miss_threshold`: Distance below which pushes are considered dangerously close (meters)
pub fn analyze_push_crossings(
    fabric: &Fabric,
    crossing_threshold: f32,
    near_miss_threshold: f32,
) -> CrossingAnalysis {
    let push_intervals: Vec<_> = fabric
        .intervals
        .iter()
        .filter(|(_, interval)| interval.has_role(Role::Pushing))
        .collect();

    let mut analysis = CrossingAnalysis {
        min_distance: f32::MAX,
        ..Default::default()
    };

    for i in 0..push_intervals.len() {
        for j in (i + 1)..push_intervals.len() {
            let (_, int1) = push_intervals[i];
            let (_, int2) = push_intervals[j];

            // Skip if they share a joint (adjacent pushes)
            if int1.alpha_key == int2.alpha_key
                || int1.alpha_key == int2.omega_key
                || int1.omega_key == int2.alpha_key
                || int1.omega_key == int2.omega_key
            {
                continue;
            }

            let p1 = fabric.location(int1.alpha_key);
            let q1 = fabric.location(int1.omega_key);
            let p2 = fabric.location(int2.alpha_key);
            let q2 = fabric.location(int2.omega_key);

            let distance = segment_min_distance(p1, q1, p2, q2);
            analysis.distances.push(distance);

            if distance < analysis.min_distance {
                analysis.min_distance = distance;
            }

            if distance < crossing_threshold {
                analysis.crossing_count += 1;
            } else if distance < near_miss_threshold {
                analysis.near_miss_count += 1;
            }
        }
    }

    if analysis.distances.is_empty() {
        analysis.min_distance = 0.0;
    }

    analysis
}

/// Count push crossings using default thresholds.
/// Uses 10mm for crossing, 50mm for near-miss.
pub fn count_push_crossings(fabric: &Fabric) -> usize {
    let analysis = analyze_push_crossings(fabric, 0.010, 0.050);
    analysis.crossing_count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_segments() {
        // Two parallel segments, 1 unit apart
        let p1 = Vec3::new(0.0, 0.0, 0.0);
        let q1 = Vec3::new(1.0, 0.0, 0.0);
        let p2 = Vec3::new(0.0, 1.0, 0.0);
        let q2 = Vec3::new(1.0, 1.0, 0.0);

        let dist = segment_min_distance(p1, q1, p2, q2);
        assert!((dist - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_crossing_segments() {
        // Two segments that cross at the origin
        let p1 = Vec3::new(-1.0, 0.0, 0.0);
        let q1 = Vec3::new(1.0, 0.0, 0.0);
        let p2 = Vec3::new(0.0, -1.0, 0.0);
        let q2 = Vec3::new(0.0, 1.0, 0.0);

        let dist = segment_min_distance(p1, q1, p2, q2);
        assert!(dist < 1e-6);
    }

    #[test]
    fn test_skew_segments() {
        // Two skew segments in 3D
        let p1 = Vec3::new(0.0, 0.0, 0.0);
        let q1 = Vec3::new(1.0, 0.0, 0.0);
        let p2 = Vec3::new(0.5, 1.0, 0.0);
        let q2 = Vec3::new(0.5, 1.0, 1.0);

        let dist = segment_min_distance(p1, q1, p2, q2);
        assert!((dist - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_point_to_segment() {
        // One degenerate segment (point)
        let p1 = Vec3::new(0.5, 0.0, 0.0);
        let q1 = Vec3::new(0.5, 0.0, 0.0); // Same point
        let p2 = Vec3::new(0.0, 1.0, 0.0);
        let q2 = Vec3::new(1.0, 1.0, 0.0);

        let dist = segment_min_distance(p1, q1, p2, q2);
        assert!((dist - 1.0).abs() < 1e-6);
    }
}
