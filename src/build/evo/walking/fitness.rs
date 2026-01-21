//! Walking fitness evaluation.
//!
//! Evaluates tensegrity structures based on their locomotion efficiency:
//! - Distance traveled per unit time (velocity)
//! - Structural efficiency (push count penalty)
//! - Energy efficiency (actuator work per meter)
//! - Stability (maintaining height during locomotion)

use crate::fabric::Fabric;
use crate::units::Seconds;
use glam::Vec3;

/// Configuration for walking fitness evaluation.
#[derive(Clone, Debug)]
pub struct WalkingFitnessConfig {
    /// Duration to run animation before measuring (seconds)
    pub evaluation_duration: Seconds,
    /// Minimum distance to achieve non-zero fitness (meters)
    pub min_travel_threshold: f32,
    /// Push count above which penalty applies
    pub push_penalty_threshold: usize,
    /// Penalty factor per push above threshold (e.g., 0.90 = 10% penalty per push)
    pub push_penalty_factor: f32,
    /// Energy penalty weight (higher = stronger penalty for energy use)
    pub energy_penalty_weight: f32,
    /// Minimum height ratio to maintain (fraction of starting height)
    pub min_stability_ratio: f32,
}

impl Default for WalkingFitnessConfig {
    fn default() -> Self {
        Self {
            evaluation_duration: Seconds(5.0),
            min_travel_threshold: 0.01, // 1cm minimum
            push_penalty_threshold: 6,
            push_penalty_factor: 0.90,
            energy_penalty_weight: 0.1,
            min_stability_ratio: 0.5,
        }
    }
}

/// Detailed results from fitness evaluation.
#[derive(Clone, Debug, Default)]
pub struct WalkingFitnessDetails {
    /// Final computed fitness value
    pub fitness: f32,
    /// Horizontal distance traveled (meters)
    pub distance: f32,
    /// Elapsed fabric time (seconds)
    pub elapsed_time: f32,
    /// Effective velocity (m/s)
    pub velocity: f32,
    /// Number of push intervals
    pub push_count: usize,
    /// Total energy input from actuators
    pub actuator_work: f32,
    /// Energy per meter traveled (efficiency metric)
    pub energy_per_meter: f32,
    /// Height maintenance ratio (final/initial)
    pub stability: f32,
    /// Whether the structure collapsed
    pub collapsed: bool,
}

impl WalkingFitnessDetails {
    /// Create details for an invalid/failed evaluation
    pub fn invalid(push_count: usize) -> Self {
        Self {
            fitness: 0.0,
            push_count,
            collapsed: true,
            ..Default::default()
        }
    }
}

/// Walking fitness evaluator.
pub struct WalkingFitness {
    config: WalkingFitnessConfig,
}

impl WalkingFitness {
    pub fn new(config: WalkingFitnessConfig) -> Self {
        Self { config }
    }

