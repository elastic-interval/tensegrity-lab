use crate::build::evo::fitness::FitnessEvaluator;
use crate::build::evo::grower::{GrowthConfig, Grower};
use crate::build::evo::population::Population;
use crate::crucible_context::CrucibleContext;
use crate::fabric::physics::presets::SETTLING;
use crate::fabric::physics::{Physics, Surface, SurfaceCharacter};
use crate::fabric::Fabric;
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;

/// Configuration for the evolution process.
#[derive(Clone, Debug)]
pub struct EvolutionConfig {
    /// Population capacity
    pub population_size: usize,
    /// Number of push intervals in initial seed
    pub seed_push_count: usize,
    /// Seconds to settle initial seed
    pub seed_settle_seconds: f32,
    /// Seconds to settle after each mutation
    pub mutation_settle_seconds: f32,
    /// Push interval length (meters)
    pub push_length: f32,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            population_size: 100,
            seed_push_count: 3,
            seed_settle_seconds: 5.0,
            mutation_settle_seconds: 2.0,
            push_length: 1.0,
        }
    }
}

/// State of the evolution process.
#[derive(Debug, Clone, PartialEq)]
pub enum EvolutionState {
    /// Creating and settling the initial seed
    CreatingSeed,
    /// Populating from seed variations
    Seeding,
    /// Settling a structure (seed or offspring)
    Settling { remaining_iterations: usize },
    /// Evaluating and inserting into population
    Evaluating,
    /// Main evolution loop
    Evolving,
}

/// Viewing mode for evolution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewingMode {
    /// Watch physics happening in real-time
    Watch,
    /// Run fast, only show best structures
    Fast,
}

/// Main controller for evolutionary tensegrity system.
pub struct Evolution {
    /// RNG for random decisions
    rng: ChaCha8Rng,
    /// The seed used for this evolution run
    #[allow(dead_code)]
    seed: u64,
    /// Live population
    population: Population,
    /// Fitness evaluator
    evaluator: FitnessEvaluator,
    /// Configuration
    config: EvolutionConfig,
    /// Current state
    state: EvolutionState,
    /// Current fabric being grown/settled
    current_fabric: Option<Fabric>,
    /// Push count of current fabric
    current_push_count: usize,
    /// Physics for settling (with gravity)
    settling_physics: Physics,
    /// The visible fabric
    pub fabric: Fabric,
    /// Total evaluations performed
    evaluations: usize,
    /// Grower for mutations
    grower: Grower,
    /// Current viewing mode
    viewing_mode: ViewingMode,
    /// Best fitness seen (for detecting new best)
    best_fitness_seen: f32,
}

impl Evolution {
    /// Create a new evolution controller.
    pub fn new(seed: u64, config: EvolutionConfig) -> Self {
        let population = Population::new(seed, config.population_size);

        let growth_config = GrowthConfig {
            push_length: crate::units::Meters(config.push_length),
            seed_settle_seconds: config.seed_settle_seconds,
            mutation_settle_seconds: config.mutation_settle_seconds,
            seed_push_count: config.seed_push_count,
            ..Default::default()
        };

        // Use settling physics with bouncy surface for gravity
        let mut settling_physics = SETTLING.clone();
        settling_physics.surface = Some(Surface::new(SurfaceCharacter::Bouncy, 1.0));

        Self {
            rng: ChaCha8Rng::seed_from_u64(seed.wrapping_add(1)),
            seed,
            population,
            evaluator: FitnessEvaluator::new(),
            config,
            state: EvolutionState::CreatingSeed,
            current_fabric: None,
            current_push_count: 0,
            settling_physics,
            fabric: Fabric::new(format!("Evo-{}", seed)),
            evaluations: 0,
            grower: Grower::new(seed.wrapping_add(2), growth_config),
            viewing_mode: ViewingMode::Watch,
            best_fitness_seen: 0.0,
        }
    }

    /// Create with default configuration.
    pub fn with_seed(seed: u64) -> Self {
        Self::new(seed, EvolutionConfig::default())
    }

    /// Adopt physics settings for evolution.
    pub fn adopt_physics(&self, context: &mut CrucibleContext) {
        *context.physics = self.settling_physics.clone();
    }

