/*
 * Trial Execution - Running a single evolutionary trial
 *
 * An ActiveTrial represents a genome being evaluated:
 * 1. Express genome into fabric
 * 2. Run physics simulation for trial duration
 * 3. Collect results for fitness evaluation
 */

use crate::build::evo::traits::{ExpressionContext, Genome, TrialConfig, TrialResult};
use crate::fabric::Fabric;
use glam::Vec3;

/// Status of a running trial.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrialStatus {
    /// Trial is still running.
    Running,
    /// Trial completed normally.
    Complete,
    /// Trial failed (structural failure, etc.).
    Failed,
}

/// A single trial being executed.
pub struct ActiveTrial<G: Genome> {
    /// The genome being evaluated.
    pub genome: G,

    /// The expressed fabric.
    pub fabric: Fabric,

    /// Trial configuration (physics, duration).
    config: TrialConfig,

    /// Current iteration count.
    iteration: usize,

    /// Initial centroid position.
    initial_centroid: Vec3,

    /// Maximum strain seen during trial.
    max_strain: f32,

    /// Whether structural failure occurred.
    structural_failure: bool,
}

impl<G: Genome> ActiveTrial<G> {
    /// Create a new trial from a genome.
    pub fn new(genome: G, config: &TrialConfig) -> Self {
        // Express genome into fabric
        let context = ExpressionContext::new(config.physics.clone());
        let fabric = genome.express(&context);

        let initial_centroid = fabric.centroid();

        Self {
            genome,
            fabric,
            config: config.clone(),
            iteration: 0,
            initial_centroid,
            max_strain: 0.0,
            structural_failure: false,
        }
    }

    /// Run one physics iteration.
    pub fn iterate(&mut self) -> TrialStatus {
        // Run physics
        self.fabric.iterate(&self.config.physics);
        self.iteration += 1;

        // Track max strain
        if self.fabric.stats.max_strain > self.max_strain {
            self.max_strain = self.fabric.stats.max_strain;
        }

        // Check for structural failure (excessive strain)
        if self.max_strain > 0.5 {
            self.structural_failure = true;
            return TrialStatus::Failed;
        }

        // Check termination
        if self.iteration >= self.config.max_iterations {
            TrialStatus::Complete
        } else {
            TrialStatus::Running
        }
    }

    /// Run multiple physics iterations (for faster execution).
    pub fn iterate_batch(&mut self, count: usize) -> TrialStatus {
        for _ in 0..count {
            match self.iterate() {
                TrialStatus::Running => continue,
                status => return status,
            }
        }
        TrialStatus::Running
    }

    /// Get current iteration count.
    pub fn iteration(&self) -> usize {
        self.iteration
    }

    /// Get progress as fraction [0, 1].
    pub fn progress(&self) -> f32 {
        self.iteration as f32 / self.config.max_iterations as f32
    }

    /// Extract final trial result.
    pub fn into_result(self) -> TrialResult {
        let final_centroid = self.fabric.centroid();
        let final_kinetic_energy = self.fabric.kinetic_energy();

        TrialResult {
            genome_id: self.genome.id(),
            fabric: self.fabric,
            duration: self.config.trial_duration,
            iteration_count: self.iteration,
            max_strain: self.max_strain,
            structural_failure: self.structural_failure,
            final_kinetic_energy,
            initial_centroid: self.initial_centroid,
            final_centroid,
        }
    }

    /// Borrow the current fabric (for rendering).
    pub fn fabric(&self) -> &Fabric {
        &self.fabric
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::evo::traits::GenomeId;
    use crate::fabric::interval::Role;
    use crate::fabric::physics::presets::CONSTRUCTION;
    use crate::fabric::Fabric;
    use crate::units::Seconds;

    /// Minimal test genome that creates a single push interval.
    #[derive(Clone, Debug)]
    struct TestGenome {
        id: GenomeId,
    }

    impl Genome for TestGenome {
        fn id(&self) -> GenomeId {
            self.id
        }

        fn express(&self, _context: &ExpressionContext) -> Fabric {
            let mut fabric = Fabric::new("test".to_string());
            let a = fabric.create_joint(Vec3::new(-0.5, 0.0, 0.0));
            let b = fabric.create_joint(Vec3::new(0.5, 0.0, 0.0));
            fabric.create_slack_interval(a, b, Role::Pushing);
            fabric
        }

        fn adjacent_possible(&self, _rng: &mut impl rand::Rng) -> Vec<Self> {
            vec![] // No mutations for test
        }

        fn describe(&self) -> String {
            "TestGenome".to_string()
        }
    }

    #[test]
    fn test_trial_runs_to_completion() {
        let genome = TestGenome {
            id: GenomeId::new(),
        };
        let config = TrialConfig::new(CONSTRUCTION, Seconds(0.01)); // 200 iterations

        let mut trial = ActiveTrial::new(genome, &config);

        // Run to completion
        loop {
            match trial.iterate() {
                TrialStatus::Running => continue,
                TrialStatus::Complete => break,
                TrialStatus::Failed => panic!("Trial should not fail"),
            }
        }

        let result = trial.into_result();
        assert!(result.iteration_count > 0);
        assert!(!result.structural_failure);
    }
}
