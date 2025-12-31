use crate::build::evo::evolution::EvolutionConfig;
use crate::build::evo::fitness::FitnessName;
use crate::build::evo::grower::MutationWeights;
use crate::units::{Meters, Seconds};

/// Builder for evolution scenarios using a fluent DSL.
///
/// # Example
/// ```ignore
/// let scenario = EvolutionScenario::new("Aggressive Growth")
///     .fitness(FitnessName::Suspended)
///     .mutations(MutationWeights {
///         shorten_pull: 10.0,
///         lengthen_pull: 10.0,
///         remove_pull: 30.0,
///         add_push: 50.0,
///         split_pull: 0.0,
///     })
///     .population(30)
///     .seed_pushes(4)
///     .settle_seed(Sec(1.0))
///     .settle_mutation(Sec(2.5))
///     .push_length(M(2.5));
/// ```
#[derive(Clone, Debug)]
pub struct EvolutionScenario {
    pub name: String,
    pub fitness: FitnessName,
    pub mutation_weights: MutationWeights,
    pub population_size: usize,
    pub seed_push_count: usize,
    pub seed_settle_seconds: f32,
    pub mutation_settle_seconds: f32,
    pub push_length: Meters,
}

impl Default for EvolutionScenario {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            fitness: FitnessName::default(),
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

    /// Set the fitness function.
    pub fn fitness(mut self, fitness: FitnessName) -> Self {
        self.fitness = fitness;
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

    /// Use aggressive mutations favoring structural changes (10/10/30/50/0).
    pub fn aggressive_mutations(self) -> Self {
        self.mutations(MutationWeights {
            shorten_pull: 10.0,
            lengthen_pull: 10.0,
            remove_pull: 30.0,
            add_push: 50.0,
            split_pull: 0.0,
        })
    }

    /// Use conservative mutations favoring fine-tuning (45/45/5/5/0).
    pub fn conservative_mutations(self) -> Self {
        self.mutations(MutationWeights {
            shorten_pull: 45.0,
            lengthen_pull: 45.0,
            remove_pull: 5.0,
            add_push: 5.0,
            split_pull: 0.0,
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
            mutation_weights: self.mutation_weights.clone(),
            fitness: self.fitness,
        }
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
    Buildable,
}

impl ScenarioName {
    /// Get the scenario for this name.
    pub fn scenario(self) -> EvolutionScenario {
        match self {
            ScenarioName::Default => EvolutionScenario::new("Default")
                .fitness(FitnessName::Suspended)
                .mutations(MutationWeights {
                    shorten_pull: 35.0,
                    lengthen_pull: 35.0,
                    remove_pull: 10.0,
                    add_push: 20.0,
                    split_pull: 0.0,
                })
                .population(20)
                .seed_pushes(3)
                .settle_seed(Sec(1.5))
                .settle_mutation(Sec(3.5))
                .push_length(M(3.0)),

            ScenarioName::Aggressive => EvolutionScenario::new("Aggressive Growth")
                .fitness(FitnessName::Suspended)
                .mutations(MutationWeights {
                    shorten_pull: 10.0,
                    lengthen_pull: 10.0,
                    remove_pull: 30.0,
                    add_push: 50.0,
                    split_pull: 0.0,
                })
                .population(30)
                .seed_pushes(4)
                .settle_seed(Sec(1.0))
                .settle_mutation(Sec(2.5))
                .push_length(M(2.5)),

            ScenarioName::Conservative => EvolutionScenario::new("Conservative Refinement")
                .fitness(FitnessName::Suspended)
                .mutations(MutationWeights {
                    shorten_pull: 45.0,
                    lengthen_pull: 45.0,
                    remove_pull: 5.0,
                    add_push: 5.0,
                    split_pull: 0.0,
                })
                .population(15)
                .seed_pushes(3)
                .settle_seed(Sec(2.0))
                .settle_mutation(Sec(4.5))
                .push_length(M(3.5)),

            ScenarioName::TallTowers => EvolutionScenario::new("Tall Towers")
                .fitness(FitnessName::Height)
                .mutations(MutationWeights {
                    shorten_pull: 30.0,
                    lengthen_pull: 30.0,
                    remove_pull: 15.0,
                    add_push: 25.0,
                    split_pull: 0.0,
                })
                .population(25)
                .seed_pushes(4)
                .settle_seed(Sec(1.5))
                .settle_mutation(Sec(4.0))
                .push_length(M(4.0)),

            ScenarioName::Buildable => EvolutionScenario::new("Buildable")
                .fitness(FitnessName::Buildable)
                .mutations(MutationWeights {
                    shorten_pull: 20.0,
                    lengthen_pull: 20.0,
                    remove_pull: 10.0,
                    add_push: 0.0,       // Disabled - causes crossings
                    split_pull: 50.0,    // Main growth mechanism
                })
                .population(50)
                .seed_pushes(4)
                .settle_seed(Sec(1.5))
                .settle_mutation(Sec(3.5))
                .push_length(M(3.0)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_scenario() {
        let scenario = EvolutionScenario::default();
        assert_eq!(scenario.fitness, FitnessName::Suspended);
        assert_eq!(scenario.population_size, 20);
    }

    #[test]
    fn test_builder_pattern() {
        let scenario = EvolutionScenario::new("Test")
            .fitness(FitnessName::Height)
            .aggressive_mutations()
            .population(30)
            .seed_pushes(5)
            .settle_seed(Sec(2.0))
            .settle_mutation(Sec(3.0))
            .push_length(M(4.0));

        assert_eq!(scenario.name, "Test");
        assert_eq!(scenario.fitness, FitnessName::Height);
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
        assert_eq!(tall.fitness, FitnessName::Height);
    }
}
