//! Mutation operators for walking genomes.
//!
//! Provides mutations for both structural and actuation genomes,
//! with constraints to prevent invalid configurations.

use rand::Rng;
use rand_chacha::ChaCha8Rng;

use super::genome::{
    ActuationGenome, ActuationPattern, Branch, BrickType, GenomeConstraints, StructuralGenome,
    WalkingGenome,
};
use crate::build::dsl::animate_phase::Waveform;

/// Types of structural mutations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructuralMutation {
    AddBranch,
    RemoveBranch,
    AdjustColumnUp,
    AdjustColumnDown,
    ChangeBrickType,
    AdjustScaleUp,
    AdjustScaleDown,
    ChangeSeedBrick,
}

/// Types of actuation mutations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActuationMutation {
    ChangePattern,
    AdjustPeriodUp,
    AdjustPeriodDown,
    AdjustAmplitudeUp,
    AdjustAmplitudeDown,
    AdjustStiffnessUp,
    AdjustStiffnessDown,
    ChangeWaveform,
    AdjustPatternParam,
}

/// Combined mutation type for logging.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MutationType {
    Structural(StructuralMutation),
    Actuation(ActuationMutation),
}

/// Configuration for mutation weights.
#[derive(Clone, Debug)]
pub struct MutationWeights {
    /// Probability of structural vs actuation mutation (0.0-1.0)
    pub structural_probability: f32,
    /// Weights for structural mutations
    pub structural: StructuralWeights,
    /// Weights for actuation mutations
    pub actuation: ActuationWeights,
}

impl Default for MutationWeights {
    fn default() -> Self {
        Self {
            structural_probability: 0.6, // 60% structural, 40% actuation
            structural: StructuralWeights::default(),
            actuation: ActuationWeights::default(),
        }
    }
}

/// Weights for structural mutation types.
#[derive(Clone, Debug)]
pub struct StructuralWeights {
    pub add_branch: f32,
    pub remove_branch: f32,
    pub adjust_column: f32,
    pub change_brick_type: f32,
    pub adjust_scale: f32,
    pub change_seed: f32,
}

impl Default for StructuralWeights {
    fn default() -> Self {
        Self {
            add_branch: 15.0,
            remove_branch: 10.0,
            adjust_column: 30.0,
            change_brick_type: 20.0,
            adjust_scale: 20.0,
            change_seed: 5.0,
        }
    }
}

impl StructuralWeights {
    fn total(&self) -> f32 {
        self.add_branch
            + self.remove_branch
            + self.adjust_column
            + self.change_brick_type
            + self.adjust_scale
            + self.change_seed
    }
}

/// Weights for actuation mutation types.
#[derive(Clone, Debug)]
pub struct ActuationWeights {
    pub change_pattern: f32,
    pub adjust_period: f32,
    pub adjust_amplitude: f32,
    pub adjust_stiffness: f32,
    pub change_waveform: f32,
    pub adjust_pattern_param: f32,
}

impl Default for ActuationWeights {
    fn default() -> Self {
        Self {
            change_pattern: 10.0,
            adjust_period: 25.0,
            adjust_amplitude: 25.0,
            adjust_stiffness: 20.0,
            change_waveform: 5.0,
            adjust_pattern_param: 15.0,
        }
    }
}

impl ActuationWeights {
    fn total(&self) -> f32 {
        self.change_pattern
            + self.adjust_period
            + self.adjust_amplitude
            + self.adjust_stiffness
            + self.change_waveform
            + self.adjust_pattern_param
    }
}

/// Mutator for walking genomes.
pub struct WalkingMutator {
    weights: MutationWeights,
    constraints: GenomeConstraints,
}

impl WalkingMutator {
    pub fn new(weights: MutationWeights, constraints: GenomeConstraints) -> Self {
        Self {
            weights,
            constraints,
        }
    }

    /// Apply a random mutation to the genome.
    /// Returns the type of mutation applied.
    pub fn mutate(&self, genome: &mut WalkingGenome, rng: &mut ChaCha8Rng) -> MutationType {
        if rng.random::<f32>() < self.weights.structural_probability {
            let mutation = self.mutate_structure(&mut genome.structure, rng);
            MutationType::Structural(mutation)
        } else {
            let mutation = self.mutate_actuation(&mut genome.actuation, rng);
            MutationType::Actuation(mutation)
        }
    }

