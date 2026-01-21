//! Walking evolution controller.
//!
//! Evolves brick-based tensegrity structures optimized for walking locomotion.
//! Uses FabricPlanExecutor to build structures from genomes, then evaluates
//! fitness based on distance traveled on a sticky surface.

use crate::build::animator::Animator;
use crate::build::dsl::fabric_plan_executor::{ExecutorStage, FabricPlanExecutor, IterateResult};
use crate::crucible_context::CrucibleContext;
use crate::fabric::physics::presets::ANIMATING;
use crate::fabric::Fabric;
use crate::{Age, DisplayState, LabEvent, StateChange};
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;

use super::fitness::{InitialState, WalkingFitness, WalkingFitnessConfig, WalkingFitnessDetails};
use super::genome::{GenomeConstraints, WalkingGenome};
use super::mutations::{MutationWeights, WalkingMutator};

/// Configuration for walking evolution.
#[derive(Clone, Debug)]
pub struct WalkingEvolutionConfig {
    /// Population size
    pub population_size: usize,
    /// Duration to animate before measuring fitness (seconds)
    pub evaluation_seconds: f32,
    /// Genome constraints
    pub constraints: GenomeConstraints,
    /// Mutation weights
    pub mutation_weights: MutationWeights,
    /// Fitness evaluation config
    pub fitness_config: WalkingFitnessConfig,
}

impl Default for WalkingEvolutionConfig {
    fn default() -> Self {
        Self {
            population_size: 20,
            evaluation_seconds: 5.0,
            constraints: GenomeConstraints::default(),
            mutation_weights: MutationWeights::default(),
            fitness_config: WalkingFitnessConfig::default(),
        }
    }
}

/// Individual in the walking population.
#[derive(Clone)]
pub struct WalkingIndividual {
    pub genome: WalkingGenome,
    pub fitness: f32,
    pub details: WalkingFitnessDetails,
    pub generation: usize,
}

impl WalkingIndividual {
    fn new(genome: WalkingGenome) -> Self {
        Self {
            genome,
            fitness: 0.0,
            details: WalkingFitnessDetails::default(),
            generation: 0,
        }
    }
}

/// State of the walking evolution process.
#[derive(Debug, Clone, PartialEq)]
pub enum WalkingState {
    /// Building a structure from genome
    Building,
    /// Running animation for fitness evaluation
    Animating,
    /// Evaluating fitness and selecting next
    Evaluating,
}

/// Viewing mode for evolution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewingMode {
    /// Watch physics happening in real-time
    Watch,
    /// Run fast, only show best structures
    Fast,
}

/// Main controller for walking tensegrity evolution.
pub struct WalkingEvolution {
    rng: ChaCha8Rng,
    config: WalkingEvolutionConfig,
    state: WalkingState,
    viewing_mode: ViewingMode,

    // Population
    population: Vec<WalkingIndividual>,
    generation: usize,
    evaluations: usize,

    // Current evaluation
    current_genome: WalkingGenome,
    current_executor: Option<FabricPlanExecutor>,
    current_animator: Option<Animator>,
    current_initial_state: Option<InitialState>,
    current_animation_start_age: Age,

    // Evaluation tools
    mutator: WalkingMutator,
    fitness_evaluator: WalkingFitness,

    // Display fabric
    pub fabric: Fabric,
}

impl WalkingEvolution {
    /// Create a new walking evolution controller.
    pub fn new() -> Self {
        Self::with_config(WalkingEvolutionConfig::default())
    }

    /// Create with specific configuration.
    pub fn with_config(config: WalkingEvolutionConfig) -> Self {
        let master_seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(42);

        Self::with_seed(master_seed, config)
    }

    /// Create with a specific seed (for deterministic testing).
    pub fn with_seed(seed: u64, config: WalkingEvolutionConfig) -> Self {
        let mutator = WalkingMutator::new(config.mutation_weights.clone(), config.constraints.clone());
        let fitness_evaluator = WalkingFitness::new(config.fitness_config.clone());

        Self {
            rng: ChaCha8Rng::seed_from_u64(seed),
            config,
            state: WalkingState::Building,
            viewing_mode: ViewingMode::Watch,
            population: Vec::new(),
            generation: 0,
            evaluations: 0,
            current_genome: WalkingGenome::default(),
            current_executor: None,
            current_animator: None,
            current_initial_state: None,
            current_animation_start_age: Age::default(),
            mutator,
            fitness_evaluator,
            fabric: Fabric::new("Walking Evolution".to_string()),
        }
    }