    /// Calculate fitness from raw metrics.
    ///
    /// This is called after animation has run, with the measured values.
    pub fn calculate_fitness(
        &self,
        start_centroid: Vec3,
        end_centroid: Vec3,
        start_height: f32,
        end_height: f32,
        elapsed_seconds: f32,
        push_count: usize,
        actuator_work: f32,
    ) -> WalkingFitnessDetails {
        // Calculate horizontal displacement (ignore Y)
        let horizontal_displacement = Vec3::new(
            end_centroid.x - start_centroid.x,
            0.0,
            end_centroid.z - start_centroid.z,
        );
        let distance = horizontal_displacement.length();

        // Check for collapse
        let stability = if start_height > 0.01 {
            end_height / start_height
        } else {
            0.0
        };
        let collapsed = stability < self.config.min_stability_ratio;

        if collapsed {
            return WalkingFitnessDetails {
                fitness: 0.0,
                distance,
                elapsed_time: elapsed_seconds,
                velocity: 0.0,
                push_count,
                actuator_work,
                energy_per_meter: f32::INFINITY,
                stability,
                collapsed: true,
            };
        }

        // Calculate velocity
        let velocity = if elapsed_seconds > 0.0 {
            distance / elapsed_seconds
        } else {
            0.0
        };

        // Check minimum travel threshold
        if distance < self.config.min_travel_threshold {
            return WalkingFitnessDetails {
                fitness: 0.0,
                distance,
                elapsed_time: elapsed_seconds,
                velocity,
                push_count,
                actuator_work,
                energy_per_meter: f32::INFINITY,
                stability,
                collapsed: false,
            };
        }

        // Calculate penalties
        let push_penalty = self.calculate_push_penalty(push_count);
        let energy_per_meter = actuator_work / distance;
        let energy_penalty = 1.0 / (1.0 + energy_per_meter * self.config.energy_penalty_weight);

        // Stability bonus (reward structures that maintain height well)
        let stability_bonus = stability.min(1.0);

        // Final fitness
        let fitness = velocity * push_penalty * energy_penalty * stability_bonus;

        WalkingFitnessDetails {
            fitness,
            distance,
            elapsed_time: elapsed_seconds,
            velocity,
            push_count,
            actuator_work,
            energy_per_meter,
            stability,
            collapsed: false,
        }
    }

    /// Calculate push count penalty.
    fn calculate_push_penalty(&self, push_count: usize) -> f32 {
        if push_count <= self.config.push_penalty_threshold {
            1.0
        } else {
            let excess = push_count - self.config.push_penalty_threshold;
            self.config.push_penalty_factor.powi(excess as i32)
        }
    }

    /// Get the evaluation duration in seconds.
    pub fn evaluation_duration(&self) -> f32 {
        self.config.evaluation_duration.0
    }

    /// Measure initial state before animation.
    pub fn measure_initial_state(fabric: &Fabric) -> InitialState {
        let centroid = fabric.centroid();
        let (min_y, max_y) = fabric.altitude_range();
        let height = max_y - min_y;
        let age = fabric.age;

        InitialState {
            centroid,
            height,
            min_altitude: min_y,
            age,
        }
    }
}

/// State captured before animation begins.
#[derive(Clone, Debug)]
pub struct InitialState {
    pub centroid: Vec3,
    pub height: f32,
    pub min_altitude: f32,
    pub age: crate::Age,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fitness_calculation_basic() {
        let fitness = WalkingFitness::new(WalkingFitnessConfig::default());

        let details = fitness.calculate_fitness(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0), // 1 meter forward
            1.0,                       // start height
            0.9,                       // end height (slightly lower)
            5.0,                       // 5 seconds
            6,                         // 6 pushes (at threshold)
            1.0,                       // 1 joule of work
        );

        assert!(details.fitness > 0.0);
        assert!((details.distance - 1.0).abs() < 0.001);
        assert!((details.velocity - 0.2).abs() < 0.001); // 1m / 5s = 0.2 m/s
        assert!(!details.collapsed);
    }

    #[test]
    fn test_fitness_collapsed_structure() {
        let fitness = WalkingFitness::new(WalkingFitnessConfig::default());

        let details = fitness.calculate_fitness(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            1.0,  // start height
            0.1,  // end height (collapsed to 10%)
            5.0,
            6,
            1.0,
        );

        assert_eq!(details.fitness, 0.0);
        assert!(details.collapsed);
    }

    #[test]
    fn test_push_penalty() {
        let fitness = WalkingFitness::new(WalkingFitnessConfig::default());

        // At threshold: no penalty
        let details1 = fitness.calculate_fitness(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            1.0, 1.0, 5.0,
            6, // at threshold
            0.0,
        );

        // Above threshold: penalty applied
        let details2 = fitness.calculate_fitness(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            1.0, 1.0, 5.0,
            10, // 4 above threshold
            0.0,
        );

        assert!(details2.fitness < details1.fitness);
    }
}
