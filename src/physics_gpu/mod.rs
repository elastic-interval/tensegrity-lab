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

#[cfg(test)]
pub(crate) fn create_headless_device(label: &str) -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter = futures::executor::block_on(instance.request_adapter(
        &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: None,
        },
    ))
    .ok()?;
    let limits = adapter.limits();
    let (device, queue) = futures::executor::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some(label),
            required_features: wgpu::Features::empty(),
            required_limits: limits,
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
            experimental_features: wgpu::ExperimentalFeatures::default(),
        },
    ))
    .ok()?;
    Some((device, queue))
}