    /// Apply a structural mutation.
    fn mutate_structure(
        &self,
        structure: &mut StructuralGenome,
        rng: &mut ChaCha8Rng,
    ) -> StructuralMutation {
        let weights = &self.weights.structural;
        let total = weights.total();
        let roll = rng.random_range(0.0..total);

        let mut threshold = weights.add_branch;
        if roll < threshold {
            return self.try_add_branch(structure, rng);
        }

        threshold += weights.remove_branch;
        if roll < threshold {
            return self.try_remove_branch(structure, rng);
        }

        threshold += weights.adjust_column;
        if roll < threshold {
            return self.adjust_column(structure, rng);
        }

        threshold += weights.change_brick_type;
        if roll < threshold {
            return self.change_branch_brick_type(structure, rng);
        }

        threshold += weights.adjust_scale;
        if roll < threshold {
            return self.adjust_scale(structure, rng);
        }

        // change_seed
        self.change_seed_brick(structure, rng)
    }

    /// Try to add a branch (respects constraints).
    fn try_add_branch(
        &self,
        structure: &mut StructuralGenome,
        rng: &mut ChaCha8Rng,
    ) -> StructuralMutation {
        if structure.branches.len() >= self.constraints.max_branches {
            // Can't add more, adjust column instead
            return self.adjust_column(structure, rng);
        }

        // Check push count constraint
        let new_push_count = structure.push_count() + 3; // SingleTwist default
        if new_push_count > self.constraints.max_pushes {
            return self.adjust_column(structure, rng);
        }

        // Find an unused face
        let face_count = structure.seed_brick.face_count();
        let used_faces: Vec<usize> = structure.branches.iter().map(|b| b.seed_face).collect();
        let available: Vec<usize> = (0..face_count)
            .filter(|f| !used_faces.contains(f))
            .collect();

        if available.is_empty() {
            return self.adjust_column(structure, rng);
        }

        let face = available[rng.random_range(0..available.len())];
        structure.branches.push(Branch::minimal(face));
        StructuralMutation::AddBranch
    }

    /// Try to remove a branch (respects minimum).
    fn try_remove_branch(
        &self,
        structure: &mut StructuralGenome,
        rng: &mut ChaCha8Rng,
    ) -> StructuralMutation {
        if structure.branches.len() <= 1 {
            // Keep at least one branch
            return self.adjust_column(structure, rng);
        }

        let idx = rng.random_range(0..structure.branches.len());
        structure.branches.remove(idx);
        StructuralMutation::RemoveBranch
    }

    /// Adjust column count in a random branch.
    fn adjust_column(
        &self,
        structure: &mut StructuralGenome,
        rng: &mut ChaCha8Rng,
    ) -> StructuralMutation {
        if structure.branches.is_empty() {
            return StructuralMutation::AdjustColumnUp; // No-op
        }

        let idx = rng.random_range(0..structure.branches.len());

        // Capture values before mutable borrow
        let current_push_count = structure.push_count();
        let column_count = structure.branches[idx].column_count;
        let brick_push_count = structure.branches[idx].brick_type.push_count();

        // Decide up or down
        if rng.random::<bool>() {
            // Try to increase
            if column_count < self.constraints.max_column_depth as u8 {
                // Check push constraint
                let new_pushes = current_push_count + brick_push_count;
                if new_pushes <= self.constraints.max_pushes {
                    structure.branches[idx].column_count += 1;
                    return StructuralMutation::AdjustColumnUp;
                }
            }
            // Fall back to decrease
            if column_count > 0 {
                structure.branches[idx].column_count -= 1;
            }
            StructuralMutation::AdjustColumnDown
        } else {
            // Try to decrease
            if column_count > 0 {
                structure.branches[idx].column_count -= 1;
                StructuralMutation::AdjustColumnDown
            } else {
                // Can't decrease, try increase
                if column_count < self.constraints.max_column_depth as u8 {
                    let new_pushes = current_push_count + brick_push_count;
                    if new_pushes <= self.constraints.max_pushes {
                        structure.branches[idx].column_count += 1;
                    }
                }
                StructuralMutation::AdjustColumnUp
            }
        }
    }

