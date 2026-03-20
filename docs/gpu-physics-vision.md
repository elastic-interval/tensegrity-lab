# GPU-Accelerated Tensegrity Physics: Architecture & Bootstrap Guide

This document is a design spec for a standalone GPU-accelerated tensegrity physics engine. It is intended to be read by a future Claude instance (or human) who will bootstrap a new Rust project that cannibalizes parts of `tensegrity-lab`. The new project replaces the CPU physics with GPU compute shaders while introducing rigid push intervals as a first-class concept.

## Why a New Project

The current `tensegrity-lab` serves a real-world construction project. Its physics must remain elastic -- physical struts do flex, and the simulation must match what gets built. But for design exploration, evolution, and interactive experimentation, the elastic spring model for push intervals is:

1. **Physically misleading** -- Push intervals in tensegrity never touch each other. They're suspended entirely by tension cables. Modeling them as very stiff springs (k = 2e10 N/m) that barely deform anyway is an approximation that costs computation for negligible benefit.
2. **CPU-bound** -- The current single-threaded Verlet integration limits structure size and evolution speed.
3. **Iteration-hungry** -- Stiff springs demand small time steps to remain stable, wasting cycles on near-zero deformation.

The new project keeps pull intervals elastic (cables genuinely stretch) but treats push intervals as rigid geometric constraints. This is both more faithful to the physics and dramatically cheaper to compute, especially on GPU.

---

## Part 1: What to Cannibalize from tensegrity-lab

### Take Whole

| Module | Path | Why |
|--------|------|-----|
| **Units system** | `src/units.rs` | `Meters`, `Grams`, `Seconds`, `NewtonsPerMeter` -- type-safe dimensional analysis, essential |
| **Material definitions** | `src/fabric/material.rs` | Spring constants, linear densities for Pull/Spring materials. Push material constants become mass-only (no spring constant needed for rigid). |
| **EIG documentation** | `docs/EIG.md` | Core philosophy document |
| **Scaffold forces design** | `docs/scaffold.md` | Implicit strut forces for arrangement -- directly applicable to GPU |
| **Connector/hinge geometry** | `docs/connectors.md`, `src/fabric/attachment.rs` | Physical construction geometry. Keep for export, not for physics. |
| **Evolution framework** | `src/build/evolution.rs`, `docs/evolution.md` | `Genome`, `FitnessDimension`, `PopulationStrategy`, `IntervalController` traits. The framework is simulation-agnostic. |

### Take and Adapt

| Module | Path | What changes |
|--------|------|--------------|
| **Fabric struct** | `src/fabric/mod.rs` | Replace `SlotMap` with SOA (struct-of-arrays) for GPU upload. Keep joint/interval topology. |
| **Interval** | `src/fabric/interval.rs` | Split into `RigidInterval` (push) and `ElasticInterval` (pull). No more unified `iterate()`. |
| **Joint** | `src/fabric/joint.rs` | Simplify -- GPU manages position/velocity arrays directly. Joint struct becomes metadata only. |
| **Physics presets** | `src/fabric/physics/` | Keep preset concept. Surface interaction moves to GPU. Damping/drag parameters stay. |
| **DSL/Tenscript** | `src/build/dsl/` | Keep the build language. It produces topology, which the GPU engine then simulates. |
| **CSV export** | `src/fabric/csv_export.rs` | Keep for physical construction output. Reads from CPU-side copy of settled state. |

### Leave Behind

| Module | Why |
|--------|-----|
| **WGPU rendering** (`src/wgpu/`) | The new project can use the same GPU device for both compute and rendering, but the render pipeline should be redesigned around the SOA data layout. |
| **Brick baking** (`src/build/oven.rs`) | Baking uses the old elastic physics to stabilize prototypes. With rigid push, brick stability is geometric, not dynamic. |
| **Approaching spans** | Rigid intervals don't "approach" a length -- they ARE a length. Elastic pull intervals may still approach during pretensing. |

---

## Part 2: Rigid Push Intervals

### The Physics

In a tensegrity structure, push intervals (compression struts) are suspended in space entirely by pull intervals (tension cables). No two push intervals share a joint. This means:

- Push intervals experience no compression deformation in the idealized model
- Their length is a geometric fact, not a spring equilibrium
- The only forces they transmit are constraint forces that maintain their length

This is exactly the setup for **holonomic constraints** in classical mechanics.

### Constraint Formulation

For a rigid push interval connecting joints A and B with ideal length L:

```
|B - A|² = L²
```

