//! Genome representation for evolving walking tensegrity structures.
//!
//! The genome has two parts:
//! - StructuralGenome: Defines which bricks to assemble and how
//! - ActuationGenome: Defines actuation patterns and parameters

use crate::build::dsl::animate_phase::Waveform;

/// Brick types available for evolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrickType {
    /// Simple 3-push brick, good for legs/columns
    SingleTwist,
    /// 6-push omnidirectional hub, good for central body
    Omni,
    /// 9-push torque-resistant brick
    Torque,
}

impl BrickType {
    /// Number of attachment faces available on this brick type
    pub fn face_count(&self) -> usize {
        match self {
            BrickType::SingleTwist => 2, // Top and bottom
            BrickType::Omni => 8,        // 8 triangular faces
            BrickType::Torque => 8,      // 8 faces
        }
    }

    /// Number of pushes in this brick type
    pub fn push_count(&self) -> usize {
        match self {
            BrickType::SingleTwist => 3,
            BrickType::Omni => 6,
            BrickType::Torque => 9,
        }
    }
}

/// A branch extending from the seed brick.
#[derive(Clone, Debug)]
pub struct Branch {
    /// Which face of the seed brick to attach to (0..seed.face_count())
    pub seed_face: usize,
    /// Number of bricks in the column (0 = just attachment, 1-2 = column)
    pub column_count: u8,
    /// What type of brick to use in the column
    pub brick_type: BrickType,
    /// Scale factor for bricks in this branch (0.8-1.2)
    pub scale: f32,
}

impl Branch {
    /// Create a minimal branch with just an attachment point
    pub fn minimal(seed_face: usize) -> Self {
        Self {
            seed_face,
            column_count: 0,
            brick_type: BrickType::SingleTwist,
            scale: 1.0,
        }
    }

    /// Create a single-brick column branch
    pub fn single(seed_face: usize, brick_type: BrickType) -> Self {
        Self {
            seed_face,
            column_count: 1,
            brick_type,
            scale: 1.0,
        }
    }

    /// Total push count for this branch
    pub fn push_count(&self) -> usize {
        self.brick_type.push_count() * self.column_count as usize
    }
}

/// The structural genome defining brick arrangement.
#[derive(Clone, Debug)]
pub struct StructuralGenome {
    /// The central seed brick type
    pub seed_brick: BrickType,
    /// Scale of the seed brick (0.5-1.5)
    pub seed_scale: f32,
    /// Branches extending from the seed (max 3 initially)
    pub branches: Vec<Branch>,
}

impl Default for StructuralGenome {
    fn default() -> Self {
        // Starting structure: SingleTwist seed with 2 single-brick columns
        // Push count: 3 seed + 2*3 branch columns = 9 pushes
        Self {
            seed_brick: BrickType::SingleTwist,
            seed_scale: 1.0,
            branches: vec![
                Branch::single(0, BrickType::SingleTwist),
                Branch::single(1, BrickType::SingleTwist),
            ],
        }
    }
}

impl StructuralGenome {
    /// Total push count for the entire structure
    pub fn push_count(&self) -> usize {
        let seed_pushes = self.seed_brick.push_count();
        let branch_pushes: usize = self.branches.iter().map(|b| b.push_count()).sum();
        seed_pushes + branch_pushes
    }

    /// Validate that the genome is within constraints
    pub fn is_valid(&self, config: &GenomeConstraints) -> bool {
        if self.branches.len() > config.max_branches {
            return false;
        }
        if self.seed_scale < config.min_scale || self.seed_scale > config.max_scale {
            return false;
        }
        for branch in &self.branches {
            if branch.column_count > config.max_column_depth as u8 {
                return false;
            }
            if branch.scale < config.min_scale || branch.scale > config.max_scale {
                return false;
            }
            if branch.seed_face >= self.seed_brick.face_count() {
                return false;
            }
        }
        if self.push_count() > config.max_pushes {
            return false;
        }
        true
    }
}

/// Actuation patterns that map to structure topology.
#[derive(Clone, Debug)]
pub enum ActuationPattern {
    /// Phase offset increases with distance from center
    Wave {
        /// Wavelength in number of bricks
        wavelength: f32,
    },
    /// Odd and even branches alternate
    Alternating {
        /// Phase shift between odd/even (0.0-1.0)
        shift: f32,
    },
    /// All actuators fire together
    Synchronized,
    /// Each branch has independent phase offset
    PerBranch {
        /// Phase offset for each branch (up to 6)
        offsets: [f32; 6],
    },
}

