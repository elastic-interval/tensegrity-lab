use crate::build::evo::fitness::{FitnessDetails, FitnessEvaluator, FitnessName};
use crate::build::evo::grower::{GrowthConfig, Grower, MutationWeights};
use crate::build::evo::population::{MutationType, Population};
use crate::crucible_context::CrucibleContext;
use crate::fabric::physics::presets::SETTLING;
use crate::fabric::physics::{Physics, Surface, SurfaceCharacter};
use crate::fabric::Fabric;
use crate::{DisplayState, LabEvent, StateChange, ITERATION_DURATION};
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

/// Configuration for the evolution process.
#[derive(Clone, Debug)]
pub struct EvolutionConfig {
    pub name: String,
    pub population_size: usize,
    pub seed_push_count: usize,
    pub seed_settle_seconds: f32,
    pub mutation_settle_seconds: f32,
    /// Push length in meters
    pub push_length: f32,
    pub mutation_weights: MutationWeights,
    pub fitness: FitnessName,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            population_size: 20,  // Larger population for more diversity
            seed_push_count: 3,
            seed_settle_seconds: 1.5,   // Faster settling
            mutation_settle_seconds: 3.5, // Time to fall and settle
            push_length: 3.0,  // Longer pushes look less thick visually
            mutation_weights: MutationWeights::default(),
            fitness: FitnessName::default(),
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
    /// Master RNG for generating individual seeds
    rng: ChaCha8Rng,
    population: Population,
    evaluator: FitnessEvaluator,
    config: EvolutionConfig,
    state: EvolutionState,
    current_fabric: Option<Fabric>,
    current_seed: u64,
    current_push_count: usize,
    current_parent_mutations: usize,
    current_parent_log: Vec<(MutationType, f32)>,
    current_mutation: MutationType,
    settling_physics: Physics,
    pub fabric: Fabric,
    evaluations: usize,
    grower: Grower,
    viewing_mode: ViewingMode,
    /// Cached fitness details for display (computed before fabric swap)
    cached_fitness: Option<FitnessDetails>,
}

impl Evolution {
    /// Create a new evolution controller with random seed from system time.
    pub fn new() -> Self {
        Self::with_config(EvolutionConfig::default())
    }

    /// Create with specific configuration.
    pub fn with_config(config: EvolutionConfig) -> Self {
        // Generate master seed from system time
        let master_seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(42);

        let population = Population::new(master_seed, config.population_size);

        let growth_config = GrowthConfig {
            push_length: crate::units::Meters(config.push_length),
            seed_settle_seconds: config.seed_settle_seconds,
            mutation_settle_seconds: config.mutation_settle_seconds,
            seed_push_count: config.seed_push_count,
            mutation_weights: config.mutation_weights.clone(),
            ..Default::default()
        };

        // Use settling physics with frozen surface so joints stick when they land
        let mut settling_physics = SETTLING.clone();
        settling_physics.surface = Some(Surface::new(SurfaceCharacter::Frozen, 1.0));

        // Create fitness evaluator based on config
        let evaluator = FitnessEvaluator::with_function(config.fitness.fitness_function());

        Self {
            rng: ChaCha8Rng::seed_from_u64(master_seed),
            population,
            evaluator,
            config,
            state: EvolutionState::CreatingSeed,
            current_fabric: None,
            current_seed: 0,
            current_push_count: 0,
            current_parent_mutations: 0,
            current_parent_log: Vec::new(),
            current_mutation: MutationType::Seed,
            settling_physics,
            fabric: Fabric::new("Evolution".to_string()),
            evaluations: 0,
            grower: Grower::new(master_seed.wrapping_add(1), growth_config),
            viewing_mode: ViewingMode::Watch,
            cached_fitness: None,
        }
    }

