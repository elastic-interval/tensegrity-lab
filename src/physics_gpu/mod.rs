//! GPU compute backend for parallel fabric stepping.
//!
//! Design: `docs/gpu-compute-backend.md`.
//!
//! The CPU build pipeline stays authoritative. This module accepts
//! slices of already-built `Fabric`s, parallelizes each one onto the
//! GPU, and steps them all in lockstep — one slot per fabric, no
//! shared topology. Phase 2 supports single-fabric batches only;
//! phase 4 will generalize to N.

pub mod batch;
pub mod params;

pub use batch::{run_generation, GpuBatch, SlotFacts};
pub use params::{GpuPhysicsConfig, PhysicsParams};

#[cfg(test)]
mod parity_test;
#[cfg(test)]
mod smoke_test;
#[cfg(test)]
mod sphere_sweep_test;
