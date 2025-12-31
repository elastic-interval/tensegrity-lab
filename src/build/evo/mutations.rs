use crate::build::evo::grower::{GrowthConfig, Grower, MutationWeights};
use crate::build::evo::population::MutationType;
use crate::fabric::Fabric;
use crate::units::Meters;
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;

/// Result of applying a mutation.
pub struct MutationResult {
    /// The type of mutation applied
    pub mutation_type: MutationType,
    /// The new push count after mutation
    pub push_count: usize,
    /// Suggested settling time in seconds
    pub settle_seconds: f32,
}

/// Trait for mutation strategies.
///
/// Different strategies can favor different types of mutations:
/// - Default: balanced exploration
/// - Aggressive: more structural changes (add/remove)
/// - Conservative: more fine-tuning (shorten/lengthen)
pub trait MutationStrategy: Send + Sync {
    /// Name of this mutation strategy (for display and config).
    fn name(&self) -> &'static str;

    /// Apply a mutation to the fabric.
    ///
    /// The height parameter indicates if the structure is flat (< 0.1) which may
    /// trigger special handling.
    fn apply_mutation(
        &mut self,
        fabric: &mut Fabric,
        push_count: usize,
        height: f32,
    ) -> MutationResult;
}

/// Get a mutation strategy by name.
pub fn get_mutation_strategy(name: &str, seed: u64, push_length: Meters) -> Option<Box<dyn MutationStrategy>> {
    match name {
        "default" => Some(Box::new(DefaultMutationStrategy::new(seed, push_length))),
        "aggressive" => Some(Box::new(AggressiveMutationStrategy::new(seed, push_length))),
        "conservative" => Some(Box::new(ConservativeMutationStrategy::new(seed, push_length))),
        _ => None,
    }
}

/// List available mutation strategy names.
pub fn available_mutation_strategies() -> Vec<&'static str> {
    vec!["default", "aggressive", "conservative"]
}

// ============================================================================
// DefaultMutationStrategy - balanced exploration
// ============================================================================

/// Default mutation strategy with balanced weights.
///
/// Weights: shorten=35, lengthen=35, remove=10, add=20
pub struct DefaultMutationStrategy {
    rng: ChaCha8Rng,
    grower: Grower,
    flat_perturbation: f32,
}

impl DefaultMutationStrategy {
    pub fn new(seed: u64, push_length: Meters) -> Self {
        let config = GrowthConfig {
            push_length,
            mutation_weights: MutationWeights {
                shorten_pull: 35.0,
                lengthen_pull: 35.0,
                remove_pull: 10.0,
                add_push: 20.0,
                split_pull: 0.0,
            },
            ..Default::default()
        };
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed),
            grower: Grower::new(seed.wrapping_add(1), config),
            flat_perturbation: 0.05, // 5cm random nudges
        }
    }

    fn apply_flat_mutation(&mut self, fabric: &mut Fabric, push_count: usize) -> MutationResult {
        // Try removing a pull to let structure unfold
        let mutation_type = if self.grower.remove_random_pull(fabric) {
            MutationType::FlatRemovePull
        } else {
            self.grower.add_more_connections(fabric);
            MutationType::FlatAddConnections
        };

        // Lift flat structure above floor
        let lift_altitude = 0.2;
        let translation = fabric.centralize_translation(Some(lift_altitude));
        fabric.apply_translation(translation);

        // Add perturbations to help it snap open
        for joint in fabric.joints.values_mut() {
            joint.location.x += self.rng.random_range(-self.flat_perturbation..self.flat_perturbation);
            joint.location.y += self.rng.random_range(-self.flat_perturbation..self.flat_perturbation);
            joint.location.z += self.rng.random_range(-self.flat_perturbation..self.flat_perturbation);
        }
        fabric.zero_velocities();

        MutationResult {
            mutation_type,
            push_count,
            settle_seconds: 3.5, // Default settle time for flat structures
        }
    }

    fn apply_normal_mutation(&mut self, fabric: &mut Fabric, push_count: usize) -> MutationResult {
        let (new_count, mutation_type) = self.grower.apply_random_mutation(fabric, push_count);

        // Lift structure slightly so frozen joints unstick from floor
        let lift_altitude = 0.1;
        let translation = fabric.centralize_translation(Some(lift_altitude));
        fabric.apply_translation(translation);
        fabric.zero_velocities();

        let settle_seconds = match mutation_type {
            MutationType::ShortenPull | MutationType::LengthenPull => 4.0,
            MutationType::AddPush | MutationType::RemovePull => 6.0,
            _ => 3.5,
        };

        MutationResult {
            mutation_type,
            push_count: new_count,
            settle_seconds,
        }
    }
}

