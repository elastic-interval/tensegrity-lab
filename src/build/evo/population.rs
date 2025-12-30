use crate::fabric::Fabric;
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;

/// An individual in the population - stores actual fabric state.
#[derive(Clone)]
pub struct Individual {
    /// The actual fabric with joint positions and intervals
    pub fabric: Fabric,
    /// Fitness score (higher is better)
    pub fitness: f32,
    /// Height of the structure (cached to avoid recalculation)
    pub height: f32,
    /// Number of push intervals (for cost calculation)
    pub push_count: usize,
    /// Generation when this individual was created
    pub generation: usize,
    /// Number of mutations in this individual's lineage
    pub mutations: usize,
}

impl Individual {
    /// Create a new individual from a fabric.
    pub fn new(fabric: Fabric, fitness: f32, height: f32, push_count: usize, generation: usize) -> Self {
        Self {
            fabric,
            fitness,
            height,
            push_count,
            generation,
            mutations: 0,
        }
    }

    /// Create offspring with incremented mutation count.
    pub fn offspring(fabric: Fabric, fitness: f32, height: f32, push_count: usize, generation: usize, parent_mutations: usize) -> Self {
        Self {
            fabric,
            fitness,
            height,
            push_count,
            generation,
            mutations: parent_mutations + 1,
        }
    }
}

/// A live population of individuals competing for survival.
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
    pub fn add_initial(&mut self, fabric: Fabric, fitness: f32, height: f32, push_count: usize) {
        if self.pool.len() < self.capacity {
            let individual = Individual::new(fabric.clone(), fitness, height, push_count, 0);
            self.update_best(&individual);
            self.pool.push(individual);
        }
    }

    /// Pick an individual as a parent for mutation, favoring fitter individuals.
    /// Uses rank-based selection: best has weight N, worst has weight 1.
    /// This allows escaping local maxima while still favoring the fittest.
    pub fn pick_random(&mut self) -> Option<&Individual> {
        if self.pool.is_empty() {
            return None;
        }

        // Get indices sorted by fitness (best first)
        let mut indices: Vec<usize> = (0..self.pool.len()).collect();
        indices.sort_by(|&a, &b| {
            self.pool[b].fitness
                .partial_cmp(&self.pool[a].fitness)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Rank-based weights: best gets N, worst gets 1
        // Total weight = N + (N-1) + ... + 1 = N*(N+1)/2
        let n = self.pool.len();
        let total_weight = n * (n + 1) / 2;

        // Pick a random weight
        let target = self.rng.random_range(0..total_weight);

        // Find which rank this falls into
        let mut cumulative = 0;
        for (rank, &idx) in indices.iter().enumerate() {
            let weight = n - rank; // Best has weight N, worst has weight 1
            cumulative += weight;
            if target < cumulative {
                return Some(&self.pool[idx]);
            }
        }

        // Fallback (shouldn't reach here)
        Some(&self.pool[indices[0]])
    }

    /// Pick a random parent and return a clone of its fabric for mutation.
    pub fn pick_parent_fabric(&mut self) -> Option<Fabric> {
        self.pick_random().map(|ind| ind.fabric.clone())
    }

    /// Get the push count of the picked parent (call after pick_parent_fabric).
    pub fn pick_parent_push_count(&mut self) -> Option<usize> {
        // Pick same index as last pick_random would have
        if self.pool.is_empty() {
            return None;
        }
        let idx = self.rng.random_range(0..self.pool.len());
        Some(self.pool[idx].push_count)
    }

    /// Try to insert an offspring into the population.
    pub fn try_insert(&mut self, fabric: Fabric, fitness: f32, height: f32, push_count: usize, parent_mutations: usize) -> bool {
        let individual = Individual::offspring(fabric, fitness, height, push_count, self.generation, parent_mutations);

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

    /// Get access to the pool of individuals (for testing/inspection).
    pub fn pool(&self) -> &[Individual] {
        &self.pool
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

        let avg_pushes: f32 = self.pool.iter().map(|i| i.push_count as f32).sum::<f32>()
            / self.pool.len() as f32;

        PopulationStats {
            size: self.pool.len(),
            generation: self.generation,
            min_fitness: min,
            max_fitness: max,
            mean_fitness: mean,
            std_dev,
            avg_push_count: avg_pushes,
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
    /// Average number of push intervals
    pub avg_push_count: f32,
}
