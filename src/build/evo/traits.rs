/*
 * Abstract Evolution Framework - Core Traits
 *
 * This module defines the trait hierarchy for evolving tensegrity structures.
 * The design embodies Stuart Kauffman's "adjacent possible" concept:
 * evolution explores what's reachable from the current state, not a predefined space.
 */

use crate::fabric::physics::Physics;
use crate::fabric::Fabric;
use crate::units::Seconds;
use glam::Vec3;
use rand::Rng;
use std::fmt::Debug;

pub use crate::fabric::IntervalReading;

// ============================================================================
// Genome Trait - The Hereditary Information
// ============================================================================

/// Unique identifier for a genome instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GenomeId(pub u64);

impl GenomeId {
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for GenomeId {
    fn default() -> Self {
        Self::new()
    }
}

/// Context provided during genome expression (genotype -> phenotype).
pub struct ExpressionContext {
    pub physics: Physics,
}

impl ExpressionContext {
    pub fn new(physics: Physics) -> Self {
        Self { physics }
    }
}

pub trait Genome: Clone + Send + Sync + Debug {
    fn id(&self) -> GenomeId;
    fn express(&self, context: &ExpressionContext) -> Fabric;
    fn adjacent_possible(&self, rng: &mut impl Rng) -> Vec<Self>
    where
        Self: Sized;
    fn describe(&self) -> String;

    /// Controllers to attach to intervals. Default: none.
    fn controllers(&self) -> Vec<ControllerAttachment> {
        vec![]
    }
}

// ============================================================================
// Fitness Evaluation
// ============================================================================

/// Result of running a single trial (genome expressed and simulated).
#[derive(Clone, Debug)]
pub struct TrialResult {
    pub genome_id: GenomeId,

    /// The fabric after simulation.
    pub fabric: Fabric,

    /// Duration of the trial in fabric time.
    pub duration: Seconds,

    /// Number of physics iterations run.
    pub iteration_count: usize,

    /// Maximum strain encountered during trial.
    pub max_strain: f32,

    /// Whether any interval failed (went slack or over-strained).
    pub structural_failure: bool,

    /// Final kinetic energy (lower = more settled).
    pub final_kinetic_energy: f32,

    /// Centroid position at start of trial.
    pub initial_centroid: Vec3,

    /// Centroid position at end of trial.
    pub final_centroid: Vec3,
}

impl TrialResult {
    /// Displacement of centroid during trial.
    pub fn displacement(&self) -> Vec3 {
        self.final_centroid - self.initial_centroid
    }

    /// Horizontal displacement (XZ plane).
    pub fn horizontal_displacement(&self) -> f32 {
        let d = self.displacement();
        (d.x * d.x + d.z * d.z).sqrt()
    }
}

/// A single dimension of fitness evaluation.
/// Multiple dimensions can be composed for multi-objective optimization.
pub trait FitnessDimension: Send + Sync {
    /// Human-readable name for this dimension.
    fn name(&self) -> &str;

    /// Evaluate this dimension. Returns score where higher is better.
    /// Takes the trial result after simulation.
    fn evaluate(&self, trial: &TrialResult) -> f32;

    /// Weight of this dimension in composite fitness (default 1.0).
    fn weight(&self) -> f32 {
        1.0
    }
}

/// Composite fitness from multiple dimensions.
pub struct CompositeFitness {
    dimensions: Vec<Box<dyn FitnessDimension>>,
}

impl CompositeFitness {
    pub fn new() -> Self {
        Self { dimensions: vec![] }
    }

    pub fn with_dimension(mut self, dim: Box<dyn FitnessDimension>) -> Self {
        self.dimensions.push(dim);
        self
    }

    pub fn add_dimension(&mut self, dim: Box<dyn FitnessDimension>) {
        self.dimensions.push(dim);
    }

    /// Evaluate composite fitness as weighted average.
    pub fn evaluate(&self, trial: &TrialResult) -> f32 {
        if self.dimensions.is_empty() {
            return 0.0;
        }

        let mut total_weight = 0.0;
        let mut weighted_sum = 0.0;

        for dim in &self.dimensions {
            let weight = dim.weight();
            let score = dim.evaluate(trial);
            weighted_sum += score * weight;
            total_weight += weight;
        }

        if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            0.0
        }
    }

    /// Get detailed breakdown of fitness by dimension.
    pub fn evaluate_detailed(&self, trial: &TrialResult) -> Vec<(String, f32, f32)> {
        self.dimensions
            .iter()
            .map(|dim| (dim.name().to_string(), dim.evaluate(trial), dim.weight()))
            .collect()
    }
}

impl Default for CompositeFitness {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Population Strategy
// ============================================================================

/// Statistics about the current population.
#[derive(Clone, Debug)]
pub struct PopulationStats {
    pub generation: usize,
    pub population_size: usize,
    pub trials_completed: usize,
    pub best_fitness: f32,
    pub mean_fitness: f32,
    pub worst_fitness: f32,
}

/// Strategy for managing a population of genomes.
///
/// This trait enables different selection and reproduction strategies:
/// - Generational (replace all at once)
/// - Steady-state (continuous replacement)
/// - Island model (multiple subpopulations)
pub trait PopulationStrategy<G: Genome>: Send {
    /// Initialize the population with seed genomes.
    fn initialize(&mut self, seed_genomes: Vec<G>);

    /// Select the next genome to evaluate.
    /// Returns None if generation is complete.
    fn next_for_trial(&mut self) -> Option<G>;

    /// Record the result of a trial.
    fn record_result(&mut self, genome: G, fitness: f32);

    /// Called when a generation is complete. Performs selection and reproduction.
    fn advance_generation(&mut self);

    /// Get the current best genome(s).
    fn best(&self) -> Option<&G>;

    /// Get population statistics.
    fn stats(&self) -> PopulationStats;

    /// Check if evolution should terminate.
    fn should_terminate(&self) -> bool;
}

// ============================================================================
// Sensorimotor Control
// ============================================================================

pub trait IntervalController: Send + Sync {
    fn react(&mut self, reading: &IntervalReading) -> Option<f32>;
    fn reset(&mut self) {}
}

pub struct ControllerAttachment {
    pub interval_index: usize,
    pub controller: Box<dyn IntervalController>,
}

// ============================================================================
// Trial Configuration
// ============================================================================

/// Configuration for running trials.
#[derive(Clone, Debug)]
pub struct TrialConfig {
    /// Physics settings for the trial.
    pub physics: Physics,

    /// Duration of each trial in fabric time.
    pub trial_duration: Seconds,

    /// Maximum iterations per trial (safety limit).
    pub max_iterations: usize,
}

impl TrialConfig {
    pub fn new(physics: Physics, trial_duration: Seconds) -> Self {
        // Calculate iterations from duration (50µs per iteration)
        let max_iterations = (trial_duration.0 / 0.00005) as usize;
        Self {
            physics,
            trial_duration,
            max_iterations,
        }
    }
}