    /// Adopt physics settings for evolution.
    pub fn adopt_physics(&self, context: &mut CrucibleContext) {
        *context.physics = ANIMATING.clone();
    }

    /// Main iteration loop - called each frame.
    pub fn iterate(&mut self, context: &mut CrucibleContext, iterations_per_frame: usize) {
        match self.state {
            WalkingState::Building => self.iterate_building(iterations_per_frame),
            WalkingState::Animating => self.iterate_animating(context, iterations_per_frame),
            WalkingState::Evaluating => self.iterate_evaluating(),
        }

        // Update display fabric
        if let Some(ref executor) = self.current_executor {
            self.fabric = executor.fabric.clone();
        }
        *context.fabric = self.fabric.clone();
        context.fabric.update_bounding_radius();

        // Send display state
        self.send_display_state(context);
    }

    /// Handle building state - run executor until complete.
    fn iterate_building(&mut self, iterations_per_frame: usize) {
        // Initialize executor if needed
        if self.current_executor.is_none() {
            let plan = self.current_genome.to_fabric_plan();
            self.current_executor = Some(FabricPlanExecutor::new_headless(plan));
        }

        let executor = self.current_executor.as_mut().unwrap();

        // Run executor iterations
        for _ in 0..iterations_per_frame {
            match executor.iterate() {
                IterateResult::Complete => {
                    // Building complete - start animation
                    self.fabric = executor.fabric.clone();
                    self.start_animation();
                    return;
                }
                IterateResult::Continue => {
                    // Check if BUILD phase is done
                    if matches!(executor.stage(), ExecutorStage::Building) {
                        if let Some(plan_runner) = executor.plan_runner() {
                            if plan_runner.is_done() {
                                executor.start_pretension();
                            }
                        }
                    }
                }
            }
        }

        // Update fabric for display
        self.fabric = executor.fabric.clone();
    }

    /// Start animation phase for fitness evaluation.
    fn start_animation(&mut self) {
        // Get animate phase from the fabric plan
        let plan = self.current_genome.to_fabric_plan();
        if let Some(animate_phase) = plan.animate_phase {
            // Measure initial state before animation
            self.current_initial_state = Some(WalkingFitness::measure_initial_state(&self.fabric));
            self.current_animation_start_age = self.fabric.age;

            // Create animator - it will set up actuators on the fabric
            // Store the animator for animation iteration
            self.current_animator = Some(Animator::new_headless(animate_phase, &mut self.fabric));
            self.state = WalkingState::Animating;
        } else {
            // No animation phase - go directly to evaluation
            self.state = WalkingState::Evaluating;
        }
    }

    /// Handle animation state - run animator until evaluation time complete.
    fn iterate_animating(&mut self, _context: &mut CrucibleContext, iterations_per_frame: usize) {
        if let Some(ref mut animator) = self.current_animator {
            // Run animator in headless mode
            animator.iterate_headless(&mut self.fabric, &ANIMATING, iterations_per_frame);

            // Check if evaluation duration has elapsed
            let elapsed = self.fabric.age.elapsed_since(self.current_animation_start_age);

            if elapsed.0 >= self.fitness_evaluator.evaluation_duration() {
                // Animation complete - evaluate
                self.state = WalkingState::Evaluating;
            }
        } else {
            // No animator - go to evaluation
            self.state = WalkingState::Evaluating;
        }
    }

    /// Handle evaluation state - compute fitness and select next genome.
    fn iterate_evaluating(&mut self) {
        // Compute fitness
        let details = if let Some(ref initial) = self.current_initial_state {
            let centroid = self.fabric.centroid();
            let (min_y, max_y) = self.fabric.altitude_range();
            let current_height = max_y - min_y;
            let elapsed = self.fabric.age.elapsed_since(self.current_animation_start_age);

            self.fitness_evaluator.calculate_fitness(
                initial.centroid,
                centroid,
                initial.height,
                current_height,
                elapsed.0,
                self.current_genome.push_count(),
                0.0, // TODO: Track actuator work
            )
        } else {
            WalkingFitnessDetails::invalid(self.current_genome.push_count())
        };

        // Create individual and insert into population
        let mut individual = WalkingIndividual::new(self.current_genome.clone());
        individual.fitness = details.fitness;
        individual.details = details;
        individual.generation = self.generation;

        self.insert_individual(individual);
        self.evaluations += 1;

        // Select and mutate for next evaluation
        self.select_and_mutate();

        // Reset state for next evaluation
        self.current_executor = None;
        self.current_animator = None;
        self.current_initial_state = None;
        self.state = WalkingState::Building;
    }