    /// Main iteration loop - called each frame.
    /// Behavior depends on viewing mode.
    pub fn iterate(&mut self, context: &mut CrucibleContext) {
        // Always run exactly one step per frame - time budgeting happens inside settle_step
        self.step();

        // Update visible fabric
        match self.viewing_mode {
            ViewingMode::Watch => {
                // Show current fabric being worked on (so we see physics)
                if let Some(ref fabric) = self.current_fabric {
                    self.fabric = fabric.clone();
                } else if let Some(best) = self.population.best_current() {
                    self.fabric = best.fabric.clone();
                }
            }
            ViewingMode::Fast => {
                // Always show best current structure
                if let Some(best) = self.population.best_current() {
                    if best.fitness > self.best_fitness_seen {
                        self.best_fitness_seen = best.fitness;
                    }
                    self.fabric = best.fabric.clone();
                }
            }
        }

        // Update bounding radius for proper surface sizing and camera
        self.fabric.update_bounding_radius();

        // Update context fabric
        *context.fabric = self.fabric.clone();
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

    /// Single evolution step.
    fn step(&mut self) {
        match self.state.clone() {
            EvolutionState::CreatingSeed => {
                self.create_seed();
            }
            EvolutionState::Seeding => {
                self.seed_population();
            }
            EvolutionState::Settling { remaining_iterations } => {
                self.settle_step(remaining_iterations);
            }
            EvolutionState::Evaluating => {
                self.evaluate_current();
            }
            EvolutionState::Evolving => {
                self.evolution_step();
            }
        }
    }

    /// Create the initial seed structure.
    fn create_seed(&mut self) {
        let (fabric, push_count) = self.grower.create_seed();
        self.current_fabric = Some(fabric);
        self.current_push_count = push_count;

        // Start settling
        let iterations = (self.config.seed_settle_seconds / 0.00005) as usize;
        self.state = EvolutionState::Settling {
            remaining_iterations: iterations,
        };
    }

    /// Add variations of the seed to the population.
    fn seed_population(&mut self) {
        if self.population.is_full() {
            self.state = EvolutionState::Evolving;
            return;
        }

        // Clone the settled seed and add to population (no additional settling)
        if let Some(ref seed_fabric) = self.current_fabric {
            let fabric = seed_fabric.clone();
            let fitness = self.evaluator.evaluate(&fabric, self.current_push_count);
            self.population.add_initial(fabric, fitness, self.current_push_count);
            self.evaluations += 1;
        }
    }

    /// Perform settling iterations with time budget.
    fn settle_step(&mut self, remaining: usize) {
        if remaining == 0 {
            // Done settling
            if self.population.is_full() {
                self.state = EvolutionState::Evaluating;
            } else {
                self.state = EvolutionState::Seeding;
            }
            return;
        }

        // Time budget per frame to stay responsive
        // Watch: slower so we can see physics
        // Fast: faster but still responsive
        let max_millis = match self.viewing_mode {
            ViewingMode::Watch => 1.0,  // 1ms for physics
            ViewingMode::Fast => 5.0,   // 5ms for physics
        };

        let start = std::time::Instant::now();
        let mut done = 0;

        if let Some(ref mut fabric) = self.current_fabric {
            while done < remaining {
                fabric.iterate(&self.settling_physics);
                done += 1;

                // Check time every 100 iterations to reduce overhead
                if done % 100 == 0 {
                    if start.elapsed().as_secs_f64() * 1000.0 > max_millis {
                        break;
                    }
                }
            }
        }

        self.state = EvolutionState::Settling {
            remaining_iterations: remaining - done,
        };
    }

    /// Evaluate current fabric and insert into population.
    fn evaluate_current(&mut self) {
        if let Some(fabric) = self.current_fabric.take() {
            let fitness = self.evaluator.evaluate(&fabric, self.current_push_count);
            self.population.try_insert(fabric, fitness, self.current_push_count);
            self.population.next_generation();
            self.evaluations += 1;
        }
        self.state = EvolutionState::Evolving;
    }

    /// Main evolution step: select parent, mutate, settle, evaluate.
    fn evolution_step(&mut self) {
        // Pick a parent
        let (parent_fabric, parent_push_count) = match self.population.pick_random() {
            Some(ind) => (ind.fabric.clone(), ind.push_count),
            None => return,
        };

        let mut offspring = parent_fabric;
        let mut new_push_count = parent_push_count;

        // Check if parent has height
        let height = self.evaluator.evaluate_detailed(&offspring, parent_push_count).height;

        if height < 0.1 {
            // Structure is flat - try to add more connections instead of new push
            self.grower.add_more_connections(&mut offspring);
            // Keep same push count since we only added pulls
        } else {
            // Structure has height - choose mutation type randomly
            let mutation_choice = self.rng.random_range(0.0..1.0);

            if mutation_choice < 0.4 {
                // 40% chance: shorten a random pull (can increase height)
                self.grower.shorten_random_pull(&mut offspring);
            } else {
                // 60% chance: add a new push
                new_push_count = self.grower.mutate(&mut offspring, parent_push_count);
            }
        }

        self.current_fabric = Some(offspring);
        self.current_push_count = new_push_count;

        // Start settling the mutation
        let iterations = (self.config.mutation_settle_seconds / 0.00005) as usize;
        self.state = EvolutionState::Settling {
            remaining_iterations: iterations,
        };
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
            avg_push_count: pop_stats.avg_push_count,
        }
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
    /// Average push count in population
    pub avg_push_count: f32,
}