impl Default for ActuationPattern {
    fn default() -> Self {
        ActuationPattern::Alternating { shift: 0.5 }
    }
}

/// The actuation genome defining how the structure moves.
#[derive(Clone, Debug)]
pub struct ActuationGenome {
    /// The actuation pattern type
    pub pattern: ActuationPattern,
    /// Oscillation period in seconds (0.3-2.0)
    pub period: f32,
    /// Contraction amplitude as percentage (1.0-5.0)
    pub amplitude: f32,
    /// Actuator stiffness as percentage (5.0-20.0)
    pub stiffness: f32,
    /// Waveform type
    pub waveform: Waveform,
}

impl Default for ActuationGenome {
    fn default() -> Self {
        Self {
            pattern: ActuationPattern::default(),
            period: 0.5,
            amplitude: 3.0,
            stiffness: 10.0,
            waveform: Waveform::Sine,
        }
    }
}

impl ActuationGenome {
    /// Validate that the actuation parameters are within constraints
    pub fn is_valid(&self, config: &GenomeConstraints) -> bool {
        if self.period < config.min_period || self.period > config.max_period {
            return false;
        }
        if self.amplitude < config.min_amplitude || self.amplitude > config.max_amplitude {
            return false;
        }
        if self.stiffness < config.min_stiffness || self.stiffness > config.max_stiffness {
            return false;
        }
        true
    }
}

/// The complete walking genome.
#[derive(Clone, Debug)]
pub struct WalkingGenome {
    /// Structural arrangement of bricks
    pub structure: StructuralGenome,
    /// Actuation pattern and parameters
    pub actuation: ActuationGenome,
}

impl Default for WalkingGenome {
    fn default() -> Self {
        Self {
            structure: StructuralGenome::default(),
            actuation: ActuationGenome::default(),
        }
    }
}

impl WalkingGenome {
    /// Create a minimal walking genome for testing
    pub fn minimal() -> Self {
        Self {
            structure: StructuralGenome {
                seed_brick: BrickType::SingleTwist,
                seed_scale: 1.0,
                branches: vec![Branch::minimal(0)],
            },
            actuation: ActuationGenome::default(),
        }
    }

    /// Total push count
    pub fn push_count(&self) -> usize {
        self.structure.push_count()
    }

    /// Validate the entire genome
    pub fn is_valid(&self, config: &GenomeConstraints) -> bool {
        self.structure.is_valid(config) && self.actuation.is_valid(config)
    }
}

/// Constraints for genome validity.
#[derive(Clone, Debug)]
pub struct GenomeConstraints {
    pub max_branches: usize,
    pub max_column_depth: usize,
    pub max_pushes: usize,
    pub min_scale: f32,
    pub max_scale: f32,
    pub min_period: f32,
    pub max_period: f32,
    pub min_amplitude: f32,
    pub max_amplitude: f32,
    pub min_stiffness: f32,
    pub max_stiffness: f32,
}

impl Default for GenomeConstraints {
    fn default() -> Self {
        Self {
            max_branches: 3,
            max_column_depth: 2,
            max_pushes: 12,
            min_scale: 0.5,
            max_scale: 1.5,
            min_period: 0.3,
            max_period: 2.0,
            min_amplitude: 1.0,
            max_amplitude: 5.0,
            min_stiffness: 5.0,
            max_stiffness: 20.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_genome_is_valid() {
        let genome = WalkingGenome::default();
        let constraints = GenomeConstraints::default();
        assert!(genome.is_valid(&constraints));
    }

    #[test]
    fn test_minimal_genome_is_valid() {
        let genome = WalkingGenome::minimal();
        let constraints = GenomeConstraints::default();
        assert!(genome.is_valid(&constraints));
    }

    #[test]
    fn test_push_count() {
        let genome = WalkingGenome::default();
        // SingleTwist seed (3) + 2 SingleTwist branches (3 each) = 9
        assert_eq!(genome.push_count(), 9);
    }

    #[test]
    fn test_too_many_branches_invalid() {
        let mut genome = WalkingGenome::default();
        genome.structure.branches = vec![
            Branch::single(0, BrickType::SingleTwist),
            Branch::single(1, BrickType::SingleTwist),
            Branch::single(2, BrickType::SingleTwist),
            Branch::single(3, BrickType::SingleTwist), // 4th branch
        ];
        let constraints = GenomeConstraints::default();
        assert!(!genome.is_valid(&constraints));
    }
}
