//! GPU compute backend for parallel fabric stepping.
//!
//! Design: `docs/gpu-compute-backend.md`.
//!
//! The CPU build pipeline stays authoritative. This module accepts
//! slices of already-built `Fabric`s, freezes each one, and steps them
//! all in lockstep on the GPU — one slot per fabric, no shared topology.

pub mod growable;
pub mod params;

pub use growable::GrowablePhysics;
pub use params::{GpuPhysicsConfig, PhysicsParams};

#[cfg(test)]
mod smoke_test;
