use crate::build::evo::traits::{
    ExpressionContext, Genome, IntervalController, TrialConfig, TrialResult,
};
use crate::fabric::interval::Span;
use crate::fabric::{Fabric, IntervalKey};
use crate::units::Meters;
use glam::Vec3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrialStatus {
    Running,
    Complete,
    Failed,
}

struct AttachedController {
    interval_key: IntervalKey,
    controller: Box<dyn IntervalController>,
}

pub struct ActiveTrial<G: Genome> {
    pub genome: G,
    pub fabric: Fabric,
    config: TrialConfig,
    controllers: Vec<AttachedController>,
    iteration: usize,
    initial_centroid: Vec3,
    max_strain: f32,
    structural_failure: bool,
}

impl<G: Genome> ActiveTrial<G> {
    pub fn new(genome: G, config: &TrialConfig) -> Self {
        let context = ExpressionContext::new(config.physics.clone());
        let fabric = genome.express(&context);
        let initial_centroid = fabric.centroid();

        // Collect interval keys for indexing
        let interval_keys: Vec<IntervalKey> = fabric.intervals.keys().collect();

        // Attach controllers from genome
        let controllers: Vec<AttachedController> = genome
            .controllers()
            .into_iter()
            .filter_map(|attachment| {
                interval_keys
                    .get(attachment.interval_index)
                    .map(|&key| AttachedController {
                        interval_key: key,
                        controller: attachment.controller,
                    })
            })
            .collect();

        Self {
            genome,
            fabric,
            config: config.clone(),
            controllers,
            iteration: 0,
            initial_centroid,
            max_strain: 0.0,
            structural_failure: false,
        }
    }

    pub fn iterate(&mut self) -> TrialStatus {
        // Sensorimotor loop: sense and react
        for attached in &mut self.controllers {
            if let Some(reading) = self.fabric.interval_reading(attached.interval_key) {
                if let Some(new_target) = attached.controller.react(&reading) {
                    if let Some(interval) = self.fabric.intervals.get_mut(attached.interval_key) {
                        interval.span = Span::Fixed {
                            length: Meters(new_target),
                        };
                    }
                }
            }
        }

        // Physics
        self.fabric.iterate(&self.config.physics);
        self.iteration += 1;

        // Track max strain
        if self.fabric.stats.max_strain > self.max_strain {
            self.max_strain = self.fabric.stats.max_strain;
        }

        // Check for structural failure
        if self.max_strain > 0.5 {
            self.structural_failure = true;
            return TrialStatus::Failed;
        }

        if self.iteration >= self.config.max_iterations {
            TrialStatus::Complete
        } else {
            TrialStatus::Running
        }
    }

    pub fn iterate_batch(&mut self, count: usize) -> TrialStatus {
        for _ in 0..count {
            match self.iterate() {
                TrialStatus::Running => continue,
                status => return status,
            }
        }
        TrialStatus::Running
    }

    pub fn iteration(&self) -> usize {
        self.iteration
    }

    pub fn progress(&self) -> f32 {
        self.iteration as f32 / self.config.max_iterations as f32
    }

    pub fn into_result(self) -> TrialResult {
        TrialResult {
            genome_id: self.genome.id(),
            fabric: self.fabric,
            duration: self.config.trial_duration,
            iteration_count: self.iteration,
            max_strain: self.max_strain,
            structural_failure: self.structural_failure,
            final_kinetic_energy: 0.0, // computed below
            initial_centroid: self.initial_centroid,
            final_centroid: Vec3::ZERO, // computed below
        }
    }

    pub fn fabric(&self) -> &Fabric {
        &self.fabric
    }
}

// Fix: compute final values before consuming self
impl<G: Genome> ActiveTrial<G> {
    pub fn complete(self) -> TrialResult {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::evo::traits::GenomeId;
    use crate::fabric::interval::Role;
    use crate::fabric::physics::presets::CONSTRUCTION;
    use crate::units::Seconds;

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
            vec![]
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
        let config = TrialConfig::new(CONSTRUCTION, Seconds(0.01));

        let mut trial = ActiveTrial::new(genome, &config);

        loop {
            match trial.iterate() {
                TrialStatus::Running => continue,
                TrialStatus::Complete => break,
                TrialStatus::Failed => panic!("Trial should not fail"),
            }
        }

        let result = trial.complete();
        assert!(result.iteration_count > 0);
        assert!(!result.structural_failure);
    }
}
