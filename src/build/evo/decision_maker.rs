use crate::build::evo::genome::Genome;
use glam::Vec3;
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;

/// Wraps a seeded PRNG with genome-based skip logic.
///
/// The DecisionMaker provides deterministic random decisions that can be
/// perfectly reproduced by replaying with the same seed and genome.
/// The genome controls which random numbers to skip, creating different
/// outcomes from the same seed.
pub struct DecisionMaker {
    rng: ChaCha8Rng,
    genome: Genome,
    /// Position in the "virtual" sequence (counting skips)
    virtual_position: usize,
}

impl DecisionMaker {
    /// Create a new DecisionMaker with a seed and genome.
    pub fn new(seed: u64, genome: Genome) -> Self {
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed),
            genome,
            virtual_position: 0,
        }
    }

    /// Get the next random f32 in [0, 1), respecting skips.
    fn next_f32(&mut self) -> f32 {
        loop {
            let value: f32 = self.rng.random();

            if self.genome.should_skip(self.virtual_position) {
                self.virtual_position += 1;
                continue; // Skip this value, get another
            }

            self.virtual_position += 1;
            return value;
        }
    }

    /// Make a boolean decision (50/50 chance).
    pub fn decide(&mut self) -> bool {
        self.next_f32() > 0.5
    }

    /// Choose an index in range [0, max).
    pub fn choose(&mut self, max: usize) -> usize {
        if max == 0 {
            return 0;
        }
        (self.next_f32() * max as f32).floor() as usize
    }

    /// Choose a value in a range [min, max).
    pub fn range(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }

    /// Generate a random unit vector (normalized direction).
    pub fn random_direction(&mut self) -> Vec3 {
        // Generate a random vector and normalize
        let x = self.next_f32() * 2.0 - 1.0;
        let y = self.next_f32() * 2.0 - 1.0;
        let z = self.next_f32() * 2.0 - 1.0;
        Vec3::new(x, y, z).normalize_or_zero()
    }

    /// Get current virtual position (useful for mutation targeting).
    pub fn virtual_position(&self) -> usize {
        self.virtual_position
    }

    /// Clone the genome (for creating offspring).
    pub fn clone_genome(&self) -> Genome {
        self.genome.clone()
    }

    /// Get a reference to the genome.
    pub fn genome(&self) -> &Genome {
        &self.genome
    }
}
