// GPU compute backend for tensegrity-lab. Mirrors the semantics of
// Fabric::iterate (src/fabric/mod.rs) exactly, so a per-fabric step on
// the GPU produces the same positions as the CPU reference path
// within float precision and atomic-quantization tolerance.
//
// Ported from chopstix/src/gpu/physics.wgsl, with the following changes:
//  - rigid-strut path (SHAKE/RATTLE) removed entirely; tensegrity-lab
//    uses spring pushes
//  - per-interval linear density is uploaded rather than hardcoded
//    0.05 kg/m, so elastic and push half-mass accumulation matches
//    the CPU's material.linear_density × actual_length / 2
//  - push forces include the CPU slack check (push goes slack when
//    stretched beyond ideal)
//  - quadratic viscosity term added to second_half_kick, applied
//    before the linear drag term, matching joint.apply_damping_and_surface

// Joint state (Group 0)
@group(0) @binding(0) var<storage, read_write> positions: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> velocities: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> force_x: array<atomic<i32>>;
@group(0) @binding(3) var<storage, read_write> force_y: array<atomic<i32>>;
@group(0) @binding(4) var<storage, read_write> force_z: array<atomic<i32>>;
@group(0) @binding(5) var<storage, read_write> masses: array<atomic<i32>>;
@group(0) @binding(6) var<storage, read_write> frozen: atomic<u32>;

// Elastic interval topology (Group 1)
@group(1) @binding(0) var<storage, read> elastic_alpha: array<u32>;
@group(1) @binding(1) var<storage, read> elastic_omega: array<u32>;
@group(1) @binding(2) var<storage, read> elastic_ideal: array<f32>;
@group(1) @binding(3) var<storage, read> elastic_k: array<f32>;
@group(1) @binding(4) var<storage, read> elastic_linear_density: array<f32>;

// Push interval topology (Group 3)
@group(3) @binding(0) var<storage, read> push_alpha: array<u32>;
@group(3) @binding(1) var<storage, read> push_omega: array<u32>;
@group(3) @binding(2) var<storage, read> push_ideal: array<f32>;
@group(3) @binding(3) var<storage, read> push_k: array<f32>;
@group(3) @binding(4) var<storage, read> push_linear_density: array<f32>;

// Uniform params (Group 2). Byte-identical to PhysicsParams in params.rs.
struct Params {
    dt: f32,
    gravity: f32,
    drag: f32,
    viscosity: f32,
    num_joints: u32,
    num_elastic: u32,
    _reserved0: u32,
    ambient_mass: f32,
    force_scale: f32,
    ground_y: f32,
    _reserved1: f32,
    speed_limit: f32,
    num_push: u32,
    surface_character: u32,
    _pad2: u32,
    _pad3: u32,
}
@group(2) @binding(0) var<uniform> params: Params;

fn check_speed_limit(vel: vec3<f32>) {
    let speed_sq = vel.x * vel.x + vel.y * vel.y + vel.z * vel.z;
    if speed_sq > params.speed_limit * params.speed_limit {
        atomicStore(&frozen, 1u);
    }
}

fn is_frozen() -> bool {
    return atomicLoad(&frozen) != 0u;
}

// Atomic-integer quantization scale for masses. Raw grams × MASS_SCALE
// gives smallest representable mass 1e-4 grams = 0.1 micrograms.
const MASS_SCALE: f32 = 1e4;

// First half-kick + drift: v += 0.5*(F/m)*dt, x += v*dt
// Mirrors the first two for-loops in Fabric::iterate.
@compute @workgroup_size(64)
fn half_kick_and_drift(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if idx >= params.num_joints || is_frozen() { return; }

    let fx = f32(atomicLoad(&force_x[idx])) / params.force_scale;
    let fy = f32(atomicLoad(&force_y[idx])) / params.force_scale;
    let fz = f32(atomicLoad(&force_z[idx])) / params.force_scale;
    let m = f32(atomicLoad(&masses[idx])) / MASS_SCALE;

    var vel = velocities[idx];
    // tensegrity-lab's unit system: force/mass-in-grams is treated as
    // an acceleration directly, no kg conversion. Matching.
    let inv_m = select(0.0, 1.0 / m, m > 0.0);
    vel.x += 0.5 * fx * inv_m * params.dt;
    vel.y += 0.5 * fy * inv_m * params.dt;
    vel.z += 0.5 * fz * inv_m * params.dt;

    check_speed_limit(vec3<f32>(vel.x, vel.y, vel.z));
    velocities[idx] = vel;

    var pos = positions[idx];
    pos.x += vel.x * params.dt;
    pos.y += vel.y * params.dt;
    pos.z += vel.z * params.dt;
    positions[idx] = pos;
}

// Elastic forces. Matches Interval::iterate for pull-like roles:
// slack when compressed (strain <= 0), otherwise F = k*strain*ideal
// applied in full (±) to each endpoint, and half the dynamically
// computed mass added to each endpoint's accumulated_mass.
@compute @workgroup_size(64)
fn elastic_forces(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if idx >= params.num_elastic || is_frozen() { return; }

    let a = elastic_alpha[idx];
    let o = elastic_omega[idx];
    let ideal = elastic_ideal[idx];
    let k = elastic_k[idx];
    let linear_density = elastic_linear_density[idx];

    let pos_a = positions[a];
    let pos_o = positions[o];

    let dx = pos_o.x - pos_a.x;
    let dy = pos_o.y - pos_a.y;
    let dz = pos_o.z - pos_a.z;
    let actual = sqrt(dx * dx + dy * dy + dz * dz);
    if actual < 0.0001 { return; }

    // Mass contribution is independent of slack state — the cable
    // still has mass even when slack.
    let interval_mass = linear_density * actual;
    let half_mass_i = i32(interval_mass * 0.5 * MASS_SCALE);
    atomicAdd(&masses[a], half_mass_i);
    atomicAdd(&masses[o], half_mass_i);

    let strain = (actual - ideal) / ideal;
    if strain <= 0.0 { return; }

    let force_mag = k * strain * ideal;
    let inv_actual = 1.0 / actual;
    let fx = force_mag * dx * inv_actual;
    let fy = force_mag * dy * inv_actual;
    let fz = force_mag * dz * inv_actual;

    let ifx = i32(fx * params.force_scale);
    let ify = i32(fy * params.force_scale);
    let ifz = i32(fz * params.force_scale);

    atomicAdd(&force_x[a], ifx);
    atomicAdd(&force_y[a], ify);
    atomicAdd(&force_z[a], ifz);
    atomicAdd(&force_x[o], -ifx);
    atomicAdd(&force_y[o], -ify);
    atomicAdd(&force_z[o], -ifz);
}

