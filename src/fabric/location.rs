/// Location with position history for advanced physics damping
///
/// Maintains a ring buffer of recent positions to enable:
/// - Smoothed velocity calculation
/// - Oscillation detection
/// - Frequency-aware damping
///
/// Computational strategy:
/// - Cheap operations (O(N)): Always performed, but N is small (5-10)
/// - Expensive operations: Only when oscillation detected

use cgmath::{EuclideanSpace, InnerSpace, Matrix4, MetricSpace, Point3, Transform, Vector3};
use std::ops::{Add, Sub};

/// History size - const generic allows zero-cost abstraction
/// N=5-6 is enough to see 2-3 oscillation cycles at typical frequencies
pub const HISTORY_SIZE: usize = 6;


#[derive(Clone, Debug, PartialEq)]
pub struct Location {
    history: [Point3<f32>; HISTORY_SIZE],
    write_index: usize,
    count: usize, // Number of valid entries (0 to HISTORY_SIZE)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OscillationLevel {
    None,      // Smooth motion
    Mild,      // Some oscillation
    Strong,    // Significant oscillation, needs damping
}

impl Location {
    /// Create new location with initial position
    pub fn new(initial: Point3<f32>) -> Self {
        Self {
            history: [initial; HISTORY_SIZE],
            write_index: 0,
            count: 1,
        }
    }

    /// Update with new position - O(1)
    /// Stores position in ring buffer for history tracking
    pub fn update(&mut self, pos: Point3<f32>) {
        self.history[self.write_index] = pos;
        self.write_index = (self.write_index + 1) % HISTORY_SIZE;
        self.count = (self.count + 1).min(HISTORY_SIZE);
    }

    /// Get current position - O(1)
    /// Returns the most recent position (not smoothed)
    pub fn current(&self) -> Point3<f32> {
        self.get_relative(0)
    }

    /// Get position N frames ago - O(1)
    /// offset = 0 is current, 1 is previous, etc.
    fn get_relative(&self, offset: usize) -> Point3<f32> {
        if offset >= self.count {
            return self.history[0]; // Return oldest available
        }
        let idx = if self.write_index > offset {
            self.write_index - offset - 1
        } else {
            HISTORY_SIZE + self.write_index - offset - 1
        };
        self.history[idx]
    }

    /// Calculate smoothed velocity using history - O(N) but N is small
    /// Uses central difference over available history for stability
    pub fn velocity_smooth(&self, dt: f32) -> Vector3<f32> {
        if self.count < 2 {
            return Vector3::new(0.0, 0.0, 0.0);
        }

        // Use up to 4 frames back for velocity (balances smoothness vs latency)
        let lookback = self.count.min(4);
        let current = self.current();
        let past = self.get_relative(lookback - 1);

        (current - past) / (lookback as f32 * dt)
    }

    /// Simple velocity (current - previous) for compatibility - O(1)
    pub fn velocity_simple(&self, dt: f32) -> Vector3<f32> {
        if self.count < 2 {
            return Vector3::new(0.0, 0.0, 0.0);
        }
        (self.current() - self.get_relative(1)) / dt
    }

    /// Cheap oscillation detection - O(N) but N is small
    /// Counts direction changes in velocity to detect oscillation
    pub fn oscillation_level(&self) -> OscillationLevel {
        if self.count < 4 {
            return OscillationLevel::None;
        }

        // Count velocity direction reversals
        let mut direction_changes = 0;
        let check_frames = self.count.min(HISTORY_SIZE);

        for i in 1..check_frames - 1 {
            let v_prev = self.get_relative(i + 1) - self.get_relative(i + 2);
            let v_curr = self.get_relative(i) - self.get_relative(i + 1);

            // If velocity vectors point in opposite directions, we have oscillation
            if v_prev.dot(v_curr) < 0.0 {
                direction_changes += 1;
            }
        }

        // Classify based on number of direction changes
        match direction_changes {
            0..=1 => OscillationLevel::None,
            2 => OscillationLevel::Mild,
            _ => OscillationLevel::Strong,
        }
    }

    /// Calculate oscillation strength - O(N)
    /// Returns 0.0-1.0 indicating how much oscillatory motion is present
    /// Only call this when oscillation detected (expensive path)
    pub fn oscillation_strength(&self) -> f32 {
        if self.count < 4 {
            return 0.0;
        }

        // Calculate variance in velocity magnitudes
        let mut velocities = Vec::with_capacity(self.count - 1);
        for i in 0..self.count - 1 {
            let v = self.get_relative(i) - self.get_relative(i + 1);
            velocities.push(v.magnitude());
        }

        if velocities.is_empty() {
            return 0.0;
        }

        // High variance in speed indicates oscillation
        let mean: f32 = velocities.iter().sum::<f32>() / velocities.len() as f32;
        let variance: f32 = velocities.iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f32>() / velocities.len() as f32;

        let std_dev = variance.sqrt();

        // Normalize to 0-1 range (coefficient of variation)
        if mean > 0.0001 {
            (std_dev / mean).min(1.0)
        } else {
            0.0
        }
    }

