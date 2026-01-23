/*
 * Stability Fitness - Rewards settled, stable structures
 *
 * A stable structure:
 * - Has low kinetic energy (has settled down)
 * - Has reasonable strain levels (not about to fail)
 * - Hasn't collapsed (still has structure)
 */

use crate::build::evo::traits::{FitnessDimension, TrialResult};

/// Fitness dimension that rewards structural stability.
pub struct StabilityFitness {
    /// Weight of this dimension.
    pub weight: f32,
    /// Strain threshold above which fitness is penalized.
    pub strain_threshold: f32,
    /// Kinetic energy threshold for "settled" classification.
    pub kinetic_threshold: f32,
}

impl Default for StabilityFitness {
    fn default() -> Self {
        Self {
            weight: 1.0,
            strain_threshold: 0.1,   // 10% strain
            kinetic_threshold: 0.01, // Very low kinetic energy
        }
    }
}

impl StabilityFitness {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }
}

impl FitnessDimension for StabilityFitness {
    fn name(&self) -> &str {
        "Stability"
    }

    fn evaluate(&self, trial: &TrialResult) -> f32 {
        // Failed structures get zero fitness
        if trial.structural_failure {
            return 0.0;
        }

        // Empty structures get zero fitness
        if trial.fabric.joints.len() < 2 || trial.fabric.intervals.len() < 1 {
            return 0.0;
        }

        // Kinetic energy score: reward low energy (settled structure)
        // Score approaches 1.0 as energy approaches 0
        let kinetic_score = 1.0 / (1.0 + trial.final_kinetic_energy / self.kinetic_threshold);

        // Strain score: reward low maximum strain
        // Score approaches 1.0 as strain approaches 0
        let strain_score = if trial.max_strain < self.strain_threshold {
            1.0
        } else {
            self.strain_threshold / trial.max_strain
        };

        // Complexity bonus: slightly reward having more structure
        // This prevents trivial solutions
        let complexity_bonus = (trial.fabric.intervals.len() as f32).sqrt() * 0.1;

        // Combine scores
        let base_score = kinetic_score * strain_score;
        (base_score + complexity_bonus).min(1.0)
    }

    fn weight(&self) -> f32 {
        self.weight
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::evo::traits::GenomeId;
    use crate::fabric::interval::Role;
    use crate::fabric::Fabric;
    use crate::units::Seconds;
    use glam::Vec3;

    fn make_trial_result(
        kinetic_energy: f32,
        max_strain: f32,
        interval_count: usize,
        failed: bool,
    ) -> TrialResult {
        let mut fabric = Fabric::new("test".to_string());
        // Add joints first
        let mut joints = vec![];
        for i in 0..interval_count + 1 {
            joints.push(fabric.create_joint(Vec3::new(i as f32, 0.0, 0.0)));
        }
        // Add intervals between consecutive joints
        for i in 0..interval_count {
            fabric.create_slack_interval(joints[i], joints[i + 1], Role::Pulling);
        }

        TrialResult {
            genome_id: GenomeId::new(),
            fabric,
            duration: Seconds(1.0),
            iteration_count: 1000,
            max_strain,
            structural_failure: failed,
            final_kinetic_energy: kinetic_energy,
            initial_centroid: Vec3::ZERO,
            final_centroid: Vec3::ZERO,
        }
    }

    #[test]
    fn test_failed_structure_gets_zero() {
        let fitness = StabilityFitness::default();
        let trial = make_trial_result(0.0, 0.0, 3, true);
        assert_eq!(fitness.evaluate(&trial), 0.0);
    }

    #[test]
    fn test_stable_structure_gets_high_score() {
        let fitness = StabilityFitness::default();
        let trial = make_trial_result(0.001, 0.05, 5, false);
        let score = fitness.evaluate(&trial);
        assert!(score > 0.5, "Expected high score, got {}", score);
    }

    #[test]
    fn test_unstable_structure_gets_low_score() {
        let fitness = StabilityFitness::default();
        let trial = make_trial_result(10.0, 0.3, 3, false);
        let score = fitness.evaluate(&trial);
        assert!(score < 0.5, "Expected low score, got {}", score);
    }
}