// Push forces. Matches Interval::iterate for Role::Pushing: slack when
// STRETCHED (actual > ideal, i.e. the strut would need to be longer),
// otherwise F = k*strain*ideal applied symmetrically.
@compute @workgroup_size(64)
fn push_forces(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if idx >= params.num_push || is_frozen() { return; }

    let a = push_alpha[idx];
    let o = push_omega[idx];
    let ideal = push_ideal[idx];
    let k = push_k[idx];
    let linear_density = push_linear_density[idx];

    let pos_a = positions[a];
    let pos_o = positions[o];

    let dx = pos_o.x - pos_a.x;
    let dy = pos_o.y - pos_a.y;
    let dz = pos_o.z - pos_a.z;
    let actual = sqrt(dx * dx + dy * dy + dz * dz);
    if actual < 0.0001 { return; }

    // Mass accumulation is unconditional.
    let interval_mass = linear_density * actual;
    let half_mass_i = i32(interval_mass * 0.5 * MASS_SCALE);
    atomicAdd(&masses[a], half_mass_i);
    atomicAdd(&masses[o], half_mass_i);

    // Push goes slack when stretched. Negative strain = compression,
    // which is what the strut wants to resist.
    if actual > ideal { return; }

    let strain = (actual - ideal) / ideal;
    let force_mag = k * strain * ideal;
    let inv_actual = 1.0 / actual;
    let fx = force_mag * dx * inv_actual;
    let fy = force_mag * dy * inv_actual;
    let fz = force_mag * dz * inv_actual;

    let ifx = i32(fx * params.force_scale);
    let ify = i32(fy * params.force_scale);
    let ifz = i32(fz * params.force_scale);

    atomicAdd(&force_x[a], ifx);
    atomicAdd(&force_y[a], ify);
    atomicAdd(&force_z[a], ifz);
    atomicAdd(&force_x[o], -ifx);
    atomicAdd(&force_y[o], -ify);
    atomicAdd(&force_z[o], -ifz);
}

// Second half-kick. Matches the post-intervals loop in Fabric::iterate:
// first adds gravity (if present), then the second half velocity kick,
// then quadratic viscosity, then linear drag — in that order — and
// resets forces/mass for the next iteration.
@compute @workgroup_size(64)
fn second_half_kick(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if idx >= params.num_joints { return; }

    let m = f32(atomicLoad(&masses[idx])) / MASS_SCALE;

    // Gravity is applied here rather than in a separate pass so that the
    // mass used is the complete accumulated mass (ambient + half of each
    // incident interval's mass), matching the CPU order.
    atomicAdd(&force_y[idx], i32(-m * params.gravity * params.force_scale));

    let fx = f32(atomicLoad(&force_x[idx])) / params.force_scale;
    let fy = f32(atomicLoad(&force_y[idx])) / params.force_scale;
    let fz = f32(atomicLoad(&force_z[idx])) / params.force_scale;

    // Reset forces and mass for the next iteration, exactly as
    // joint.reset_with_mass(ambient_mass) does on the CPU.
    atomicStore(&force_x[idx], 0);
    atomicStore(&force_y[idx], 0);
    atomicStore(&force_z[idx], 0);
    atomicStore(&masses[idx], i32(params.ambient_mass * MASS_SCALE));

    if is_frozen() { return; }

    let inv_m = select(0.0, 1.0 / m, m > 0.0);
    var vel = velocities[idx];
    vel.x += 0.5 * fx * inv_m * params.dt;
    vel.y += 0.5 * fy * inv_m * params.dt;
    vel.z += 0.5 * fz * inv_m * params.dt;

    // Damping — matches joint.apply_damping_and_surface with surface=None.
    // Quadratic viscosity first, then linear drag.
    let speed_sq = vel.x * vel.x + vel.y * vel.y + vel.z * vel.z;
    let viscosity_factor = 1.0 - speed_sq * params.viscosity * params.dt;
    vel.x *= viscosity_factor;
    vel.y *= viscosity_factor;
    vel.z *= viscosity_factor;

    let drag_factor = 1.0 - params.drag * params.dt;
    vel.x *= drag_factor;
    vel.y *= drag_factor;
    vel.z *= drag_factor;

    check_speed_limit(vec3<f32>(vel.x, vel.y, vel.z));
    velocities[idx] = vel;
}

// Ground collision. Disabled while surface=None (the common parity-test
// configuration). Mirrors Surface::interact for the four surface
// characters, but phase 2/3 exercises only surface_character==0.
@compute @workgroup_size(64)
fn ground_collision(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if idx >= params.num_joints || is_frozen() { return; }
    if params.surface_character == 0u { return; }

    // Surface interaction ports are intentionally omitted for phase 2.
    // They'll be added in a later phase once the no-surface parity
    // baseline is locked in.
}