    /// Change brick type in a random branch.
    fn change_branch_brick_type(
        &self,
        structure: &mut StructuralGenome,
        rng: &mut ChaCha8Rng,
    ) -> StructuralMutation {
        if structure.branches.is_empty() {
            return StructuralMutation::ChangeBrickType;
        }

        let idx = rng.random_range(0..structure.branches.len());

        // Capture values before mutable borrow
        let current_push_count = structure.push_count();
        let column_count = structure.branches[idx].column_count;
        let old_brick_push_count = structure.branches[idx].brick_type.push_count();

        let all_types = [BrickType::SingleTwist, BrickType::Omni, BrickType::Torque];
        let new_type = all_types[rng.random_range(0..all_types.len())];

        // Check if change would exceed push limit
        let old_pushes = old_brick_push_count * column_count as usize;
        let new_pushes = new_type.push_count() * column_count as usize;
        let total_new = current_push_count - old_pushes + new_pushes;

        if total_new <= self.constraints.max_pushes {
            structure.branches[idx].brick_type = new_type;
        }

        StructuralMutation::ChangeBrickType
    }

    /// Adjust scale of seed or branch.
    fn adjust_scale(
        &self,
        structure: &mut StructuralGenome,
        rng: &mut ChaCha8Rng,
    ) -> StructuralMutation {
        let adjust_seed = structure.branches.is_empty() || rng.random::<f32>() < 0.3;
        let factor = if rng.random::<bool>() { 1.1 } else { 0.9 };

        if adjust_seed {
            let new_scale = (structure.seed_scale * factor)
                .clamp(self.constraints.min_scale, self.constraints.max_scale);
            structure.seed_scale = new_scale;
        } else {
            let idx = rng.random_range(0..structure.branches.len());
            let branch = &mut structure.branches[idx];
            let new_scale =
                (branch.scale * factor).clamp(self.constraints.min_scale, self.constraints.max_scale);
            branch.scale = new_scale;
        }

        if factor > 1.0 {
            StructuralMutation::AdjustScaleUp
        } else {
            StructuralMutation::AdjustScaleDown
        }
    }

    /// Change the seed brick type.
    fn change_seed_brick(
        &self,
        structure: &mut StructuralGenome,
        rng: &mut ChaCha8Rng,
    ) -> StructuralMutation {
        let all_types = [BrickType::SingleTwist, BrickType::Omni, BrickType::Torque];
        let new_type = all_types[rng.random_range(0..all_types.len())];

        // Check push constraint
        let old_seed_pushes = structure.seed_brick.push_count();
        let new_seed_pushes = new_type.push_count();
        let total_new = structure.push_count() - old_seed_pushes + new_seed_pushes;

        if total_new <= self.constraints.max_pushes {
            structure.seed_brick = new_type;

            // Validate branch faces (might be out of range for new seed)
            let max_face = new_type.face_count();
            for branch in &mut structure.branches {
                if branch.seed_face >= max_face {
                    branch.seed_face = branch.seed_face % max_face;
                }
            }
        }

        StructuralMutation::ChangeSeedBrick
    }

    /// Apply an actuation mutation.
    fn mutate_actuation(
        &self,
        actuation: &mut ActuationGenome,
        rng: &mut ChaCha8Rng,
    ) -> ActuationMutation {
        let weights = &self.weights.actuation;
        let total = weights.total();
        let roll = rng.random_range(0.0..total);

        let mut threshold = weights.change_pattern;
        if roll < threshold {
            return self.change_pattern(actuation, rng);
        }

        threshold += weights.adjust_period;
        if roll < threshold {
            return self.adjust_period(actuation, rng);
        }

        threshold += weights.adjust_amplitude;
        if roll < threshold {
            return self.adjust_amplitude(actuation, rng);
        }

        threshold += weights.adjust_stiffness;
        if roll < threshold {
            return self.adjust_stiffness(actuation, rng);
        }

        threshold += weights.change_waveform;
        if roll < threshold {
            return self.change_waveform(actuation, rng);
        }

        // adjust_pattern_param
        self.adjust_pattern_param(actuation, rng)
    }

    fn change_pattern(
        &self,
        actuation: &mut ActuationGenome,
        rng: &mut ChaCha8Rng,
    ) -> ActuationMutation {
        let patterns = [
            ActuationPattern::Synchronized,
            ActuationPattern::Alternating { shift: 0.5 },
            ActuationPattern::Wave { wavelength: 2.0 },
            ActuationPattern::PerBranch {
                offsets: [0.0, 0.33, 0.67, 0.0, 0.33, 0.67],
            },
        ];
        actuation.pattern = patterns[rng.random_range(0..patterns.len())].clone();
        ActuationMutation::ChangePattern
    }

