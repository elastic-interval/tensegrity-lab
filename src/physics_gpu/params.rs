use crate::fabric::physics::Physics;
use crate::fabric::Fabric;
use crate::units::Unit;

/// Configuration for a `GpuBatch` step pass. Derived from a tensegrity-lab
/// `Fabric` and its active `Physics`, and byte-compatible with the WGSL
/// `Params` uniform once counts are filled in during upload.
#[derive(Clone, Debug)]
pub struct GpuPhysicsConfig {
    pub dt: f32,
    pub gravity: f32,
    pub drag: f32,
    pub viscosity: f32,
    pub ambient_mass: f32,
    pub force_scale: f32,
    pub ground_y: f32,
    pub speed_limit: f32,
    pub surface_character: u32,
}

impl GpuPhysicsConfig {
    /// Build a config from a fabric and its physics. The fabric supplies
    /// ambient mass; the physics supplies damping and (optionally) gravity.
    pub fn from_fabric(fabric: &Fabric, physics: &Physics) -> Self {
        Self {
            dt: crate::Age::iteration_duration(),
            gravity: if physics.surface.is_some() {
                crate::units::EARTH_GRAVITY.f32()
            } else {
                0.0
            },
            drag: physics.drag(),
            viscosity: physics.viscosity(),
            ambient_mass: fabric.ambient_mass().f32(),
            force_scale: 1e4,
            ground_y: 0.0,
            speed_limit: 1000.0,
            surface_character: SURFACE_ABSENT,
        }
    }
}

pub const SURFACE_ABSENT: u32 = 0;
#[allow(dead_code)]
pub const SURFACE_BOUNCY: u32 = 1;
#[allow(dead_code)]
pub const SURFACE_FROZEN: u32 = 2;
#[allow(dead_code)]
pub const SURFACE_STICKY: u32 = 3;
#[allow(dead_code)]
pub const SURFACE_SLIPPERY: u32 = 4;

/// Byte-identical mirror of the WGSL `Params` struct. 16 slots × 4 bytes
/// = 64 bytes total. Do not reorder.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PhysicsParams {
    pub dt: f32,
    pub gravity: f32,
    pub drag: f32,
    pub viscosity: f32,
    pub num_joints: u32,
    pub num_elastic: u32,
    pub _reserved0: u32,
    pub ambient_mass: f32,
    pub force_scale: f32,
    pub ground_y: f32,
    pub _reserved1: f32,
    pub speed_limit: f32,
    pub num_push: u32,
    pub surface_character: u32,
    pub _pad2: u32,
    pub _pad3: u32,
}

impl PhysicsParams {
    pub fn from_config(config: &GpuPhysicsConfig) -> Self {
        Self {
            dt: config.dt,
            gravity: config.gravity,
            drag: config.drag,
            viscosity: config.viscosity,
            num_joints: 0,
            num_elastic: 0,
            _reserved0: 0,
            ambient_mass: config.ambient_mass,
            force_scale: config.force_scale,
            ground_y: config.ground_y,
            _reserved1: 0.0,
            speed_limit: config.speed_limit,
            num_push: 0,
            surface_character: config.surface_character,
            _pad2: 0,
            _pad3: 0,
        }
    }
}