impl MutationStrategy for DefaultMutationStrategy {
    fn name(&self) -> &'static str {
        "default"
    }

    fn apply_mutation(
        &mut self,
        fabric: &mut Fabric,
        push_count: usize,
        height: f32,
    ) -> MutationResult {
        if height < 0.1 {
            self.apply_flat_mutation(fabric, push_count)
        } else {
            self.apply_normal_mutation(fabric, push_count)
        }
    }
}

// ============================================================================
// AggressiveMutationStrategy - more structural changes
// ============================================================================

/// Aggressive mutation strategy favoring structural changes.
///
/// Weights: shorten=10, lengthen=10, remove=30, add=50
/// More likely to add pushes and remove pulls, exploring structure space faster.
pub struct AggressiveMutationStrategy {
    rng: ChaCha8Rng,
    grower: Grower,
    flat_perturbation: f32,
}

impl AggressiveMutationStrategy {
    pub fn new(seed: u64, push_length: Meters) -> Self {
        let config = GrowthConfig {
            push_length,
            mutation_weights: MutationWeights {
                shorten_pull: 10.0,
                lengthen_pull: 10.0,
                remove_pull: 30.0,
                add_push: 50.0,
                split_pull: 0.0,
            },
            ..Default::default()
        };
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed),
            grower: Grower::new(seed.wrapping_add(1), config),
            flat_perturbation: 0.08, // Larger perturbations for aggressive strategy
        }
    }

    fn apply_flat_mutation(&mut self, fabric: &mut Fabric, push_count: usize) -> MutationResult {
        // Aggressive: always try to add a push to flat structures
        let new_count = self.grower.mutate(fabric, push_count);
        let mutation_type = if new_count > push_count {
            MutationType::AddPush
        } else {
            // If we couldn't add, try removing pull
            if self.grower.remove_random_pull(fabric) {
                MutationType::FlatRemovePull
            } else {
                self.grower.add_more_connections(fabric);
                MutationType::FlatAddConnections
            }
        };

        let lift_altitude = 0.3; // Higher lift for aggressive
        let translation = fabric.centralize_translation(Some(lift_altitude));
        fabric.apply_translation(translation);

        for joint in fabric.joints.values_mut() {
            joint.location.x += self.rng.random_range(-self.flat_perturbation..self.flat_perturbation);
            joint.location.y += self.rng.random_range(-self.flat_perturbation..self.flat_perturbation);
            joint.location.z += self.rng.random_range(-self.flat_perturbation..self.flat_perturbation);
        }
        fabric.zero_velocities();

        MutationResult {
            mutation_type,
            push_count: new_count,
            settle_seconds: 2.5, // Faster settling for aggressive
        }
    }

    fn apply_normal_mutation(&mut self, fabric: &mut Fabric, push_count: usize) -> MutationResult {
        let (new_count, mutation_type) = self.grower.apply_random_mutation(fabric, push_count);

        let lift_altitude = 0.15; // Slightly higher lift
        let translation = fabric.centralize_translation(Some(lift_altitude));
        fabric.apply_translation(translation);
        fabric.zero_velocities();

        let settle_seconds = match mutation_type {
            MutationType::ShortenPull | MutationType::LengthenPull => 3.0,
            MutationType::AddPush | MutationType::RemovePull => 5.0,
            _ => 2.5,
        };

        MutationResult {
            mutation_type,
            push_count: new_count,
            settle_seconds,
        }
    }
}

