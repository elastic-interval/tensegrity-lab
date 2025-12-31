use crate::build::evo::evolution::EvolutionConfig;
use crate::build::evo::fitness::{get_fitness_function, FitnessFunction, SuspendedJointsFitness};
use crate::build::evo::grower::MutationWeights;
use crate::units::{Meters, Seconds};

/// Builder for evolution scenarios using a fluent DSL.
///
/// # Example
/// ```ignore
/// let scenario = EvolutionScenario::new("Aggressive Growth")
///     .fitness("suspended")
///     .mutations(MutationWeights {
///         shorten_pull: 10.0,
///         lengthen_pull: 10.0,
///         remove_pull: 30.0,
///         add_push: 50.0,
///     })
///     .population(30)
///     .seed_pushes(4)
///     .settle_seed(Sec(1.0))
///     .settle_mutation(Sec(2.5))
///     .push_length(M(2.5));
/// ```
#[derive(Clone, Debug)]
pub struct EvolutionScenario {
    /// Scenario name
    pub name: String,
    /// Fitness function name
    pub fitness_name: String,
    /// Mutation weights
    pub mutation_weights: MutationWeights,
    /// Population size
    pub population_size: usize,
    /// Number of pushes in seed structure
    pub seed_push_count: usize,
    /// Seconds to settle initial seed
    pub seed_settle_seconds: f32,
    /// Seconds to settle after each mutation
    pub mutation_settle_seconds: f32,
    /// Push interval length
    pub push_length: Meters,
}

impl Default for EvolutionScenario {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            fitness_name: "suspended".to_string(),
            mutation_weights: MutationWeights::default(),
            population_size: 20,
            seed_push_count: 3,
            seed_settle_seconds: 1.5,
            mutation_settle_seconds: 3.5,
            push_length: Meters(3.0),
        }
    }
}

impl EvolutionScenario {
    /// Create a new scenario with a name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Set the fitness function by name ("suspended" or "height").
    pub fn fitness(mut self, name: impl Into<String>) -> Self {
        self.fitness_name = name.into();
        self
    }

    /// Set custom mutation weights.
    pub fn mutations(mut self, weights: MutationWeights) -> Self {
        self.mutation_weights = weights;
        self
    }

    /// Use default balanced mutations (35/35/10/20).
    pub fn balanced_mutations(self) -> Self {
        self.mutations(MutationWeights::default())
    }

    /// Use aggressive mutations favoring structural changes (10/10/30/50).
    pub fn aggressive_mutations(self) -> Self {
        self.mutations(MutationWeights {
            shorten_pull: 10.0,
            lengthen_pull: 10.0,
            remove_pull: 30.0,
            add_push: 50.0,
        })
    }

    /// Use conservative mutations favoring fine-tuning (45/45/5/5).
    pub fn conservative_mutations(self) -> Self {
        self.mutations(MutationWeights {
            shorten_pull: 45.0,
            lengthen_pull: 45.0,
            remove_pull: 5.0,
            add_push: 5.0,
        })
    }

    /// Set population size.
    pub fn population(mut self, size: usize) -> Self {
        self.population_size = size;
        self
    }

    /// Set number of pushes in the seed structure.
    pub fn seed_pushes(mut self, count: usize) -> Self {
        self.seed_push_count = count;
        self
    }

    /// Set seed settling time.
    pub fn settle_seed(mut self, seconds: Seconds) -> Self {
        self.seed_settle_seconds = seconds.0;
        self
    }

    /// Set mutation settling time.
    pub fn settle_mutation(mut self, seconds: Seconds) -> Self {
        self.mutation_settle_seconds = seconds.0;
        self
    }

    /// Set push interval length.
    pub fn push_length(mut self, length: Meters) -> Self {
        self.push_length = length;
        self
    }

