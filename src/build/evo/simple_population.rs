/*
 * Simple Population Strategy
 *
 * A straightforward generational strategy:
 * - Keep best N individuals (elitism)
 * - Fill rest with mutations of survivors
 * - "Demise of the least fit" - worst individuals don't reproduce
 */

use crate::build::evo::traits::{Genome, PopulationStats, PopulationStrategy};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// An individual in the population with its fitness score.
#[derive(Clone)]
struct Individual<G: Genome> {
    genome: G,
    fitness: f32,
}

/// Simple generational population with elitism.
pub struct SimplePopulation<G: Genome> {
    /// Current population.
    individuals: Vec<Individual<G>>,

    /// Genomes waiting to be evaluated this generation.
    pending: Vec<G>,

    /// Results from current generation.
    results: Vec<(G, f32)>,

    /// Target population size.
    population_size: usize,

    /// Number of best individuals to keep each generation.
    elite_count: usize,

    /// Current generation number.
    generation: usize,

    /// Maximum generations before termination.
    max_generations: Option<usize>,

    /// Fitness threshold for early termination.
    fitness_threshold: Option<f32>,

    /// Random number generator.
    rng: ChaCha8Rng,

    /// Total trials completed.
    trials_completed: usize,
}

impl<G: Genome> SimplePopulation<G> {
    /// Create a new simple population strategy.
    pub fn new(population_size: usize, elite_count: usize, seed: u64) -> Self {
        Self {
            individuals: vec![],
            pending: vec![],
            results: vec![],
            population_size,
            elite_count: elite_count.min(population_size),
            generation: 0,
            max_generations: None,
            fitness_threshold: None,
            rng: ChaCha8Rng::seed_from_u64(seed),
            trials_completed: 0,
        }
    }

    /// Set maximum generations before termination.
    pub fn with_max_generations(mut self, max: usize) -> Self {
        self.max_generations = Some(max);
        self
    }

    /// Set fitness threshold for early termination.
    pub fn with_fitness_threshold(mut self, threshold: f32) -> Self {
        self.fitness_threshold = Some(threshold);
        self
    }

    fn best_fitness(&self) -> f32 {
        self.individuals
            .iter()
            .map(|i| i.fitness)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0)
    }

    fn mean_fitness(&self) -> f32 {
        if self.individuals.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.individuals.iter().map(|i| i.fitness).sum();
        sum / self.individuals.len() as f32
    }

    fn worst_fitness(&self) -> f32 {
        self.individuals
            .iter()
            .map(|i| i.fitness)
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0)
    }
}

impl<G: Genome> PopulationStrategy<G> for SimplePopulation<G> {
    fn initialize(&mut self, seed_genomes: Vec<G>) {
        self.generation = 0;
        self.individuals.clear();
        self.results.clear();
        self.trials_completed = 0;

        // Use seed genomes as initial population
        // If we have fewer than population_size, generate mutations
        let mut initial = seed_genomes;

        // Generate more individuals if needed
        while initial.len() < self.population_size {
            if initial.is_empty() {
                break;
            }
            // Pick a random individual and mutate
            let idx = self.rng.gen_range(0..initial.len());
            let variants = initial[idx].adjacent_possible(&mut self.rng);
            if let Some(variant) = variants.into_iter().next() {
                initial.push(variant);
            } else {
                // Can't mutate, just clone
                initial.push(initial[idx].clone());
            }
        }

        // Set up for evaluation
        self.pending = initial;
    }

    fn next_for_trial(&mut self) -> Option<G> {
        self.pending.pop()
    }

    fn record_result(&mut self, genome: G, fitness: f32) {
        self.results.push((genome, fitness));
        self.trials_completed += 1;
    }

    fn advance_generation(&mut self) {
        // Convert results to individuals
        let mut new_individuals: Vec<Individual<G>> = self
            .results
            .drain(..)
            .map(|(genome, fitness)| Individual { genome, fitness })
            .collect();

        // Sort by fitness (highest first)
        new_individuals.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap());

        // Keep elites
        let elites: Vec<Individual<G>> = new_individuals
            .iter()
            .take(self.elite_count)
            .cloned()
            .collect();

        // Store current population (elites only for now)
        self.individuals = elites.clone();

        // Generate next generation from elites
        let mut next_gen: Vec<G> = elites.iter().map(|i| i.genome.clone()).collect();

        // Fill rest with mutations
        while next_gen.len() < self.population_size {
            if self.individuals.is_empty() {
                break;
            }

            // Select parent (fitness-proportional would be better, but keep it simple)
            let idx = self.rng.gen_range(0..self.individuals.len());
            let parent = &self.individuals[idx].genome;

            // Generate mutations
            let variants = parent.adjacent_possible(&mut self.rng);
            if let Some(variant) = variants.into_iter().next() {
                next_gen.push(variant);
            } else {
                // Can't mutate, just clone
                next_gen.push(parent.clone());
            }
        }

        self.pending = next_gen;
        self.generation += 1;
    }

    fn best(&self) -> Option<&G> {
        self.individuals.first().map(|i| &i.genome)
    }

    fn stats(&self) -> PopulationStats {
        PopulationStats {
            generation: self.generation,
            population_size: self.individuals.len(),
            trials_completed: self.trials_completed,
            best_fitness: self.best_fitness(),
            mean_fitness: self.mean_fitness(),
            worst_fitness: self.worst_fitness(),
        }
    }

    fn should_terminate(&self) -> bool {
        // Check max generations
        if let Some(max) = self.max_generations {
            if self.generation >= max {
                return true;
            }
        }

        // Check fitness threshold
        if let Some(threshold) = self.fitness_threshold {
            if self.best_fitness() >= threshold {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::evo::traits::GenomeId;

    #[derive(Clone, Debug)]
    struct TestGenome {
        id: GenomeId,
        value: f32,
    }

    impl Genome for TestGenome {
        fn id(&self) -> GenomeId {
            self.id
        }

        fn express(
            &self,
            _context: &crate::build::evo::traits::ExpressionContext,
        ) -> crate::fabric::Fabric {
            crate::fabric::Fabric::new("test".to_string())
        }

        fn adjacent_possible(&self, rng: &mut impl rand::Rng) -> Vec<Self> {
            vec![TestGenome {
                id: GenomeId::new(),
                value: self.value + rng.gen_range(-0.1..0.1),
            }]
        }

        fn describe(&self) -> String {
            format!("TestGenome({})", self.value)
        }
    }

    #[test]
    fn test_simple_population_initialization() {
        let mut pop: SimplePopulation<TestGenome> = SimplePopulation::new(10, 2, 42);

        let seeds = vec![TestGenome {
            id: GenomeId::new(),
            value: 0.5,
        }];

        pop.initialize(seeds);

        // Should have expanded to population_size pending evaluations
        let mut count = 0;
        while pop.next_for_trial().is_some() {
            count += 1;
        }
        assert_eq!(count, 10);
    }

    #[test]
    fn test_simple_population_advances() {
        let mut pop: SimplePopulation<TestGenome> = SimplePopulation::new(5, 2, 42);

        let seeds = vec![TestGenome {
            id: GenomeId::new(),
            value: 0.5,
        }];

        pop.initialize(seeds);

        // Evaluate all individuals
        while let Some(genome) = pop.next_for_trial() {
            pop.record_result(genome.clone(), genome.value);
        }

        // Advance generation
        pop.advance_generation();

        assert_eq!(pop.stats().generation, 1);
    }
}