    /// Get damping factor based on motion characteristics - O(N)
    /// Returns 0.0 (no extra damping) to 1.0 (maximum damping)
    /// This is the main API for physics - cheap check, expensive only when needed
    pub fn adaptive_damping_factor(&self) -> f32 {
        match self.oscillation_level() {
            OscillationLevel::None => 0.0,
            OscillationLevel::Mild => 0.2,
            OscillationLevel::Strong => {
                // Only compute detailed strength for strong oscillations
                0.5 + 0.5 * self.oscillation_strength()
            }
        }
    }

    /// Check if we have enough history for advanced features
    pub fn has_full_history(&self) -> bool {
        self.count >= HISTORY_SIZE
    }

    // ===== Point3-like API for transparent usage =====

    /// Get x coordinate of current position
    pub fn x(&self) -> f32 {
        self.current().x
    }

    /// Get y coordinate of current position
    pub fn y(&self) -> f32 {
        self.current().y
    }

    /// Get z coordinate of current position
    pub fn z(&self) -> f32 {
        self.current().z
    }

    /// Convert current position to Vector3
    pub fn to_vec(&self) -> Vector3<f32> {
        self.current().to_vec()
    }

    /// Calculate distance to another point
    pub fn distance(&self, other: Point3<f32>) -> f32 {
        self.current().distance(other)
    }

    /// Calculate squared distance to another point (faster, no sqrt)
    pub fn distance2(&self, other: Point3<f32>) -> f32 {
        self.current().distance2(other)
    }

    /// Apply translation to all positions in history
    /// This is critical when the fabric is moved/centralized - we must translate
    /// the entire history, not just the current position
    pub fn translate(&mut self, translation: Vector3<f32>) {
        for pos in &mut self.history {
            *pos += translation;
        }
    }

    /// Apply matrix transformation to all positions in history
    /// Used during rotations and other transformations
    pub fn transform(&mut self, matrix: Matrix4<f32>) {
        for pos in &mut self.history {
            *pos = matrix.transform_point(*pos);
        }
    }
}

// ===== Operator overloading for Point3-like behavior =====

/// Add Vector3 to Location (returns new Point3)
impl Add<Vector3<f32>> for &Location {
    type Output = Point3<f32>;

    fn add(self, rhs: Vector3<f32>) -> Point3<f32> {
        self.current() + rhs
    }
}

/// Subtract Vector3 from Location (returns new Point3)
impl Sub<Vector3<f32>> for &Location {
    type Output = Point3<f32>;

    fn sub(self, rhs: Vector3<f32>) -> Point3<f32> {
        self.current() - rhs
    }
}

/// Subtract Location from Location (returns Vector3)
impl Sub<&Location> for &Location {
    type Output = Vector3<f32>;

    fn sub(self, rhs: &Location) -> Vector3<f32> {
        self.current() - rhs.current()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_initialization() {
        let pos = Point3::new(1.0, 2.0, 3.0);
        let loc = Location::new(pos);
        assert_eq!(loc.current(), pos);
        assert_eq!(loc.count, 1);
    }

    #[test]
    fn test_location_update() {
        let mut loc = Location::new(Point3::new(0.0, 0.0, 0.0));
        loc.update(Point3::new(1.0, 0.0, 0.0));
        assert_eq!(loc.current(), Point3::new(1.0, 0.0, 0.0));
        assert_eq!(loc.count, 2);
    }

    #[test]
    fn test_velocity_simple() {
        let mut loc = Location::new(Point3::new(0.0, 0.0, 0.0));
        loc.update(Point3::new(1.0, 0.0, 0.0));

        let v = loc.velocity_simple(1.0);
        assert!((v.x - 1.0).abs() < 0.001);
        assert!(v.y.abs() < 0.001);
    }

    #[test]
    fn test_oscillation_detection() {
        let mut loc = Location::new(Point3::new(0.0, 0.0, 0.0));

        // Create oscillating motion: 0 -> 1 -> 0 -> 1 -> 0
        loc.update(Point3::new(1.0, 0.0, 0.0));
        loc.update(Point3::new(0.0, 0.0, 0.0));
        loc.update(Point3::new(1.0, 0.0, 0.0));
        loc.update(Point3::new(0.0, 0.0, 0.0));

        let level = loc.oscillation_level();
        assert!(matches!(level, OscillationLevel::Mild | OscillationLevel::Strong));
    }

    #[test]
    fn test_smooth_motion_no_oscillation() {
        let mut loc = Location::new(Point3::new(0.0, 0.0, 0.0));

        // Smooth motion in one direction
        for i in 1..=5 {
            loc.update(Point3::new(i as f32, 0.0, 0.0));
        }

        let level = loc.oscillation_level();
        assert_eq!(level, OscillationLevel::None);
    }
}