    /// Create with a specific master seed (for deterministic testing).
    #[cfg(test)]
    pub fn with_master_seed(master_seed: u64, config: EvolutionConfig) -> Self {
        let population = Population::new(master_seed, config.population_size);

        let growth_config = GrowthConfig {
            push_length: crate::units::Meters(config.push_length),
            seed_settle_seconds: config.seed_settle_seconds,
            mutation_settle_seconds: config.mutation_settle_seconds,
            seed_push_count: config.seed_push_count,
            mutation_weights: config.mutation_weights.clone(),
            ..Default::default()
        };

        let mut settling_physics = SETTLING.clone();
        settling_physics.surface = Some(Surface::new(SurfaceCharacter::Frozen, 1.0));

        // Create fitness evaluator based on config
        let evaluator = FitnessEvaluator::with_function(config.fitness.fitness_function());

        Self {
            rng: ChaCha8Rng::seed_from_u64(master_seed),
            population,
            evaluator,
            config,
            state: EvolutionState::CreatingSeed,
            current_fabric: None,
            current_seed: 0,
            current_push_count: 0,
            current_parent_mutations: 0,
            current_parent_log: Vec::new(),
            current_mutation: MutationType::Seed,
            settling_physics,
            fabric: Fabric::new("Evolution".to_string()),
            evaluations: 0,
            grower: Grower::new(master_seed.wrapping_add(1), growth_config),
            viewing_mode: ViewingMode::Watch,
            cached_fitness: None,
        }
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

        // Cache fitness details BEFORE swap (fixes "no data" bug)
        if let Some(ref fabric) = self.current_fabric {
            self.cached_fitness = Some(self.evaluator.evaluate_detailed(fabric, self.current_push_count));
        }

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

        // Send unified display state
        self.send_display_state(context);
    }

    /// Send unified display state for evolution mode.
    fn send_display_state(&self, context: &CrucibleContext) {
        let pop_stats = self.population.stats();

        // Get fitness details based on viewing mode
        let (mutations, fitness_details) = match self.viewing_mode {
            ViewingMode::Watch => {
                // Use cached fitness (computed before swap)
                (self.current_parent_mutations + 1, self.cached_fitness.clone())
            }
            ViewingMode::Fast => {
                // Get from best in population
                self.population.best_current()
                    .map(|ind| {
                        let details = self.evaluator.evaluate_detailed(&ind.fabric, ind.push_count);
                        (ind.mutations, Some(details))
                    })
                    .unwrap_or((0, None))
            }
        };

        // Build left panel: fitness details + population stats
        let mut left_details = Vec::new();

        if let Some(ref details) = fitness_details {
            left_details.push(format!("Fitness: {:.3}", details.fitness));
            left_details.push(format!("Suspended: {} joints", details.suspended_joints));
            left_details.push(format!("Height: {:.2}m", details.height));
            left_details.push(format!("Intervals: {}", details.interval_count));
            left_details.push(String::new()); // Blank separator
        }

        left_details.push(format!("Best: {:.3}", pop_stats.max_fitness));
        left_details.push(format!("Avg: {:.3}", pop_stats.mean_fitness));
        left_details.push(format!("Avg Mut: {:.1}", pop_stats.avg_mutations));

        let mode_suffix = match self.viewing_mode {
            ViewingMode::Watch => "",
            ViewingMode::Fast => " [Fast]",
        };

        let display = DisplayState {
            title: Some(format!("Evolution: {}{}", self.config.name, mode_suffix)),
            subtitle: Some(format!("Mutation #{}", mutations)),
            left_details,
            right_details: vec![],
        };

        let _ = context.radio.send_event(LabEvent::UpdateState(StateChange::SetDisplayState(display)));
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
        // Check if we should analyze (TEMPORARY - for comparing evolved vs designed)
        let stats = self.population.stats();
        if self.viewing_mode == ViewingMode::Fast
            && stats.avg_push_count >= 15.0
            && self.population.is_full()
        {
            self.analyze_and_compare_to_triped();
        }

        // In Fast mode with full population, use parallel evolution
        #[cfg(not(target_arch = "wasm32"))]
        if self.viewing_mode == ViewingMode::Fast
            && self.population.is_full()
            && matches!(self.state, EvolutionState::Evolving)
        {
            self.parallel_evolution_step();
            return;
        }

        // Sequential mode: run multiple cycles in Fast mode
        let cycles = if self.viewing_mode == ViewingMode::Fast { 5 } else { 1 };

        for _ in 0..cycles {
            self.step_once(iterations_per_frame);
        }
    }

