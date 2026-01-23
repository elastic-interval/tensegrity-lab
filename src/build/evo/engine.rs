use crate::build::evo::traits::{
    CompositeFitness, Genome, PopulationStats, PopulationStrategy, TrialConfig,
};
use crate::build::evo::trial::{ActiveTrial, TrialStatus};
use crate::fabric::Fabric;

/// Current state of the evolution engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineState {
    /// Waiting to start a new trial.
    Idle,
    /// Running a trial.
    Running,
    /// Generation complete, ready for selection.
    GenerationComplete,
    /// Evolution terminated.
    Terminated,
}

pub struct EvolutionEngine<G: Genome> {
    population: Box<dyn PopulationStrategy<G>>,
    fitness: CompositeFitness,
    config: TrialConfig,
    state: EngineState,
    current_trial: Option<ActiveTrial<G>>,
    pending_results: Vec<(G, f32)>,
    total_trials: usize,
}

impl<G: Genome> EvolutionEngine<G> {
    pub fn new(
        population: Box<dyn PopulationStrategy<G>>,
        fitness: CompositeFitness,
        config: TrialConfig,
    ) -> Self {
        Self {
            population,
            fitness,
            config,
            state: EngineState::Idle,
            current_trial: None,
            pending_results: vec![],
            total_trials: 0,
        }
    }

    pub fn initialize(&mut self, seeds: Vec<G>) {
        self.population.initialize(seeds);
        self.state = EngineState::Idle;
    }

    /// Get current engine state.
    pub fn state(&self) -> EngineState {
        self.state
    }

    /// Get population statistics.
    pub fn stats(&self) -> PopulationStats {
        self.population.stats()
    }

    /// Get the current best genome.
    pub fn best(&self) -> Option<&G> {
        self.population.best()
    }

    /// Get total trials run.
    pub fn total_trials(&self) -> usize {
        self.total_trials
    }

    /// Get current trial progress (0.0 to 1.0) or None if no trial active.
    pub fn trial_progress(&self) -> Option<f32> {
        self.current_trial.as_ref().map(|t| t.progress())
    }

    /// Borrow the current fabric being evaluated (for rendering).
    pub fn current_fabric(&self) -> Option<&Fabric> {
        self.current_trial.as_ref().map(|t| t.fabric())
    }

    /// Run one iteration of visual mode evolution.
    /// Call this each frame when rendering.
    /// Returns true if the fabric changed (needs redraw).
    pub fn visual_iterate(&mut self, iterations_per_frame: usize) -> bool {
        if self.population.should_terminate() {
            self.state = EngineState::Terminated;
            return false;
        }

        match self.state {
            EngineState::Idle => {
                // Start a new trial
                if let Some(genome) = self.population.next_for_trial() {
                    self.current_trial = Some(ActiveTrial::new(genome, &self.config));
                    self.state = EngineState::Running;
                    true
                } else {
                    // No more genomes to evaluate this generation
                    self.state = EngineState::GenerationComplete;
                    false
                }
            }

            EngineState::Running => {
                if let Some(trial) = &mut self.current_trial {
                    // Run iterations for this frame
                    let status = trial.iterate_batch(iterations_per_frame);

                    match status {
                        TrialStatus::Running => true,
                        TrialStatus::Complete | TrialStatus::Failed => {
                            // Trial done - evaluate fitness
                            let trial = self.current_trial.take().unwrap();
                            let genome = trial.genome.clone();
                            let result = trial.complete();
                            let fitness = self.fitness.evaluate(&result);

                            // Record result
                            self.pending_results.push((genome, fitness));
                            self.total_trials += 1;

                            // Back to idle to start next trial
                            self.state = EngineState::Idle;
                            true
                        }
                    }
                } else {
                    self.state = EngineState::Idle;
                    false
                }
            }

            EngineState::GenerationComplete => {
                // Process results and advance generation
                let results = std::mem::take(&mut self.pending_results);
                for (genome, fitness) in results {
                    self.population.record_result(genome, fitness);
                }
                self.population.advance_generation();
                self.state = EngineState::Idle;
                false
            }

            EngineState::Terminated => false,
        }
    }

    /// Get a description of current activity.
    pub fn status_description(&self) -> String {
        let stats = self.stats();
        match self.state {
            EngineState::Idle => format!(
                "Gen {} - Starting trial (best: {:.4})",
                stats.generation, stats.best_fitness
            ),
            EngineState::Running => {
                let progress = self.trial_progress().unwrap_or(0.0) * 100.0;
                format!(
                    "Gen {} - Trial {:.0}% (best: {:.4})",
                    stats.generation, progress, stats.best_fitness
                )
            }
            EngineState::GenerationComplete => format!(
                "Gen {} complete (best: {:.4})",
                stats.generation, stats.best_fitness
            ),
            EngineState::Terminated => format!(
                "Evolution complete after {} generations (best: {:.4})",
                stats.generation, stats.best_fitness
            ),
        }
    }
}
