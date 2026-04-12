// GPU compute backend for tensegrity-lab. N fabrics are stepped in
// lockstep, each occupying a "slot" in the flat buffer layout.
// global_invocation_id.y selects the slot; .x is the local index
// within that slot. Buffer access: slot * max_per_slot + local.
// Interval alpha/omega are local joint indices; the shader offsets
// them by slot * max_joints to get the global joint buffer address.

// Joint state (Group 0) — flat: [slot_0 | slot_1 | ... | slot_N]
@group(0) @binding(0) var<storage, read_write> positions: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> velocities: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> force_x: array<atomic<i32>>;
@group(0) @binding(3) var<storage, read_write> force_y: array<atomic<i32>>;
@group(0) @binding(4) var<storage, read_write> force_z: array<atomic<i32>>;
@group(0) @binding(5) var<storage, read_write> masses: array<atomic<i32>>;
@group(0) @binding(6) var<storage, read_write> frozen: array<atomic<u32>>;

// Elastic interval topology (Group 1) — flat: [slot_0 | slot_1 | ...]
@group(1) @binding(0) var<storage, read> elastic_alpha: array<u32>;
@group(1) @binding(1) var<storage, read> elastic_omega: array<u32>;
@group(1) @binding(2) var<storage, read> elastic_ideal: array<f32>;
@group(1) @binding(3) var<storage, read> elastic_k: array<f32>;
@group(1) @binding(4) var<storage, read> elastic_linear_density: array<f32>;

// Push interval topology (Group 3) — flat: [slot_0 | slot_1 | ...]
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
    max_joints: u32,
    max_elastic: u32,
    max_push: u32,
    ambient_mass: f32,
    force_scale: f32,
    ground_y: f32,
    num_slots: u32,
    speed_limit: f32,
    surface_character: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}
@group(2) @binding(0) var<uniform> params: Params;

const MASS_SCALE: f32 = 1e4;

fn slot_frozen(slot: u32) -> bool {
    return atomicLoad(&frozen[slot]) != 0u;
}

fn check_speed_limit_slot(vel: vec3<f32>, slot: u32) {
    let speed_sq = vel.x * vel.x + vel.y * vel.y + vel.z * vel.z;
    if speed_sq > params.speed_limit * params.speed_limit {
        atomicStore(&frozen[slot], 1u);
    }
}

@compute @workgroup_size(64)
fn reset_forces_and_mass(@builtin(global_invocation_id) id: vec3<u32>) {
    let local = id.x;
    let slot = id.y;
    if local >= params.max_joints { return; }
    let gj = slot * params.max_joints + local;
    atomicStore(&force_x[gj], 0);
    atomicStore(&force_y[gj], 0);
    atomicStore(&force_z[gj], 0);
    atomicStore(&masses[gj], i32(params.ambient_mass * MASS_SCALE));
}

@compute @workgroup_size(64)
fn half_kick_and_drift(@builtin(global_invocation_id) id: vec3<u32>) {
    let local = id.x;
    let slot = id.y;
    if local >= params.max_joints || slot_frozen(slot) { return; }
    let gj = slot * params.max_joints + local;

    let fx = f32(atomicLoad(&force_x[gj])) / params.force_scale;
    let fy = f32(atomicLoad(&force_y[gj])) / params.force_scale;
    let fz = f32(atomicLoad(&force_z[gj])) / params.force_scale;
    let m = f32(atomicLoad(&masses[gj])) / MASS_SCALE;

    var vel = velocities[gj];
    let inv_m = select(0.0, 1.0 / m, m > 0.0);
    vel.x += 0.5 * fx * inv_m * params.dt;
    vel.y += 0.5 * fy * inv_m * params.dt;
    vel.z += 0.5 * fz * inv_m * params.dt;

    check_speed_limit_slot(vec3<f32>(vel.x, vel.y, vel.z), slot);
    velocities[gj] = vel;

    var pos = positions[gj];
    pos.x += vel.x * params.dt;
    pos.y += vel.y * params.dt;
    pos.z += vel.z * params.dt;
    positions[gj] = pos;
}