    /// Execute one step of the state machine.
    fn step_once(&mut self, iterations_per_frame: usize) {
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

    /// Parallel evolution step - evaluates multiple mutations concurrently.
    /// Only used in Fast mode on native (not WASM).
    #[cfg(not(target_arch = "wasm32"))]
    fn parallel_evolution_step(&mut self) {
        // Number of parallel evaluations (use available parallelism)
        let num_parallel = rayon::current_num_threads().min(8);

        // Collect parent info for parallel processing
        let parents: Vec<_> = (0..num_parallel)
            .filter_map(|_| {
                self.population.pick_random().map(|ind| {
                    (
                        ind.seed,
                        ind.fabric.clone(),
                        ind.push_count,
                        ind.mutations,
                        ind.mutation_log.clone(),
                    )
                })
            })
            .collect();

        if parents.is_empty() {
            return;
        }

        // Generate seeds for each parallel task
        let task_seeds: Vec<u64> = (0..parents.len())
            .map(|_| self.rng.random())
            .collect();

        // Clone config for parallel tasks
        let growth_config = GrowthConfig {
            push_length: crate::units::Meters(self.config.push_length),
            seed_settle_seconds: self.config.seed_settle_seconds,
            mutation_settle_seconds: self.config.mutation_settle_seconds,
            seed_push_count: self.config.seed_push_count,
            mutation_weights: self.config.mutation_weights.clone(),
            ..Default::default()
        };
        let settling_physics = self.settling_physics.clone();
        let settle_iterations = (self.config.mutation_settle_seconds / ITERATION_DURATION.secs) as usize;
        let fitness_name = self.config.fitness;

        // Run mutations in parallel
        let results: Vec<_> = parents
            .into_par_iter()
            .zip(task_seeds.into_par_iter())
            .map(|((seed, fabric, push_count, mutations, log), task_seed)| {
                Self::evaluate_mutation_parallel(
                    seed,
                    fabric,
                    push_count,
                    mutations,
                    log,
                    task_seed,
                    growth_config.clone(),
                    settling_physics.clone(),
                    settle_iterations,
                    fitness_name,
                )
            })
            .collect();

        // Insert results into population
        for result in results {
            let (seed, fabric, fitness, height, push_count, mutations, log, mutation) = result;
            self.population.try_insert(
                seed, fabric, fitness, height, push_count,
                mutations, log, mutation,
            );
            self.population.next_generation();
            self.evaluations += 1;
        }
    }

    /// Evaluate a single mutation in a parallel context.
    /// This is a static method that can run on any thread.
    #[cfg(not(target_arch = "wasm32"))]
    fn evaluate_mutation_parallel(
        parent_seed: u64,
        mut offspring: Fabric,
        parent_push_count: usize,
        parent_mutations: usize,
        parent_log: Vec<(MutationType, f32)>,
        task_seed: u64,
        growth_config: GrowthConfig,
        settling_physics: Physics,
        settle_iterations: usize,
        fitness_name: FitnessName,
    ) -> (u64, Fabric, f32, f32, usize, usize, Vec<(MutationType, f32)>, MutationType) {
        // Create local RNG and grower for this thread
        let mut rng = ChaCha8Rng::seed_from_u64(task_seed);
        let mut grower = Grower::new(task_seed, growth_config);
        let evaluator = FitnessEvaluator::with_function(fitness_name.fitness_function());

        // Evaluate height to determine mutation type
        let height = evaluator.evaluate_detailed(&offspring, parent_push_count).height;

        // Apply mutation based on structure state
        let (new_push_count, mutation) = if height < 0.1 {
            // Structure is flat
            let mutation = if grower.remove_random_pull(&mut offspring) {
                MutationType::FlatRemovePull
            } else {
                grower.add_more_connections(&mut offspring);
                MutationType::FlatAddConnections
            };

            // Lift and perturb
            let translation = offspring.centralize_translation(Some(0.05));
            offspring.apply_translation(translation);
            for joint in offspring.joints.values_mut() {
                joint.location.x += rng.random_range(-0.05..0.05);
                joint.location.y += rng.random_range(-0.05..0.05);
                joint.location.z += rng.random_range(-0.05..0.05);
            }
            offspring.zero_velocities();
            (parent_push_count, mutation)
        } else {
            // Structure has height
            let (count, mutation) = grower.apply_random_mutation(&mut offspring, parent_push_count);
            let translation = offspring.centralize_translation(Some(0.05));
            offspring.apply_translation(translation);
            offspring.zero_velocities();
            (count, mutation)
        };

        // Settle completely
        for _ in 0..settle_iterations {
            offspring.iterate(&settling_physics);
        }

        // Centralize after settling
        let translation = offspring.centralize_translation(None);
        offspring.apply_translation(translation);

        // Evaluate fitness
        let details = evaluator.evaluate_detailed(&offspring, new_push_count);

        (
            parent_seed,
            offspring,
            details.fitness,
            details.height,
            new_push_count,
            parent_mutations,
            parent_log,
            mutation,
        )
    }

    /// Create a new seed structure with a unique random seed.
    fn create_seed(&mut self) {
        // Generate a unique seed for this individual
        self.current_seed = self.rng.random();

        // Create a new grower with this seed for deterministic structure
        let growth_config = GrowthConfig {
            push_length: crate::units::Meters(self.config.push_length),
            seed_settle_seconds: self.config.seed_settle_seconds,
            mutation_settle_seconds: self.config.mutation_settle_seconds,
            seed_push_count: self.config.seed_push_count,
            mutation_weights: self.config.mutation_weights.clone(),
            ..Default::default()
        };
        let mut grower = Grower::new(self.current_seed, growth_config);

        let (fabric, push_count) = grower.create_seed();
        self.current_fabric = Some(fabric);
        self.current_push_count = push_count;
        // Store the grower for this individual's future mutations
        self.grower = grower;

        // Start settling
        let iterations = (self.config.seed_settle_seconds / ITERATION_DURATION.secs) as usize;
        self.state = EvolutionState::Settling {
            remaining_iterations: iterations,
        };
    }

    /// Add the settled individual to the population, then create next seed if not full.
    fn seed_population(&mut self) {
        // Add the settled individual to population
        if let Some(fabric) = self.current_fabric.take() {
            let details = self.evaluator.evaluate_detailed(&fabric, self.current_push_count);
            self.population.add_initial(self.current_seed, fabric, details.fitness, details.height, self.current_push_count);
            self.evaluations += 1;
        }

        // Check if population is full
        if self.population.is_full() {
            self.state = EvolutionState::Evolving;
        } else {
            // Create another unique seed structure
            self.state = EvolutionState::CreatingSeed;
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

        // In Fast mode, complete all settling at once
        // In Watch mode, do incremental batches for visual feedback
        let batch = if self.viewing_mode == ViewingMode::Fast {
            remaining // Complete all at once
        } else {
            // Cap to stay responsive while allowing faster settling
            iterations_per_frame.min(5000).min(remaining)
        };

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

            // Try to insert into population (offspring inherits parent's seed)
            self.population.try_insert(
                self.current_seed, fabric, details.fitness, details.height, self.current_push_count,
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
        let (parent_seed, mut offspring, parent_push_count, parent_mutations, parent_log) =
            match self.population.pick_random() {
                Some(ind) => (ind.seed, ind.fabric.clone(), ind.push_count, ind.mutations, ind.mutation_log.clone()),
                None => return,
            };

        self.current_seed = parent_seed;  // Offspring inherits parent's seed
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
            let lift_altitude = 0.05;
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
            let lift_altitude = 0.05; // 5cm above floor
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
            MutationType::AddPush | MutationType::RemovePull | MutationType::SplitPull => 6.0,  // Structural changes need more
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

    /// TEMPORARY: Analyze evolved structure and compare to designed Triped.
    /// Panics with detailed comparison to stop execution and show results.
    fn analyze_and_compare_to_triped(&self) {
        use crate::fabric::GeometricAnalysis;

        let best = match self.population.best_current() {
            Some(ind) => ind,
            None => return,
        };

        let fabric = &best.fabric;
        let analysis = fabric.analyze_buildability();
        let pop_stats = self.population.stats();

        // Triped baseline (measured from built structure in check_final_settled_state test)
        let triped = GeometricAnalysis {
            avg_pull_connections: 3.96,
            min_pull_connections: 3,
            max_pull_connections: 5,
            overpopulated_joints: 0,
            underpopulated_joints: 0,
            min_push_distance: 0.234, // 234mm
            crossing_count: 0,
            near_miss_count: 0,
        };

        let mut report = String::new();
        report.push_str("\n");
        report.push_str("╔══════════════════════════════════════════════════════════════════════════════╗\n");
        report.push_str("║                    EVOLVED vs DESIGNED STRUCTURE ANALYSIS                   ║\n");
        report.push_str("╠══════════════════════════════════════════════════════════════════════════════╣\n");
        report.push_str("║                                                                              ║\n");
        report.push_str(&format!("║  Population Stats:                                                           ║\n"));
        report.push_str(&format!("║    Generation: {:>6}     Evaluations: {:>8}                            ║\n",
            pop_stats.generation, self.evaluations));
        report.push_str(&format!("║    Best Fitness: {:>8.3}  Mean Fitness: {:>8.3}                         ║\n",
            pop_stats.max_fitness, pop_stats.mean_fitness));
        report.push_str(&format!("║    Avg Push Count: {:>5.1}   Avg Mutations: {:>5.1}                            ║\n",
            pop_stats.avg_push_count, pop_stats.avg_mutations));
        report.push_str("║                                                                              ║\n");
        report.push_str("╠══════════════════════════════════════════════════════════════════════════════╣\n");
        report.push_str("║                                                                              ║\n");
        report.push_str("║  METRIC                          EVOLVED         TRIPED          VERDICT     ║\n");
        report.push_str("║  ──────────────────────────────────────────────────────────────────────────  ║\n");

        // Avg pulls per joint
        let verdict_avg = if (analysis.avg_pull_connections - triped.avg_pull_connections).abs() < 0.5 { "✓ GOOD" } else if analysis.avg_pull_connections > 5.0 { "✗ TOO MANY" } else { "~ OK" };
        report.push_str(&format!("║  Avg pulls/joint               {:>8.2}        {:>8.2}        {:>10}  ║\n",
            analysis.avg_pull_connections, triped.avg_pull_connections, verdict_avg));

        // Min pulls per joint
        let verdict_min = if analysis.min_pull_connections >= triped.min_pull_connections { "✓ GOOD" } else { "✗ UNSTABLE" };
        report.push_str(&format!("║  Min pulls/joint               {:>8}        {:>8}        {:>10}  ║\n",
            analysis.min_pull_connections, triped.min_pull_connections, verdict_min));

        // Max pulls per joint
        let verdict_max = if analysis.max_pull_connections <= triped.max_pull_connections { "✓ GOOD" } else if analysis.max_pull_connections <= 7 { "~ HIGH" } else { "✗ MESSY" };
        report.push_str(&format!("║  Max pulls/joint               {:>8}        {:>8}        {:>10}  ║\n",
            analysis.max_pull_connections, triped.max_pull_connections, verdict_max));

        // Overpopulated joints
        let verdict_overpop = if analysis.overpopulated_joints == 0 { "✓ GOOD" } else if analysis.overpopulated_joints < 5 { "~ FEW" } else { "✗ MANY" };
        report.push_str(&format!("║  Overpopulated joints (>5)     {:>8}        {:>8}        {:>10}  ║\n",
            analysis.overpopulated_joints, triped.overpopulated_joints, verdict_overpop));

        // Underpopulated joints
        let verdict_underpop = if analysis.underpopulated_joints == 0 { "✓ GOOD" } else if analysis.underpopulated_joints < 3 { "~ FEW" } else { "✗ UNSTABLE" };
        report.push_str(&format!("║  Underpopulated joints (<3)    {:>8}        {:>8}        {:>10}  ║\n",
            analysis.underpopulated_joints, triped.underpopulated_joints, verdict_underpop));

        // Min push distance (convert to mm for display)
        let evolved_mm = analysis.min_push_distance * 1000.0;
        let triped_mm = triped.min_push_distance * 1000.0;
        let verdict_dist = if evolved_mm > 100.0 { "✓ GOOD" } else if evolved_mm > 50.0 { "~ CLOSE" } else if evolved_mm > 10.0 { "✗ NEAR-MISS" } else { "✗ CROSSING" };
        report.push_str(&format!("║  Min push distance (mm)        {:>8.1}        {:>8.1}        {:>10}  ║\n",
            evolved_mm, triped_mm, verdict_dist));

        // Crossings
        let verdict_cross = if analysis.crossing_count == 0 { "✓ GOOD" } else { "✗ UNBUILDABLE" };
        report.push_str(&format!("║  Push crossings (<10mm)        {:>8}        {:>8}        {:>10}  ║\n",
            analysis.crossing_count, triped.crossing_count, verdict_cross));

        // Near-misses
        let verdict_near = if analysis.near_miss_count == 0 { "✓ GOOD" } else if analysis.near_miss_count < 3 { "~ FEW" } else { "✗ MANY" };
        report.push_str(&format!("║  Near-misses (10-50mm)         {:>8}        {:>8}        {:>10}  ║\n",
            analysis.near_miss_count, triped.near_miss_count, verdict_near));

        report.push_str("║                                                                              ║\n");
        report.push_str("╠══════════════════════════════════════════════════════════════════════════════╣\n");

        // Overall assessment
        let total_issues = analysis.overpopulated_joints + analysis.underpopulated_joints + analysis.crossing_count + analysis.near_miss_count;
        let buildable = analysis.crossing_count == 0 && analysis.min_pull_connections >= 3;
        let clean = analysis.overpopulated_joints == 0 && analysis.max_pull_connections <= 5;

        report.push_str("║                                                                              ║\n");
        report.push_str(&format!("║  SUMMARY:                                                                    ║\n"));
        report.push_str(&format!("║    Total joints: {}    Total intervals: {}    Push count: {}              \n",
            fabric.joints.len(), fabric.intervals.len(), best.push_count));
        report.push_str(&format!("║    Buildable: {}    Clean topology: {}    Total issues: {}                \n",
            if buildable { "YES" } else { "NO" }, if clean { "YES" } else { "NO" }, total_issues));
        report.push_str("║                                                                              ║\n");
        report.push_str("╚══════════════════════════════════════════════════════════════════════════════╝\n");

        panic!("{}", report);
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