This constraint is enforced using SHAKE (position correction) and RATTLE (velocity projection), borrowed from molecular dynamics:

**Position correction (SHAKE):**
```
Given: current positions A, B
       ideal length L
Compute: d = B - A, current_length = |d|, unit = d / current_length
         error = current_length - L
Correct: A += unit * error/2
         B -= unit * error/2
```

**Velocity projection (RATTLE):**
```
Given: velocities vA, vB, constraint unit vector
Compute: relative_vel = vB - vA
         vel_along_constraint = relative_vel · unit
Correct: vA += unit * vel_along_constraint/2
         vB -= unit * vel_along_constraint/2
```

### Why Single-Pass is Sufficient

In tensegrity, **no two push intervals share a joint**. Every joint connects to exactly one push interval and one or more pull intervals. This means rigid constraints are independent -- correcting one constraint doesn't violate another. A single pass over all rigid intervals is exact.

This is a massive simplification compared to general rigid-body constraint solving (which requires iterative solvers like Gauss-Seidel). It's one of the structural gifts of tensegrity.

### Mass Still Matters

Rigid intervals still have physical mass (they're aluminum tubes). Mass is distributed to endpoint joints:
```
interval_mass = push_linear_density * length
joint[A].mass += interval_mass / 2
joint[B].mass += interval_mass / 2
```

This mass affects gravity, inertia, and damping. The interval just can't change length.

### Strain is Zero by Definition

Rigid intervals always have strain = 0. This flows correctly into:
- Statistics (avg strain, max strain)
- Energy calculations (no elastic potential energy stored in push intervals)
- Color-by-strain rendering (push intervals are neutral)

---

## Part 3: GPU Architecture

### Data Layout: Struct of Arrays

The CPU uses `SlotMap<JointKey, Joint>` (array-of-structs). GPU compute shaders need struct-of-arrays for coalesced memory access:

```rust
// CPU side -- mirrors GPU buffers
struct FabricBuffers {
    // Joint data (N joints)
    position_x: Vec<f32>,      // GPU buffer
    position_y: Vec<f32>,
    position_z: Vec<f32>,
    velocity_x: Vec<f32>,
    velocity_y: Vec<f32>,
    velocity_z: Vec<f32>,
    force_x: Vec<f32>,
    force_y: Vec<f32>,
    force_z: Vec<f32>,
    mass: Vec<f32>,            // Accumulated mass (recomputed each frame)

    // Elastic interval data (M elastic intervals: pull, spring, etc.)
    elastic_alpha: Vec<u32>,   // Joint index
    elastic_omega: Vec<u32>,   // Joint index
    elastic_ideal: Vec<f32>,   // Ideal length
    elastic_k: Vec<f32>,       // Spring constant (precomputed: k_base / ideal * rigidity * stiffness)
    elastic_strain: Vec<f32>,  // Output: computed strain
    elastic_is_pull: Vec<u32>, // 1 if pull-like (slack when compressed), 0 if spring (never slack)

    // Rigid interval data (P push intervals)
    rigid_alpha: Vec<u32>,     // Joint index
    rigid_omega: Vec<u32>,     // Joint index
    rigid_length: Vec<f32>,    // Constraint length
    rigid_half_mass: Vec<f32>, // Precomputed: linear_density * length / 2
}
```

### Compute Pipeline: One Frame

Each frame dispatches multiple compute passes. The number of iterations per frame is calculated dynamically: `iterations = target_time_scale * 20000 / FPS`.

For each iteration within the frame:

```
Pass 1: HALF_KICK (joints)
    v += 0.5 * (F / m) * dt

Pass 2: DRIFT (joints)
    x += v * dt

Pass 3: RESET_FORCES (joints)
    F = 0
    m = ambient_mass

Pass 4: ELASTIC_FORCES (elastic intervals)
    For each elastic interval:
        compute actual length, strain
        if slack: strain = 0, force = 0
        else: force = k * strain * ideal_length
        atomicAdd force to alpha joint (+direction)
        atomicAdd force to omega joint (-direction)
        atomicAdd half_mass to both joints

Pass 5: RIGID_MASS (rigid intervals)
    For each rigid interval:
        atomicAdd rigid_half_mass to alpha and omega joints
    (No force computation -- that's the whole point)

Pass 6: GRAVITY (joints, conditional)
    if has_surface:
        F.y -= m * 9.81

Pass 7: SECOND_HALF_KICK + DAMPING (joints)
    v += 0.5 * (F / m) * dt
    apply drag and viscosity
    surface interaction (if applicable)

Pass 8: RIGID_CONSTRAINTS (rigid intervals)
    SHAKE: correct positions to maintain constraint length
    RATTLE: project out velocity along constraint axis
    (Single pass -- constraints are independent in tensegrity)
```

### Why Not One Big Shader?

The passes above map naturally to separate compute dispatches because of data dependencies:
- Forces must be accumulated before the second kick
- Positions must be updated before forces are recomputed
- Constraints must be enforced after velocities are updated

Each pass is embarrassingly parallel within its domain (all joints independent, all elastic intervals independent, all rigid intervals independent).

### Atomic Operations for Force Accumulation

Multiple intervals write forces to the same joint. On GPU, this requires atomic operations:
```wgsl
// In WGSL compute shader
atomicAdd(&force_x[alpha_idx], force_component_x);
atomicAdd(&force_y[alpha_idx], force_component_y);
// etc.
```

WGSL supports `atomicAdd` on `atomic<i32>` and `atomic<u32>`. For `f32`, use the standard trick: reinterpret as `i32`, atomicCompareExchangeWeak in a loop. Alternatively, use fixed-point arithmetic (multiply by 1e6, accumulate as integers, divide back).

### Buffer Sizes and Transfer

For a structure with 500 joints and 2000 intervals (typical large tensegrity):

| Buffer | Size | Direction |
|--------|------|-----------|
| Joint positions | 6 KB | GPU → CPU (for rendering, per frame) |
| Joint velocities | 6 KB | GPU internal |
| Joint forces | 6 KB | GPU internal |
| Joint masses | 2 KB | GPU internal |
| Elastic intervals | 40 KB | CPU → GPU (once, or on topology change) |
| Rigid intervals | 20 KB | CPU → GPU (once, or on topology change) |
| Strain output | 8 KB | GPU → CPU (for stats/rendering, per frame) |

Total GPU memory: ~90 KB. Trivial. The win is not memory but parallelism -- 2000 force calculations per iteration, 1000 iterations per frame = 2 million force calculations per frame, all parallel.

### Iteration Batching

Running 1000 separate dispatches per frame is expensive due to dispatch overhead. Instead, batch iterations within a single dispatch using a loop in the shader:

```wgsl
@compute @workgroup_size(256)
fn iterate_batch(@builtin(global_invocation_id) id: vec3<u32>) {
    let joint_idx = id.x;
    if (joint_idx >= num_joints) { return; }

    for (var iter = 0u; iter < iterations_per_dispatch; iter++) {
        // All passes for this joint, with workgroupBarrier() between passes
        half_kick(joint_idx);
        workgroupBarrier();
        drift(joint_idx);
        workgroupBarrier();
        // ... etc
    }
}
```

**Caveat**: `workgroupBarrier()` only synchronizes within a workgroup, not across workgroups. For global synchronization between passes, you still need separate dispatches or use the multi-pass approach with storage buffers. The practical approach: batch 10-50 iterations per dispatch, dispatch 20-100 times per frame. Profile and adjust.

---

## Part 4: Scaffold Forces on GPU

The scaffold system (see `docs/scaffold.md`) arranges push intervals in 3D space before cables are added. This is a perfect GPU workload.

### Strut-Strut Interaction

For N push intervals, there are O(N²) potential interactions (but only nearby pairs actually interact, filtered by sphere of influence). On GPU:

```
Pass: SCAFFOLD_FORCES (rigid intervals x rigid intervals)
    For each pair of rigid intervals within sphere of influence:
        Compute line-segment distance
        Apply proximity repulsion force to endpoints
        Apply perpendicularity torque to endpoints
```

**Approach A (physics-level forces)** from `scaffold.md` maps directly to GPU: the force computation is a parallel operation over strut pairs.

**Approach B (scaffold springs)** also works: create temporary elastic intervals between strut endpoint pairs, and the existing elastic force pass handles them.

On GPU, Approach A is preferable because:
- No dynamic interval creation/deletion (topology changes require buffer reallocation)
- The N² pair check is naturally parallel
- Sphere-of-influence filtering reduces actual work

### Spatial Hashing for Pair Finding

To avoid O(N²) brute force, use a spatial hash grid on GPU:
1. Hash each strut endpoint into grid cells
2. For each strut, check only struts in nearby cells
3. This is a well-studied GPU algorithm (used in SPH fluid simulation)

For typical tensegrity structures (10-1000 struts), brute force N² is fine. Spatial hashing becomes necessary only at very large scales.

---

## Part 5: Evolution on GPU

The evolution framework (`docs/evolution.md`) runs trials: express genome → simulate → evaluate fitness. Currently each trial runs single-threaded on CPU. With GPU physics:

### Batch Trials

Run M trials simultaneously on GPU. Each trial gets its own slice of the joint/interval buffers:

```
Joint buffer: [trial_0_joints | trial_1_joints | ... | trial_M_joints]
Interval buffer: [trial_0_intervals | trial_1_intervals | ... | trial_M_intervals]
```

The compute shaders operate identically -- they just index into offset regions. A single dispatch processes all trials in parallel.

For 100 trials of 200-joint structures: 20,000 joints, 80,000 intervals. Still well within GPU capacity.

### Controller Execution

The `IntervalController` trait currently runs on CPU (`react()` returns an `Option<f32>` target length). For GPU evolution, encode controllers as data:

```rust
struct GpuController {
    interval_idx: u32,
    waveform: u32,      // Sine, Square, Triangle, etc.
    frequency: f32,
    amplitude: f32,
    phase: f32,
    // Sensor inputs (which interval's strain to read)
    sensor_interval: u32,
    sensor_gain: f32,
}
```

A GPU pass applies all controllers each iteration, reading sensor values from the strain buffer and writing target lengths to the elastic interval ideal-length buffer.

---

## Part 6: Pretensing with Rigid Push

In the current system, pretensing works by extending push interval ideal lengths and letting the spring forces redistribute tension. With rigid push intervals:

### The Mechanism Changes

Instead of:
1. Set push span to `Approaching { start, target, duration }`
2. Spring forces gradually pull structure into new equilibrium

It becomes:
1. Set push `Rigid { length }` to the new target length directly
2. Constraint enforcement immediately repositions joints
3. Pull intervals (still elastic) respond to the new geometry

This is actually simpler and faster. No waiting for spring equilibrium -- the push interval IS the new length on the next iteration, and cables adjust.

### Symmetric Group Extension

The current pretensing algorithm (`fabric_plan_executor.rs`) extends push intervals by symmetric groups -- all struts at the same topological depth get extended together. This works identically with rigid intervals:

```
For each symmetric group needing extension:
    For each rigid interval in the group:
        rigid.length += increment
    // Constraint enforcement handles the rest
```

The `PretenseStage::Measuring` / `PretenseStage::Extending` cycle simplifies: there's no `Approaching` span to wait for. Extend, then immediately measure the resulting pull strains.

---

## Part 7: Bootstrap Instructions

### For the Claude Instance Building This

You are building a new Rust project: a GPU-accelerated tensegrity physics engine. Here's how to start:

#### Step 1: Project Skeleton

```bash
cargo init tensegrity-gpu
cd tensegrity-gpu
# Add dependencies
cargo add wgpu pollster glam bytemuck
```

Core crates:
- `wgpu` -- GPU compute and rendering
- `glam` -- Vec3 math (CPU side)
- `bytemuck` -- Safe transmute for GPU buffer upload
- `pollster` -- Async runtime for wgpu initialization

#### Step 2: Copy Foundation

From `tensegrity-lab`, copy and adapt:
1. `src/units.rs` -- Whole file, it's self-contained
2. `src/fabric/material.rs` -- Remove `Push` spring constant (rigid intervals don't need it), keep linear density and `Pull`/`Spring` constants
3. `src/fabric/physics/` -- Keep `Physics` struct and presets, adapt for GPU parameter passing

#### Step 3: Define Data Model

```rust
// Topology (CPU-side, immutable during simulation)
struct Topology {
    joint_count: usize,
    elastic_intervals: Vec<(u32, u32, f32, f32)>,  // (alpha, omega, ideal, k)
    rigid_intervals: Vec<(u32, u32, f32)>,           // (alpha, omega, length)
}

// State (GPU buffers)
struct SimulationState {
    joint_positions: wgpu::Buffer,   // vec3<f32> * N
    joint_velocities: wgpu::Buffer,  // vec3<f32> * N
    joint_forces: wgpu::Buffer,      // vec3<f32> * N
    joint_masses: wgpu::Buffer,      // f32 * N
    // ... interval buffers
}
```

#### Step 4: Write Compute Shaders

Start with a single shader file containing all passes. Each pass is a separate entry point:

```wgsl
@group(0) @binding(0) var<storage, read_write> pos_x: array<f32>;
@group(0) @binding(1) var<storage, read_write> pos_y: array<f32>;
// ... etc

struct Params {
    dt: f32,
    gravity: f32,
    drag: f32,
    num_joints: u32,
    num_elastic: u32,
    num_rigid: u32,
    has_surface: u32,
}

@group(1) @binding(0) var<uniform> params: Params;

@compute @workgroup_size(256)
fn half_kick(@builtin(global_invocation_id) id: vec3<u32>) { ... }

@compute @workgroup_size(256)
fn drift(@builtin(global_invocation_id) id: vec3<u32>) { ... }

@compute @workgroup_size(256)
fn elastic_forces(@builtin(global_invocation_id) id: vec3<u32>) { ... }

@compute @workgroup_size(256)
fn rigid_constraints(@builtin(global_invocation_id) id: vec3<u32>) { ... }
```

#### Step 5: Validate Against tensegrity-lab

Build the same structure in both engines. Compare:
- Final joint positions (should match within tolerance)
- Pull interval strains (should match)
- Push interval strains (should be ~0 in GPU engine vs ~1e-8 in CPU engine)
- Total energy over time

The DSL system from tensegrity-lab produces topology. Write a converter that takes a `FabricPlan`, runs the build phase on CPU (or port the build phase too), then hands the resulting topology to the GPU engine for pretensing and simulation.

#### Step 6: Add Rendering

The GPU already has joint positions in a buffer. Rendering is a second pipeline on the same device:
- Instance-draw cylinders between interval endpoints
- Instance-draw spheres at joint positions
- Read position buffer directly -- no CPU round-trip needed for rendering

This is a major advantage of GPU physics: the data is already on the GPU where the renderer needs it.

### Key Constants to Preserve

| Constant | Value | Meaning |
|----------|-------|---------|
| Iteration duration | 50 microseconds | Fundamental time quantum |
| Earth gravity | 9.81 m/s² | Applied when surface exists |
| Pull spring constant (at 1m) | 6.7e9 N/m | Dyneema cable stiffness |
| Spring constant (at 1m) | 9e4 N/m | Soft spring/actuator |
| Push linear density | 3000 g/m | Aluminum tube mass |
| Pull linear density | 50 g/m | Dyneema cable mass |
| Joint ambient mass | 2280 g | Connector hardware mass |
| Mass scale exponent | 3.5 | `mass *= scale^3.5` during apply_scale |
| Target time scale (build) | 5.0 | 5x speedup during construction |
| Target time scale (realtime) | 1.0 | Physics testing |
| Iterations formula | `scale * 20000 / FPS` | Dynamic iteration count |

### Key Physics to Preserve

**Slack detection**: Pull intervals go slack (zero force) when compressed. Push intervals go slack when stretched. Springs (Role::Springy) are never slack -- they push and pull.

**Spring constant scales with 1/L**: `k(L) = k_at_1m / L`. A 2m cable is half as stiff as a 1m cable. This models real material behavior (longer rope stretches more under same force).

**Damping model**: Two independent terms:
- `drag`: scales velocity directly (`v *= 1 - drag * dt`)
- `viscosity`: additional damping factor

**Surface interaction**: At y <= 0, behavior depends on SurfaceCharacter (Frozen, Sticky, Bouncy, Slippery). Keep this on GPU -- it's per-joint, per-iteration.

---

## Part 8: What This Enables

### Larger Structures
CPU limits: ~500 joints at interactive rates. GPU: 50,000+ joints at the same frame rate. This enables:
- Full-scale architectural tensegrity simulation
- Dense cable networks without performance penalty
- Real-time interaction with large structures

### Faster Evolution
With batch GPU trials, evolution that currently takes hours can run in minutes. Population sizes of 1000+ become practical, enabling:
- More thorough exploration of structural space
- Complex fitness landscapes (multi-objective optimization)
- Real-time interactive evolution

### Scaffold at Scale
The implicit strut forces from `scaffold.md` scale quadratically with strut count. On GPU, even O(N²) pair interactions are fast for N < 10,000. This enables:
- Automatic arrangement of hundreds of struts
- Real-time scaffold force visualization
- Evolutionary discovery of novel tensegrity topologies

### Rigid + Elastic Hybrid
The split between rigid push and elastic pull creates a natural computation hierarchy:
- Rigid constraints: exact, single-pass, cheap
- Elastic forces: approximate (spring model), parallel, standard
- The two don't interfere -- rigid constraints are enforced after elastic forces

This hybrid is more numerically stable than the current all-elastic approach, where the stiff push springs (k = 2e10) fight with soft pull springs (k = 6.7e9) and require small time steps to stay stable.