impl MutationStrategy for AggressiveMutationStrategy {
    fn name(&self) -> &'static str {
        "aggressive"
    }

    fn apply_mutation(
        &mut self,
        fabric: &mut Fabric,
        push_count: usize,
        height: f32,
    ) -> MutationResult {
        if height < 0.1 {
            self.apply_flat_mutation(fabric, push_count)
        } else {
            self.apply_normal_mutation(fabric, push_count)
        }
    }
}

// ============================================================================
// ConservativeMutationStrategy - more fine-tuning
// ============================================================================

/// Conservative mutation strategy favoring fine-tuning.
///
/// Weights: shorten=45, lengthen=45, remove=5, add=5
/// More likely to adjust existing pulls, refining structure rather than changing it.
pub struct ConservativeMutationStrategy {
    rng: ChaCha8Rng,
    grower: Grower,
    flat_perturbation: f32,
}

impl ConservativeMutationStrategy {
    pub fn new(seed: u64, push_length: Meters) -> Self {
        let config = GrowthConfig {
            push_length,
            mutation_weights: MutationWeights {
                shorten_pull: 45.0,
                lengthen_pull: 45.0,
                remove_pull: 5.0,
                add_push: 5.0,
                split_pull: 0.0,
            },
            ..Default::default()
        };
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed),
            grower: Grower::new(seed.wrapping_add(1), config),
            flat_perturbation: 0.03, // Smaller perturbations for conservative
        }
    }

    fn apply_flat_mutation(&mut self, fabric: &mut Fabric, push_count: usize) -> MutationResult {
        // Conservative: try adding connections before removing pulls
        let mutation_type = if self.grower.add_more_connections(fabric) {
            MutationType::FlatAddConnections
        } else if self.grower.remove_random_pull(fabric) {
            MutationType::FlatRemovePull
        } else {
            MutationType::FlatAddConnections // Fallback
        };

        let lift_altitude = 0.15;
        let translation = fabric.centralize_translation(Some(lift_altitude));
        fabric.apply_translation(translation);

        for joint in fabric.joints.values_mut() {
            joint.location.x += self.rng.random_range(-self.flat_perturbation..self.flat_perturbation);
            joint.location.y += self.rng.random_range(-self.flat_perturbation..self.flat_perturbation);
            joint.location.z += self.rng.random_range(-self.flat_perturbation..self.flat_perturbation);
        }
        fabric.zero_velocities();

        MutationResult {
            mutation_type,
            push_count,
            settle_seconds: 4.0, // Longer settling for conservative
        }
    }

    fn apply_normal_mutation(&mut self, fabric: &mut Fabric, push_count: usize) -> MutationResult {
        let (new_count, mutation_type) = self.grower.apply_random_mutation(fabric, push_count);

        let lift_altitude = 0.08; // Smaller lift
        let translation = fabric.centralize_translation(Some(lift_altitude));
        fabric.apply_translation(translation);
        fabric.zero_velocities();

        let settle_seconds = match mutation_type {
            MutationType::ShortenPull | MutationType::LengthenPull => 5.0,
            MutationType::AddPush | MutationType::RemovePull => 7.0,
            _ => 4.0,
        };

        MutationResult {
            mutation_type,
            push_count: new_count,
            settle_seconds,
        }
    }
}

impl MutationStrategy for ConservativeMutationStrategy {
    fn name(&self) -> &'static str {
        "conservative"
    }

    fn apply_mutation(
        &mut self,
        fabric: &mut Fabric,
        push_count: usize,
        height: f32,
    ) -> MutationResult {
        if height < 0.1 {
            self.apply_flat_mutation(fabric, push_count)
        } else {
            self.apply_normal_mutation(fabric, push_count)
        }
    }
}