@compute @workgroup_size(64)
fn elastic_forces(@builtin(global_invocation_id) id: vec3<u32>) {
    let local = id.x;
    let slot = id.y;
    if local >= params.max_elastic || slot_frozen(slot) { return; }
    let gi = slot * params.max_elastic + local;
    let joint_offset = slot * params.max_joints;

    let a = elastic_alpha[gi] + joint_offset;
    let o = elastic_omega[gi] + joint_offset;
    let ideal = elastic_ideal[gi];
    let k = elastic_k[gi];
    let linear_density = elastic_linear_density[gi];

    let pos_a = positions[a];
    let pos_o = positions[o];

    let dx = pos_o.x - pos_a.x;
    let dy = pos_o.y - pos_a.y;
    let dz = pos_o.z - pos_a.z;
    let length_sq = dx * dx + dy * dy + dz * dz;
    let actual = sqrt(length_sq);
    if actual < 0.0001 { return; }

    let interval_mass = linear_density * actual;
    let half_mass_i = i32(interval_mass * 0.5 * MASS_SCALE);
    atomicAdd(&masses[a], half_mass_i);
    atomicAdd(&masses[o], half_mass_i);

    let strain = (actual - ideal) / ideal;
    if strain <= 0.0 { return; }

    let ux = dx / actual;
    let uy = dy / actual;
    let uz = dz / actual;

    let extension = strain * ideal;
    let force_mag = k * extension;

    let fx = ux * force_mag;
    let fy = uy * force_mag;
    let fz = uz * force_mag;

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

@compute @workgroup_size(64)
fn push_forces(@builtin(global_invocation_id) id: vec3<u32>) {
    let local = id.x;
    let slot = id.y;
    if local >= params.max_push || slot_frozen(slot) { return; }
    let gi = slot * params.max_push + local;
    let joint_offset = slot * params.max_joints;

    let a = push_alpha[gi] + joint_offset;
    let o = push_omega[gi] + joint_offset;
    let ideal = push_ideal[gi];
    let k = push_k[gi];
    let linear_density = push_linear_density[gi];

    let pos_a = positions[a];
    let pos_o = positions[o];

    let dx = pos_o.x - pos_a.x;
    let dy = pos_o.y - pos_a.y;
    let dz = pos_o.z - pos_a.z;
    let length_sq = dx * dx + dy * dy + dz * dz;
    let actual = sqrt(length_sq);
    if actual < 0.0001 { return; }

    let interval_mass = linear_density * actual;
    let half_mass_i = i32(interval_mass * 0.5 * MASS_SCALE);
    atomicAdd(&masses[a], half_mass_i);
    atomicAdd(&masses[o], half_mass_i);

    if actual > ideal { return; }

    let ux = dx / actual;
    let uy = dy / actual;
    let uz = dz / actual;

    let strain = (actual - ideal) / ideal;
    let extension = strain * ideal;
    let force_mag = k * extension;

    let fx = ux * force_mag;
    let fy = uy * force_mag;
    let fz = uz * force_mag;

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

@compute @workgroup_size(64)
fn second_half_kick(@builtin(global_invocation_id) id: vec3<u32>) {
    let local = id.x;
    let slot = id.y;
    if local >= params.max_joints { return; }
    let gj = slot * params.max_joints + local;

    let m = f32(atomicLoad(&masses[gj])) / MASS_SCALE;
    atomicAdd(&force_y[gj], i32(-m * params.gravity * params.force_scale));

    let fx = f32(atomicLoad(&force_x[gj])) / params.force_scale;
    let fy = f32(atomicLoad(&force_y[gj])) / params.force_scale;
    let fz = f32(atomicLoad(&force_z[gj])) / params.force_scale;

    if slot_frozen(slot) { return; }

    let inv_m = select(0.0, 1.0 / m, m > 0.0);
    var vel = velocities[gj];
    vel.x += 0.5 * fx * inv_m * params.dt;
    vel.y += 0.5 * fy * inv_m * params.dt;
    vel.z += 0.5 * fz * inv_m * params.dt;

    let speed_sq = vel.x * vel.x + vel.y * vel.y + vel.z * vel.z;
    let viscosity_factor = 1.0 - speed_sq * params.viscosity * params.dt;
    vel.x *= viscosity_factor;
    vel.y *= viscosity_factor;
    vel.z *= viscosity_factor;

    let drag_factor = 1.0 - params.drag * params.dt;
    vel.x *= drag_factor;
    vel.y *= drag_factor;
    vel.z *= drag_factor;

    check_speed_limit_slot(vec3<f32>(vel.x, vel.y, vel.z), slot);
    velocities[gj] = vel;
}

@compute @workgroup_size(64)
fn ground_collision(@builtin(global_invocation_id) id: vec3<u32>) {
    let local = id.x;
    let slot = id.y;
    if local >= params.max_joints || slot_frozen(slot) { return; }
    if params.surface_character == 0u { return; }
}