    /// Insert individual into population, maintaining size limit.
    fn insert_individual(&mut self, individual: WalkingIndividual) {
        self.population.push(individual);

        // Sort by fitness (descending)
        self.population.sort_by(|a, b| {
            b.fitness.partial_cmp(&a.fitness).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Trim to population size
        if self.population.len() > self.config.population_size {
            self.population.truncate(self.config.population_size);
        }
    }

    /// Select parent and mutate for next evaluation.
    fn select_and_mutate(&mut self) {
        self.generation += 1;

        if self.population.is_empty() {
            // First individual - use default genome
            self.current_genome = WalkingGenome::default();
            return;
        }

        // Tournament selection: pick 3 random, choose best
        let tournament_size = 3.min(self.population.len());
        let mut best_idx = self.rng.random_range(0..self.population.len());
        let mut best_fitness = self.population[best_idx].fitness;

        for _ in 1..tournament_size {
            let idx = self.rng.random_range(0..self.population.len());
            if self.population[idx].fitness > best_fitness {
                best_idx = idx;
                best_fitness = self.population[idx].fitness;
            }
        }

        // Clone parent genome and mutate
        let mut genome = self.population[best_idx].genome.clone();
        self.mutator.mutate(&mut genome, &mut self.rng);
        self.current_genome = genome;
    }

    /// Toggle between Watch and Fast viewing modes.
    pub fn toggle_viewing_mode(&mut self) {
        self.viewing_mode = match self.viewing_mode {
            ViewingMode::Watch => ViewingMode::Fast,
            ViewingMode::Fast => ViewingMode::Watch,
        };
    }

    /// Get current viewing mode.
    pub fn viewing_mode(&self) -> ViewingMode {
        self.viewing_mode
    }

    /// Send display state update.
    fn send_display_state(&self, context: &CrucibleContext) {
        let mode_suffix = match self.viewing_mode {
            ViewingMode::Watch => "",
            ViewingMode::Fast => " [Fast]",
        };

        let state_str = match self.state {
            WalkingState::Building => "Building",
            WalkingState::Animating => "Animating",
            WalkingState::Evaluating => "Evaluating",
        };

        let mut left_details = vec![
            format!("State: {}", state_str),
            format!("Gen: {}", self.generation),
            format!("Evals: {}", self.evaluations),
            format!("Pop: {}", self.population.len()),
        ];

        // Add best fitness info
        if let Some(best) = self.population.first() {
            left_details.push(String::new());
            left_details.push(format!("Best: {:.4}", best.fitness));
            left_details.push(format!("  Dist: {:.3}m", best.details.distance));
            left_details.push(format!("  Vel: {:.4}m/s", best.details.velocity));
            left_details.push(format!("  Pushes: {}", best.genome.push_count()));
        }

        // Add current genome info
        left_details.push(String::new());
        left_details.push(format!("Current: {} pushes", self.current_genome.push_count()));

        let display = DisplayState {
            title: Some(format!("Walking Evolution{}", mode_suffix)),
            subtitle: Some(format!("Generation {}", self.generation)),
            left_details,
            right_details: vec![],
        };

        let _ = context
            .radio
            .send_event(LabEvent::UpdateState(StateChange::SetDisplayState(display)));
    }
}

impl Default for WalkingEvolution {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_walking_evolution_creation() {
        let evolution = WalkingEvolution::new();
        assert_eq!(evolution.state, WalkingState::Building);
        assert!(evolution.population.is_empty());
    }

    #[test]
    fn test_walking_evolution_with_seed() {
        let config = WalkingEvolutionConfig::default();
        let evo1 = WalkingEvolution::with_seed(42, config.clone());
        let evo2 = WalkingEvolution::with_seed(42, config);

        // Same seed should produce same initial genome
        assert_eq!(
            evo1.current_genome.push_count(),
            evo2.current_genome.push_count()
        );
    }
}
