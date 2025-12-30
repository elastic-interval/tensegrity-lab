use crate::build::evo::fitness::FitnessEvaluator;
use crate::build::evo::grower::{GrowthConfig, Grower};
use crate::build::evo::population::{MutationType, Population};
use crate::crucible_context::CrucibleContext;
use crate::fabric::physics::presets::SETTLING;
use crate::fabric::physics::{Physics, Surface, SurfaceCharacter};
use crate::fabric::Fabric;
use crate::{LabEvent, StateChange, ITERATION_DURATION};
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
            population_size: 20,  // Larger population for more diversity
            seed_push_count: 3,
            seed_settle_seconds: 1.5,   // Faster settling
            mutation_settle_seconds: 3.5, // Time to fall and settle
            push_length: 3.0,  // Longer pushes look less thick visually
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
    rng: ChaCha8Rng,
    #[allow(dead_code)]
    seed: u64,
    population: Population,
    evaluator: FitnessEvaluator,
    config: EvolutionConfig,
    state: EvolutionState,
    current_fabric: Option<Fabric>,
    current_push_count: usize,
    current_parent_mutations: usize,
    current_parent_log: Vec<(MutationType, f32)>,
    current_mutation: MutationType,
    settling_physics: Physics,
    pub fabric: Fabric,
    evaluations: usize,
    grower: Grower,
    viewing_mode: ViewingMode,
    max_fitness_ever: f32,
    max_height_ever: f32,
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

        // Use settling physics with frozen surface so joints stick when they land
        let mut settling_physics = SETTLING.clone();
        settling_physics.surface = Some(Surface::new(SurfaceCharacter::Frozen, 1.0));

        Self {
            rng: ChaCha8Rng::seed_from_u64(seed.wrapping_add(1)),
            seed,
            population,
            evaluator: FitnessEvaluator::new(),
            config,
            state: EvolutionState::CreatingSeed,
            current_fabric: None,
            current_push_count: 0,
            current_parent_mutations: 0,
            current_parent_log: Vec::new(),
            current_mutation: MutationType::Seed,
            settling_physics,
            fabric: Fabric::new(format!("Evo-{}", seed)),
            evaluations: 0,
            grower: Grower::new(seed.wrapping_add(2), growth_config),
            viewing_mode: ViewingMode::Watch,
            max_fitness_ever: 0.0,
            max_height_ever: 0.0,
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
    /// iterations_per_frame controls physics speed (from time_scale).
    pub fn iterate(&mut self, context: &mut CrucibleContext, iterations_per_frame: usize) {
        // Swap current_fabric back from context if we moved it there last frame
        if self.current_fabric.is_none() && !context.fabric.joints.is_empty() {
            // Take the fabric back from context for continued work
            let empty = Fabric::new("temp".to_string());
            self.current_fabric = Some(std::mem::replace(&mut *context.fabric, empty));
        }

        // Run step with the given iteration budget
        self.step(iterations_per_frame);

        // Update context fabric for display (swap instead of clone when possible)
        match self.viewing_mode {
            ViewingMode::Watch => {
                // Show current fabric being worked on - swap to avoid clone
                if let Some(fabric) = self.current_fabric.take() {
                    *context.fabric = fabric;
                    // Note: current_fabric is now None, will be swapped back next frame
                } else if let Some(best) = self.population.best_current() {
                    // Fallback: clone from population (can't move)
                    *context.fabric = best.fabric.clone();
                }
            }
            ViewingMode::Fast => {
                // Show best - must clone since it stays in population
                if let Some(best) = self.population.best_current() {
                    *context.fabric = best.fabric.clone();
                }
            }
        }

        // Update bounding radius for proper surface sizing and camera
        context.fabric.update_bounding_radius();

        // Calculate current population stats using CACHED values (no recalculation!)
        let mut current_max_fitness = 0.0f32;
        let mut current_max_height = 0.0f32;
        let mut total_fitness = 0.0f32;
        let mut total_height = 0.0f32;
        let count = self.population.pool().len() as f32;

        for ind in self.population.pool() {
            // Use cached values - no expensive recalculation
            current_max_fitness = current_max_fitness.max(ind.fitness);
            current_max_height = current_max_height.max(ind.height);
            total_fitness += ind.fitness;
            total_height += ind.height;
        }

        // Update high-water marks (NEVER decrease)
        self.max_fitness_ever = self.max_fitness_ever.max(current_max_fitness);
        self.max_height_ever = self.max_height_ever.max(current_max_height);

        let avg_fitness = if count > 0.0 { total_fitness / count } else { 0.0 };
        let avg_height = if count > 0.0 { total_height / count } else { 0.0 };

        // Get mutation count of the displayed fabric
        let displayed_mutations = match self.viewing_mode {
            ViewingMode::Watch => self.current_parent_mutations + 1, // Current offspring
            ViewingMode::Fast => self.population.best_current()
                .map(|ind| ind.mutations)
                .unwrap_or(0),
        };

        let label = format!(
            "Mut:{} | Fit:{:.5}({:.5}) | Ht:{:.5}({:.5})m",
            displayed_mutations,
            self.max_fitness_ever, avg_fitness,
            self.max_height_ever, avg_height
        );
        let _ = context.radio.send_event(LabEvent::UpdateState(StateChange::SetStageLabel(label)));
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
    fn step(&mut self, iterations_per_frame: usize) {
        match self.state.clone() {
            EvolutionState::CreatingSeed => {
                self.create_seed();
            }
            EvolutionState::Seeding => {
                self.seed_population();
            }
            EvolutionState::Settling { remaining_iterations } => {
                self.settle_step(remaining_iterations, iterations_per_frame);
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
        let iterations = (self.config.seed_settle_seconds / ITERATION_DURATION.secs) as usize;
        self.state = EvolutionState::Settling {
            remaining_iterations: iterations,
        };
    }

    /// Fill population with clones of the settled seed.
    /// All diversity comes from evolution, not seeding.
    fn seed_population(&mut self) {
        if self.population.is_full() {
            self.state = EvolutionState::Evolving;
            return;
        }

        // Clone the settled seed as-is (no mutations - let evolution create diversity)
        if let Some(ref seed_fabric) = self.current_fabric {
            let fabric = seed_fabric.clone();
            let details = self.evaluator.evaluate_detailed(&fabric, self.current_push_count);
            self.population.add_initial(fabric, details.fitness, details.height, self.current_push_count);
            self.evaluations += 1;
        }
    }

    /// Perform settling iterations based on time_scale (iterations_per_frame).
    fn settle_step(&mut self, remaining: usize, iterations_per_frame: usize) {
        if remaining == 0 {
            // Done settling - centralize the fabric so it stays in view
            if let Some(ref mut fabric) = self.current_fabric {
                let translation = fabric.centralize_translation(None);
                fabric.apply_translation(translation);
            }
            // Transition to next state
            if self.population.is_full() {
                self.state = EvolutionState::Evaluating;
            } else {
                self.state = EvolutionState::Seeding;
            }
            return;
        }

        // Use iterations_per_frame from time_scale, but cap to stay responsive
        // Max 5000 iterations per frame to keep UI smooth while allowing faster settling
        let batch = iterations_per_frame.min(5000).min(remaining);

        if let Some(ref mut fabric) = self.current_fabric {
            for _ in 0..batch {
                fabric.iterate(&self.settling_physics);
            }
        }

        self.state = EvolutionState::Settling {
            remaining_iterations: remaining - batch,
        };
    }

    /// Evaluate current fabric and insert into population.
    fn evaluate_current(&mut self) {
        if let Some(fabric) = self.current_fabric.take() {
            let details = self.evaluator.evaluate_detailed(&fabric, self.current_push_count);
            let parent_log = self.current_parent_log.clone();
            let mutation = self.current_mutation.clone();

            // Try to insert into population
            self.population.try_insert(
                fabric, details.fitness, details.height, self.current_push_count,
                self.current_parent_mutations, parent_log, mutation,
            );
            self.population.next_generation();
            self.evaluations += 1;
        }
        self.state = EvolutionState::Evolving;
    }

    /// Main evolution step: pick parent from population, mutate, settle, evaluate.
    fn evolution_step(&mut self) {
        // Pick a random parent from the population
        let (mut offspring, parent_push_count, parent_mutations, parent_log) =
            match self.population.pick_random() {
                Some(ind) => (ind.fabric.clone(), ind.push_count, ind.mutations, ind.mutation_log.clone()),
                None => return,
            };

        self.current_parent_mutations = parent_mutations;
        self.current_parent_log = parent_log;
        // Check if parent has height
        let height = self.evaluator.evaluate_detailed(&offspring, parent_push_count).height;

        // Apply mutation based on structure state
        let new_push_count = if height < 0.1 {
            // Structure is flat - try removing a pull to let it unfold
            let mutation = if self.grower.remove_random_pull(&mut offspring) {
                MutationType::FlatRemovePull
            } else {
                self.grower.add_more_connections(&mut offspring);
                MutationType::FlatAddConnections
            };
            self.current_mutation = mutation;

            // Lift flat structures and add large perturbations to help them snap open
            let lift_altitude = 0.2;
            let translation = offspring.centralize_translation(Some(lift_altitude));
            offspring.apply_translation(translation);

            let perturbation_size = 0.05; // 5cm random nudges for flat structures
            for joint in offspring.joints.values_mut() {
                joint.location.x += self.rng.random_range(-perturbation_size..perturbation_size);
                joint.location.y += self.rng.random_range(-perturbation_size..perturbation_size);
                joint.location.z += self.rng.random_range(-perturbation_size..perturbation_size);
            }
            offspring.zero_velocities();

            parent_push_count
        } else {
            // Structure has height - apply weighted random mutation
            let (count, mutation) = self.grower.apply_random_mutation(&mut offspring, parent_push_count);
            self.current_mutation = mutation.clone();

            // Lift structure slightly so frozen joints unstick from floor
            let lift_altitude = 0.1; // 10cm above floor
            let translation = offspring.centralize_translation(Some(lift_altitude));
            offspring.apply_translation(translation);
            offspring.zero_velocities();

            count
        };

        self.current_fabric = Some(offspring);
        self.current_push_count = new_push_count;

        // Settling times: long enough to see physics settle completely
        let settle_seconds = match &self.current_mutation {
            MutationType::ShortenPull | MutationType::LengthenPull => 4.0, // Fine-tuning mutations
            MutationType::AddPush | MutationType::RemovePull => 6.0,       // Structural changes need more
            _ => self.config.mutation_settle_seconds,                       // Flat structures
        };

        let iterations = (settle_seconds / ITERATION_DURATION.secs) as usize;
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
