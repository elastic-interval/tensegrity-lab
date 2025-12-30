use crate::build::evo::genome::Genome;
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;

/// An individual in the population with its genome and fitness.
#[derive(Clone, Debug)]
pub struct Individual {
    /// The genome encoding this individual's structure
    pub genome: Genome,
    /// Fitness score (higher is better)
    pub fitness: f32,
    /// Generation when this individual was created
    pub generation: usize,
}

impl Individual {
    /// Create a new individual.
    pub fn new(genome: Genome, fitness: f32, generation: usize) -> Self {
        Self {
            genome,
            fitness,
            generation,
        }
    }
}

/// A live population of individuals competing for survival.
///
/// Implements steady-state evolution where offspring compete against
/// the current population for survival.
pub struct Population {
    /// RNG for random selection
    rng: ChaCha8Rng,
    /// The live population pool
    pool: Vec<Individual>,
    /// Maximum population size
    capacity: usize,
    /// Current generation counter
    generation: usize,
    /// Best individual ever seen
    best_ever: Option<Individual>,
}

impl Population {
    /// Create a new empty population with the given seed and capacity.
    pub fn new(seed: u64, capacity: usize) -> Self {
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed),
            pool: Vec::with_capacity(capacity),
            capacity,
            generation: 0,
            best_ever: None,
        }
    }

    /// Add an initial individual to the population.
    ///
    /// This is used during population seeding, before evolution begins.
    pub fn add_initial(&mut self, genome: Genome, fitness: f32) {
        if self.pool.len() < self.capacity {
            let individual = Individual::new(genome.clone(), fitness, 0);
            self.update_best(&individual);
            self.pool.push(individual);
        }
    }

    /// Pick a random individual as a parent for mutation.
    ///
    /// Returns None if the population is empty.
    pub fn pick_random(&mut self) -> Option<&Individual> {
        if self.pool.is_empty() {
            return None;
        }
        let idx = self.rng.random_range(0..self.pool.len());
        Some(&self.pool[idx])
    }

    /// Pick a random parent and return a clone of its genome for mutation.
    ///
    /// Returns None if the population is empty.
    pub fn pick_parent_genome(&mut self) -> Option<Genome> {
        self.pick_random().map(|ind| ind.genome.clone())
    }

    /// Try to insert an offspring into the population.
    ///
    /// The offspring competes against the worst individual:
    /// - If better, replaces the worst
    /// - If worse or equal, is discarded
    ///
    /// Returns true if the offspring was inserted.
    pub fn try_insert(&mut self, genome: Genome, fitness: f32) -> bool {
        let individual = Individual::new(genome, fitness, self.generation);

        // Update best-ever tracking
        self.update_best(&individual);

        // If population isn't full, always insert
        if self.pool.len() < self.capacity {
            self.pool.push(individual);
            return true;
        }

        // Find the worst individual
        let worst_idx = self.find_worst_index();
        let worst_fitness = self.pool[worst_idx].fitness;

        // Replace if strictly better
        if fitness > worst_fitness {
            self.pool[worst_idx] = individual;
            return true;
        }

        false
    }

    /// Increment the generation counter.
    pub fn next_generation(&mut self) {
        self.generation += 1;
    }

    /// Get the current generation.
    pub fn generation(&self) -> usize {
        self.generation
    }

    /// Get the current population size.
    pub fn size(&self) -> usize {
        self.pool.len()
    }

    /// Get the population capacity.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Check if the population is at capacity.
    pub fn is_full(&self) -> bool {
        self.pool.len() >= self.capacity
    }

    /// Get the best individual currently in the population.
    pub fn best_current(&self) -> Option<&Individual> {
        self.pool.iter().max_by(|a, b| {
            a.fitness
                .partial_cmp(&b.fitness)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Get the best individual ever seen.
    pub fn best_ever(&self) -> Option<&Individual> {
        self.best_ever.as_ref()
    }

    /// Get the worst individual currently in the population.
    pub fn worst_current(&self) -> Option<&Individual> {
        if self.pool.is_empty() {
            return None;
        }
        let idx = self.find_worst_index();
        Some(&self.pool[idx])
    }

    /// Calculate population statistics.
    pub fn stats(&self) -> PopulationStats {
        if self.pool.is_empty() {
            return PopulationStats::default();
        }

        let fitnesses: Vec<f32> = self.pool.iter().map(|i| i.fitness).collect();
        let sum: f32 = fitnesses.iter().sum();
        let mean = sum / fitnesses.len() as f32;

        let min = fitnesses.iter().fold(f32::MAX, |a, &b| a.min(b));
        let max = fitnesses.iter().fold(f32::MIN, |a, &b| a.max(b));

        let variance: f32 = fitnesses.iter().map(|f| (f - mean).powi(2)).sum::<f32>()
            / fitnesses.len() as f32;
        let std_dev = variance.sqrt();

        PopulationStats {
            size: self.pool.len(),
            generation: self.generation,
            min_fitness: min,
            max_fitness: max,
            mean_fitness: mean,
            std_dev,
        }
    }

    /// Get an iterator over all individuals.
    pub fn iter(&self) -> impl Iterator<Item = &Individual> {
        self.pool.iter()
    }

    /// Find the index of the worst individual.
    fn find_worst_index(&self) -> usize {
        self.pool
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                a.fitness
                    .partial_cmp(&b.fitness)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(idx, _)| idx)
            .unwrap_or(0)
    }

    /// Update the best-ever tracking.
    fn update_best(&mut self, individual: &Individual) {
        let dominated = match &self.best_ever {
            None => true,
            Some(best) => individual.fitness > best.fitness,
        };

        if dominated {
            self.best_ever = Some(individual.clone());
        }
    }
}

/// Statistics about the population.
#[derive(Debug, Clone, Default)]
pub struct PopulationStats {
    /// Current population size
    pub size: usize,
    /// Current generation
    pub generation: usize,
    /// Minimum fitness in population
    pub min_fitness: f32,
    /// Maximum fitness in population
    pub max_fitness: f32,
    /// Mean fitness
    pub mean_fitness: f32,
    /// Standard deviation of fitness
    pub std_dev: f32,
}
