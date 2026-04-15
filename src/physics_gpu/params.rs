use crate::fabric::physics::Physics;
use crate::fabric::Fabric;
use crate::units::Unit;

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
    pub surface_scale: f32,
}

impl GpuPhysicsConfig {
    pub fn from_fabric(fabric: &Fabric, physics: &Physics) -> Self {
        // Surface character mapping matches the WGSL switch in ground_collision:
        // 0=absent, 1=bouncy, 2=frozen, 3=sticky, 4=slippery.
        let (surface_character, surface_scale) = match &physics.surface {
            None => (0, 1.0),
            Some(surface) => {
                use crate::fabric::physics::SurfaceCharacter::*;
                let character = match surface.character {
                    Bouncy => 1,
                    Frozen => 2,
                    Sticky => 3,
                    Slippery => 4,
                };
                (character, surface.scale)
            }
        };
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
            force_scale: 1.0,
            ground_y: 0.0,
            // High sanity bound (NaN / runaway catch), not a physics parameter.
            // Now that quadratic damping is unconditionally stable, routine
            // transients stay orders of magnitude below this.
            speed_limit: 10_000.0,
            surface_character,
            surface_scale,
        }
    }
}

/// Byte-identical mirror of the WGSL `Params` struct.
/// 16 slots × 4 bytes = 64 bytes. Do not reorder.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PhysicsParams {
    pub dt: f32,
    pub gravity: f32,
    pub drag: f32,
    pub viscosity: f32,
    pub max_joints: u32,
    pub max_elastic: u32,
    pub max_push: u32,
    pub ambient_mass: f32,
    pub force_scale: f32,
    pub ground_y: f32,
    pub num_slots: u32,
    pub speed_limit: f32,
    pub surface_character: u32,
    pub surface_scale: f32,
    pub _pad1: u32,
    pub _pad2: u32,
}

impl PhysicsParams {
    pub fn from_config(
        config: &GpuPhysicsConfig,
        max_joints: u32,
        max_elastic: u32,
        max_push: u32,
        num_slots: u32,
    ) -> Self {
        Self {
            dt: config.dt,
            gravity: config.gravity,
            drag: config.drag,
            viscosity: config.viscosity,
            max_joints,
            max_elastic,
            max_push,
            ambient_mass: config.ambient_mass,
            force_scale: config.force_scale,
            ground_y: config.ground_y,
            num_slots,
            speed_limit: config.speed_limit,
            surface_character: config.surface_character,
            surface_scale: config.surface_scale,
            _pad1: 0,
            _pad2: 0,
        }
    }
}