    /// Convert to EvolutionConfig for use with Evolution.
    pub fn to_config(&self) -> EvolutionConfig {
        EvolutionConfig {
            name: self.name.clone(),
            population_size: self.population_size,
            seed_push_count: self.seed_push_count,
            seed_settle_seconds: self.seed_settle_seconds,
            mutation_settle_seconds: self.mutation_settle_seconds,
            push_length: self.push_length.0,
        }
    }

    /// Get the fitness function for this scenario.
    pub fn fitness_function(&self) -> Box<dyn FitnessFunction> {
        get_fitness_function(&self.fitness_name)
            .unwrap_or_else(|| Box::new(SuspendedJointsFitness::default()))
    }
}

// ============================================================================
// Convenience functions for common unit types (like fabric DSL)
// ============================================================================

/// Seconds helper for DSL.
#[allow(non_snake_case)]
pub fn Sec(seconds: f32) -> Seconds {
    Seconds(seconds)
}

/// Meters helper for DSL.
#[allow(non_snake_case)]
pub fn M(meters: f32) -> Meters {
    Meters(meters)
}

// ============================================================================
// Predefined scenarios (like FabricName)
// ============================================================================

/// Predefined evolution scenarios.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioName {
    Default,
    Aggressive,
    Conservative,
    TallTowers,
}

impl ScenarioName {
    /// Get the scenario for this name.
    pub fn scenario(self) -> EvolutionScenario {
        match self {
            ScenarioName::Default => EvolutionScenario::new("Default")
                .fitness("suspended")
                .balanced_mutations()
                .population(20)
                .seed_pushes(3)
                .settle_seed(Sec(1.5))
                .settle_mutation(Sec(3.5))
                .push_length(M(3.0)),

            ScenarioName::Aggressive => EvolutionScenario::new("Aggressive Growth")
                .fitness("suspended")
                .aggressive_mutations()
                .population(30)
                .seed_pushes(4)
                .settle_seed(Sec(1.0))
                .settle_mutation(Sec(2.5))
                .push_length(M(2.5)),

            ScenarioName::Conservative => EvolutionScenario::new("Conservative Refinement")
                .fitness("suspended")
                .conservative_mutations()
                .population(15)
                .seed_pushes(3)
                .settle_seed(Sec(2.0))
                .settle_mutation(Sec(4.5))
                .push_length(M(3.5)),

            ScenarioName::TallTowers => EvolutionScenario::new("Tall Towers")
                .fitness("height")
                .mutations(MutationWeights {
                    shorten_pull: 30.0,
                    lengthen_pull: 30.0,
                    remove_pull: 15.0,
                    add_push: 25.0,
                })
                .population(25)
                .seed_pushes(4)
                .settle_seed(Sec(1.5))
                .settle_mutation(Sec(4.0))
                .push_length(M(4.0)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_scenario() {
        let scenario = EvolutionScenario::default();
        assert_eq!(scenario.fitness_name, "suspended");
        assert_eq!(scenario.population_size, 20);
    }

    #[test]
    fn test_builder_pattern() {
        let scenario = EvolutionScenario::new("Test")
            .fitness("height")
            .aggressive_mutations()
            .population(30)
            .seed_pushes(5)
            .settle_seed(Sec(2.0))
            .settle_mutation(Sec(3.0))
            .push_length(M(4.0));

        assert_eq!(scenario.name, "Test");
        assert_eq!(scenario.fitness_name, "height");
        assert_eq!(scenario.mutation_weights.add_push, 50.0);
        assert_eq!(scenario.population_size, 30);
        assert_eq!(scenario.seed_push_count, 5);
        assert_eq!(scenario.push_length.0, 4.0);
    }

    #[test]
    fn test_predefined_scenarios() {
        let aggressive = ScenarioName::Aggressive.scenario();
        assert_eq!(aggressive.name, "Aggressive Growth");
        assert_eq!(aggressive.mutation_weights.add_push, 50.0);

        let tall = ScenarioName::TallTowers.scenario();
        assert_eq!(tall.fitness_name, "height");
    }
}