    fn adjust_period(
        &self,
        actuation: &mut ActuationGenome,
        rng: &mut ChaCha8Rng,
    ) -> ActuationMutation {
        let factor = if rng.random::<bool>() { 1.15 } else { 0.85 };
        actuation.period = (actuation.period * factor)
            .clamp(self.constraints.min_period, self.constraints.max_period);

        if factor > 1.0 {
            ActuationMutation::AdjustPeriodUp
        } else {
            ActuationMutation::AdjustPeriodDown
        }
    }

    fn adjust_amplitude(
        &self,
        actuation: &mut ActuationGenome,
        rng: &mut ChaCha8Rng,
    ) -> ActuationMutation {
        let factor = if rng.random::<bool>() { 1.1 } else { 0.9 };
        actuation.amplitude = (actuation.amplitude * factor)
            .clamp(self.constraints.min_amplitude, self.constraints.max_amplitude);

        if factor > 1.0 {
            ActuationMutation::AdjustAmplitudeUp
        } else {
            ActuationMutation::AdjustAmplitudeDown
        }
    }

    fn adjust_stiffness(
        &self,
        actuation: &mut ActuationGenome,
        rng: &mut ChaCha8Rng,
    ) -> ActuationMutation {
        let factor = if rng.random::<bool>() { 1.1 } else { 0.9 };
        actuation.stiffness = (actuation.stiffness * factor)
            .clamp(self.constraints.min_stiffness, self.constraints.max_stiffness);

        if factor > 1.0 {
            ActuationMutation::AdjustStiffnessUp
        } else {
            ActuationMutation::AdjustStiffnessDown
        }
    }

    fn change_waveform(
        &self,
        actuation: &mut ActuationGenome,
        rng: &mut ChaCha8Rng,
    ) -> ActuationMutation {
        actuation.waveform = match actuation.waveform {
            Waveform::Sine => Waveform::Pulse {
                duty_cycle: crate::units::Percent(50.0),
            },
            Waveform::Pulse { .. } => Waveform::Sine,
        };
        let _ = rng; // Suppress unused warning
        ActuationMutation::ChangeWaveform
    }

    fn adjust_pattern_param(
        &self,
        actuation: &mut ActuationGenome,
        rng: &mut ChaCha8Rng,
    ) -> ActuationMutation {
        match &mut actuation.pattern {
            ActuationPattern::Synchronized => {
                // No params to adjust, change to something else
                actuation.pattern = ActuationPattern::Alternating { shift: 0.5 };
            }
            ActuationPattern::Alternating { shift } => {
                *shift = (*shift + rng.random_range(-0.1..0.1)).clamp(0.1, 0.9);
            }
            ActuationPattern::Wave { wavelength } => {
                let factor = if rng.random::<bool>() { 1.2 } else { 0.8 };
                *wavelength = (*wavelength * factor).clamp(0.5, 5.0);
            }
            ActuationPattern::PerBranch { offsets } => {
                let idx = rng.random_range(0..offsets.len());
                offsets[idx] = (offsets[idx] + rng.random_range(-0.1..0.1)).clamp(0.0, 1.0);
            }
        }
        ActuationMutation::AdjustPatternParam
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_chacha::rand_core::SeedableRng;

    #[test]
    fn test_mutations_preserve_validity() {
        let weights = MutationWeights::default();
        let constraints = GenomeConstraints::default();
        let mutator = WalkingMutator::new(weights, constraints.clone());
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let mut genome = WalkingGenome::default();
        assert!(genome.is_valid(&constraints));

        // Apply 100 mutations and verify validity is preserved
        for _ in 0..100 {
            mutator.mutate(&mut genome, &mut rng);
            assert!(
                genome.is_valid(&constraints),
                "Genome invalid after mutation: {:?}",
                genome
            );
        }
    }

    #[test]
    fn test_push_count_stays_bounded() {
        let weights = MutationWeights::default();
        let constraints = GenomeConstraints::default();
        let mutator = WalkingMutator::new(weights, constraints.clone());
        let mut rng = ChaCha8Rng::seed_from_u64(123);

        let mut genome = WalkingGenome::default();

        for _ in 0..100 {
            mutator.mutate(&mut genome, &mut rng);
            assert!(
                genome.push_count() <= constraints.max_pushes,
                "Push count {} exceeds max {}",
                genome.push_count(),
                constraints.max_pushes
            );
        }
    }
}
