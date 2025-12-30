use crate::build::evo::fitness::FitnessEvaluator;
use crate::build::evo::genome::Genome;
use crate::build::evo::grower::{GrowthConfig, GrowthResult, Grower};
use crate::build::evo::population::Population;
use crate::crucible_context::CrucibleContext;
use crate::fabric::physics::presets::CONSTRUCTION;
use crate::fabric::Fabric;
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;

/// Configuration for the evolution process.
#[derive(Clone, Debug)]
pub struct EvolutionConfig {
    /// Population capacity (100-1000 recommended)
    pub population_size: usize,
    /// Number of mutations to try per parent selection
    pub mutations_per_parent: usize,
    /// Maximum growth steps per structure
    pub max_growth_steps: usize,
    /// Physics settling iterations between steps
    pub settle_iterations: usize,
    /// Push interval length (meters)
    pub push_length: f32,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            population_size: 100,
            mutations_per_parent: 3,
            max_growth_steps: 50,
            settle_iterations: 5000,
            push_length: 1.0,
        }
    }
}

/// State of the evolution process.
#[derive(Debug, Clone, PartialEq)]
pub enum EvolutionState {
    /// Initializing the population with random genomes
    Seeding,
    /// Main evolution loop: select, mutate, evaluate, compete
    Evolving,
    /// Currently growing a structure
    Growing,
    /// Currently settling physics
    Settling,
}

/// Main controller for evolutionary tensegrity system.
///
/// Manages a live population of tensegrity structures that evolve through
/// blind variation and natural selection.
pub struct Evolution {
    /// RNG for mutation and selection
    rng: ChaCha8Rng,
    /// The seed used for this evolution run
    seed: u64,
    /// Live population of individuals
    population: Population,
    /// Fitness evaluator
    evaluator: FitnessEvaluator,
    /// Configuration
    config: EvolutionConfig,
    /// Current state
    state: EvolutionState,
    /// Current grower (when growing a structure)
    current_grower: Option<Grower>,
    /// Pending mutations to evaluate
    pending_mutations: Vec<Genome>,
    /// Settling countdown
    settle_countdown: usize,
    /// The visible fabric (best current or growing)
    pub visible_fabric: Fabric,
    /// Total evaluations performed
    evaluations: usize,
}

impl Evolution {
    /// Create a new evolution controller.
    pub fn new(seed: u64, config: EvolutionConfig) -> Self {
        let population = Population::new(seed, config.population_size);

        Self {
            rng: ChaCha8Rng::seed_from_u64(seed.wrapping_add(1)), // Offset from population seed
            seed,
            population,
            evaluator: FitnessEvaluator::new(),
            config,
            state: EvolutionState::Seeding,
            current_grower: None,
            pending_mutations: Vec::new(),
            settle_countdown: 0,
            visible_fabric: Fabric::new(format!("Evo-{}", seed)),
            evaluations: 0,
        }
    }

    /// Create with default configuration.
    pub fn with_seed(seed: u64) -> Self {
        Self::new(seed, EvolutionConfig::default())
    }

    /// Adopt physics settings for evolution.
    pub fn adopt_physics(&self, context: &mut CrucibleContext) {
        *context.physics = CONSTRUCTION;
    }

    /// Main iteration loop - called each frame.
    pub fn iterate(&mut self, context: &mut CrucibleContext) {
        match self.state {
            EvolutionState::Seeding => {
                self.seed_population_step();
            }
            EvolutionState::Evolving => {
                self.evolution_step();
            }
            EvolutionState::Growing => {
                self.growing_step();
            }
            EvolutionState::Settling => {
                self.settling_step(context);
            }
        }

        // Update visible fabric
        self.update_visible_fabric();

        // Update context fabric
        *context.fabric = self.visible_fabric.clone();
    }

    /// Seed the population with random initial structures.
    fn seed_population_step(&mut self) {
        if self.population.is_full() {
            self.state = EvolutionState::Evolving;
            return;
        }

        // Grow a structure with empty genome
        let genome = Genome::new();
        self.start_growing(genome);
    }

