//! Walking tensegrity evolution module.
//!
//! This module implements evolution of brick-based tensegrity structures
//! optimized for walking locomotion on a sticky surface.
//!
//! # Architecture
//!
//! The system evolves two coupled genomes:
//! - **StructuralGenome**: Defines brick arrangement (seed + branches)
//! - **ActuationGenome**: Defines movement patterns and parameters
//!
//! Fitness is evaluated based on:
//! - Distance traveled per unit time (velocity)
//! - Structural efficiency (push count penalty)
//! - Energy efficiency (actuator work per meter)
//! - Stability (maintaining height during locomotion)
//!
//! # Lifecycle
//!
//! Each evaluation follows the standard FabricPlan lifecycle:
//! 1. Build: Assemble structure from bricks above ground
//! 2. Pretense: Apply tension to stabilize
//! 3. Fall: Drop to sticky surface
//! 4. Settle: Allow structure to stabilize
//! 5. Animate: Run actuators and measure locomotion

mod compiler;
mod evolution;
mod fitness;
mod genome;
mod mutations;

pub use evolution::{
    ViewingMode, WalkingEvolution, WalkingEvolutionConfig, WalkingIndividual, WalkingState,
};
pub use fitness::{InitialState, WalkingFitness, WalkingFitnessConfig, WalkingFitnessDetails};
pub use genome::{
    ActuationGenome, ActuationPattern, Branch, BrickType, GenomeConstraints, StructuralGenome,
    WalkingGenome,
};
pub use mutations::{MutationType, MutationWeights, WalkingMutator};