    /// Main evolution step: select parent, create mutations.
    fn evolution_step(&mut self) {
        if self.pending_mutations.is_empty() {
            // Select a parent and create mutations
            if let Some(parent_genome) = self.population.pick_parent_genome() {
                for _ in 0..self.config.mutations_per_parent {
                    // Pick a random position to insert a skip
                    let position = self.rng.random_range(0..100);
                    let mutated = parent_genome.with_skip_at(position);
                    self.pending_mutations.push(mutated);
                }
            }
        }

        if let Some(genome) = self.pending_mutations.pop() {
            self.start_growing(genome);
        }
    }

    /// Start growing a structure from a genome.
    fn start_growing(&mut self, genome: Genome) {
        let growth_config = GrowthConfig {
            max_steps: self.config.max_growth_steps,
            settle_iterations: self.config.settle_iterations,
            ..Default::default()
        };

        let grower = Grower::new(self.seed, genome, growth_config);
        self.current_grower = Some(grower);
        self.state = EvolutionState::Growing;
    }

    /// Continue growing the current structure.
    fn growing_step(&mut self) {
        if let Some(ref mut grower) = self.current_grower {
            match grower.grow_step() {
                GrowthResult::Continue => {
                    // Keep growing
                }
                GrowthResult::Complete | GrowthResult::Failed(_) => {
                    // Start settling
                    self.settle_countdown = self.config.settle_iterations / 1000;
                    self.state = EvolutionState::Settling;
                }
            }
        }
    }

    /// Settle physics after growth.
    fn settling_step(&mut self, context: &mut CrucibleContext) {
        if self.settle_countdown > 0 {
            // Run physics iterations on the grower's fabric
            if let Some(ref mut grower) = self.current_grower {
                for _ in 0..1000 {
                    grower.fabric.iterate(context.physics);
                }
            }
            self.settle_countdown -= 1;
        } else {
            // Evaluate and insert into population
            self.finish_current();
        }
    }

    /// Finish evaluating the current structure.
    fn finish_current(&mut self) {
        if let Some(grower) = self.current_grower.take() {
            let fitness = self.evaluator.evaluate(&grower.fabric);
            let genome = grower.clone_genome();

            self.evaluations += 1;

            if self.state == EvolutionState::Settling
                && self.population.size() < self.config.population_size
            {
                // Still seeding
                self.population.add_initial(genome, fitness);
                self.state = EvolutionState::Seeding;
            } else {
                // Normal evolution
                self.population.try_insert(genome, fitness);
                self.population.next_generation();
                self.state = EvolutionState::Evolving;
            }
        }
    }

    /// Update the visible fabric for display.
    fn update_visible_fabric(&mut self) {
        if let Some(ref grower) = self.current_grower {
            self.visible_fabric = grower.fabric.clone();
        } else if let Some(best) = self.population.best_current() {
            // Regrow the best for display
            let growth_config = GrowthConfig {
                max_steps: self.config.max_growth_steps,
                ..Default::default()
            };
            let mut grower = Grower::new(self.seed, best.genome.clone(), growth_config);
            grower.grow_complete();
            self.visible_fabric = grower.fabric;
        }
    }

    /// Get current evolution state.
    pub fn state(&self) -> &EvolutionState {
        &self.state
    }

    /// Get population statistics.
    pub fn stats(&self) -> EvolutionStats {
        let pop_stats = self.population.stats();
        EvolutionStats {
            generation: pop_stats.generation,
            population_size: pop_stats.size,
            evaluations: self.evaluations,
            best_fitness: pop_stats.max_fitness,
            mean_fitness: pop_stats.mean_fitness,
            min_fitness: pop_stats.min_fitness,
            diversity: pop_stats.std_dev,
        }
    }

    /// Get the best individual's genome.
    pub fn best_genome(&self) -> Option<Genome> {
        self.population.best_ever().map(|ind| ind.genome.clone())
    }

    /// Get reference to the population.
    pub fn population(&self) -> &Population {
        &self.population
    }
}

/// Statistics about the evolution process.
#[derive(Debug, Clone, Default)]
pub struct EvolutionStats {
    /// Current generation
    pub generation: usize,
    /// Current population size
    pub population_size: usize,
    /// Total evaluations performed
    pub evaluations: usize,
    /// Best fitness in population
    pub best_fitness: f32,
    /// Mean fitness in population
    pub mean_fitness: f32,
    /// Minimum fitness in population
    pub min_fitness: f32,
    /// Fitness diversity (std dev)
    pub diversity: f32,
}
