# GPU Compute Backend: Integration Plan

**Audience:** a future Claude instance picking this up cold.
**Date written:** 2026-04-11 (absolute, not relative).
**Status:** Design approved by Gerald. No code written yet in this project.
**Prior art to read first:** [gpu-physics-vision.md](gpu-physics-vision.md) — earlier vision document (March 2026) that may overlap or predate this plan. Read it, then treat this doc as the current source of truth for the near-term port.

## 1. Context

Tensegrity-lab is mature and works beautifully: the CPU Fabric + build/DSL/brick/oven/face pipeline produces correct, settled structures (Column, OpenClaw, Triped, etc.) and has been tuned over years. A sibling project `../chopstix` was started as a GPU-physics experiment. Chopstix reimplemented the build/brick/placement/face layer from scratch to drive its GPU compute shaders, and that reimplementation has drifted from tensegrity-lab's semantics. OpenClaw in chopstix builds with legs going the wrong direction, Single-Left and Single-Right bricks behave asymmetrically, etc. — bugs in drift, not in the GPU shaders.

The shaders themselves work correctly for chopstix's Sphere/Klein/Möbius demos. They are the valuable output of the chopstix experiment.

**The plan:** port chopstix's GPU compute path into tensegrity-lab, where the correct CPU semantics already live. Tensegrity-lab gains an optional, opt-in GPU physics backend; chopstix's redundant build/brick tree is discarded (or, at worst, kept as a scratch pad for shader iteration).

## 2. Zero-regression principles — non-negotiable

Gerald's explicit ask: tensegrity-lab is mature and the integration must not regress any existing behavior. Interpret this strictly:

1. **`Fabric::iterate()` is not modified.** It remains the CPU reference path. Every existing fabric (sphere, brick, claw, triped, evo) continues to run on it by default.
2. **`Joint`, `Interval`, `Role`, `Span` struct layouts are not modified.** No added fields, no reordering, no derived traits added that might affect existing consumers. The GPU layer reads these structures; it does not redesign them.
3. **The GPU backend is additive and opt-in.** New module `src/physics_gpu/` sibling to `src/wgpu/`, `src/fabric/`, `src/build/`. Existing code does not import from it. Selection happens at the call site, not inside `Fabric`.
4. **The Oven does not switch to GPU in this port.** The current `baked_bricks.rs` values are calibrated to tensegrity-lab's CPU integrator. Changing the integrator under the oven invalidates every baked brick — a catastrophic regression. Baking stays CPU.
5. **The render loop is not restructured.** When GPU physics is active, a compute pass is added *before* the render pass using the same device/queue (`src/wgpu/mod.rs:78-88`). When GPU physics is inactive, nothing changes.
6. **Existing tests must stay green at every step.** Run `cargo test` before committing each phase. No phase should modify any existing `*_test.rs` file in `src/build/dsl/` or `src/fabric/` except to add new tests alongside.
7. **No "while we're here" refactors.** Every line of pre-existing code touched is a regression vector. If a refactor is tempting, defer it to a follow-up PR after parity is proven.
8. **Numeric parity is the acceptance criterion.** The port is "done" when a Fabric stepped on GPU for N frames matches the same Fabric stepped on CPU for N frames, within a tolerance set by CPU vs GPU float semantics and integer-force-atomic quantization (expect ~1e-3 relative on positions, not machine-epsilon). Until that test passes, the GPU backend is not wired into any default code path.

## 3. Architectural shape

```
tensegrity-lab/src/
├── fabric/               # unchanged — CPU authority
├── build/                # unchanged — oven still calls fabric.iterate()
├── wgpu/                 # unchanged — rendering only
└── physics_gpu/          # NEW — everything GPU-physics lives here
    ├── mod.rs            # GpuPhysics facade
    ├── shaders/
    │   └── physics.wgsl  # ported from chopstix/src/gpu/physics.wgsl
    ├── growable.rs       # ported from chopstix/src/gpu/growable.rs
    ├── params.rs         # PhysicsParams uniform struct
    ├── upload.rs         # Fabric → GPU buffer adapter
    ├── readback.rs       # GPU → Fabric position sync
    └── parity_test.rs    # CPU vs GPU numeric comparison harness
```

Nothing outside `physics_gpu/` imports from `physics_gpu/` initially except an explicit opt-in test or a feature-gated alternate path in `application.rs`. That's the isolation firewall.

## 4. Source material in chopstix (paths are authoritative)

All paths relative to `/Users/fluxe/RustroverProjects/chopstix`. These files contain the code to port.

### Shaders
- `src/gpu/physics.wgsl` — single WGSL file, multiple entry points. Workgroup size 64. Entry points:
  - `half_kick_and_drift` (line 62)
  - `shake_constraints` (line 90) — rigid strut length correction, *not needed* for tensegrity-lab's port (tensegrity-lab uses spring pushes, not rigid struts — see §7)
  - `elastic_forces` (line 121) — Hooke's law, goes slack when compressed
  - `rigid_mass` (line 166) — also *not needed* initially
  - `second_half_kick` (line 177) — velocity update, gravity, drag, force reset
  - `rattle_constraints` (line 213) — also *not needed*
  - `ground_collision` (line 252) — surface interaction
  - `push_forces` (line 309) — spring-based push, no slack check

### Rust
- `src/gpu/growable.rs` (lines 27-90: struct; full file ~650 lines) — owns all GPU buffers, bind groups, pipelines. Exposes `append_joints`, `append_elastic`, `append_push`, `update_counts`, `dispatch`, `copy_positions_to_staging`, `read_positions`. **This is the primary port target.**
- `src/gpu/physics.rs` (lines 93-117: `PhysicsCompute`) — earlier wrapper used for sphere/klein/mobius. `GrowablePhysics` is the newer, more general equivalent; port `GrowablePhysics`, not `PhysicsCompute`.
- `src/gpu/mod.rs` — module layout, mostly re-exports.
- `src/gpu/` — also holds `TensegritySphereBuffers` and friends, specific to the non-tenscript demos. **Do not port these.** They're not relevant to Fabric-backed physics.

### Uniform buffer (`PhysicsParams`)
`src/gpu/growable.rs:6-24` mirrors the WGSL `Params` struct at `src/gpu/physics.wgsl:28-45`. 16 fields, 64 bytes, includes `dt`, `gravity`, `drag`, `ambient_mass`, `force_scale`, `ground_y`, `speed_limit`, `surface_character`, and per-pass counts. Keep the struct layout byte-identical to the WGSL side during the port — bytemuck will enforce this.

## 5. Tensegrity-lab targets (paths are authoritative)

All paths relative to `/Users/fluxe/RustroverProjects/tensegrity-lab`.

- `src/fabric/mod.rs:377-390` — `Fabric` struct (joints/intervals/faces in SlotMaps).
- `src/fabric/mod.rs:590-670` — `Fabric::iterate(physics)`. This is the CPU reference. The GPU path must produce numerically equivalent output. Read the half-kick/drift/force/second-half-kick/damping sequence carefully.
- `src/fabric/joint.rs:63-69` — `Joint { path, location, force, velocity, accumulated_mass }`.
- `src/fabric/interval.rs:486-496` — `Interval { alpha_key, omega_key, role, material, span, unit, strain, stiffness, connections }`.
- `src/fabric/interval.rs:316-331` — `Span::{Fixed, Approaching, Measuring}`.
- `src/fabric/interval.rs:371-381` — `Role::{Pushing=0, Pulling=1, Springy=2, Circumference=3, BowTie=4, FaceRadial=5, Support=6, GuyLine=9, PrismPull=11}`.
- `src/fabric/physics.rs` — `Physics` struct (drag, gravity, surface, timestep). This maps onto `PhysicsParams`.
- `src/wgpu/mod.rs:78-88` — `Wgpu` struct, single `wgpu::Device` + `wgpu::Queue`. Physics GPU pipelines live here too, sharing the device/queue.
- `src/scene.rs:164-237` — render loop + command encoder pattern. The compute pass gets injected here when GPU physics is active.
- `src/application.rs:575-586` — per-frame physics loop (up to 3 `fabric.iterate()` calls per render frame). GPU activation switches this branch.
- `src/build/oven.rs:183-259` — baking loop calls `fabric.iterate()` directly. **Do not touch in this port** (see principle #4).
- `Cargo.toml:27-28, 38` — `wgpu = "=28.0.0"`, with `webgl` feature for wasm. Chopstix is on wgpu 25. **Expect breaking API changes between 25 and 28.** Validate during phase 1.

## 6. Phased port plan

Each phase ends with a commit and `cargo test`. No phase proceeds until the previous is green.

### Phase 0 — Scaffolding (half a day)
- Create `src/physics_gpu/` module, add to `lib.rs` behind a `cfg(not(target_arch = "wasm32"))` gate or feature flag. (wasm later; don't bite it now.)
- Add `physics_gpu/mod.rs` with an empty `GpuPhysics` struct.
- Confirm `wgpu` 28 compiles the chopstix 25-era shader syntax (almost certainly yes; WGSL is stable).

### Phase 1 — Port `GrowablePhysics` verbatim (1-2 days)
- Copy `chopstix/src/gpu/physics.wgsl` → `src/physics_gpu/shaders/physics.wgsl` unchanged.
- Copy `chopstix/src/gpu/growable.rs` → `src/physics_gpu/growable.rs`, adapt imports only.
- Port the `PhysicsParams` uniform struct.
- Upgrade any wgpu 25 → 28 API differences. Likely: `Features`, `Limits`, texture/buffer descriptor field renames, `wgpu::PollType::Wait` vs `wgpu::Maintain::Wait`, pipeline compilation options. Keep a list of changes; if they're non-trivial, add a short note to this doc.
- Add a unit test in `src/physics_gpu/` that creates a headless device (pattern: `wgpu::Instance::request_adapter` with `None` surface), uploads a tiny fabric (e.g., single-push + two joints), dispatches N iterations, reads back positions. No tensegrity-lab code involved — just proving the port compiles and runs in isolation.

### Phase 2 — Adapter: Fabric → GPU snapshot (1 day)
- `src/physics_gpu/upload.rs`: function `fn upload_fabric(gpu: &mut GpuPhysics, fabric: &Fabric) -> FabricGpuHandle`.
- Walk `fabric.joints` in SlotMap iteration order; build a dense `JointKey → u32` map and an `[f32; 4]` position array and `[f32; 4]` velocity array. Call `gpu.append_joints`.
- Walk `fabric.intervals`; partition by role:
  - `Role::Pushing` → push buffer.
  - **All other roles** (`Pulling`, `Springy`, `Circumference`, `BowTie`, `FaceRadial`, `Support`, `GuyLine`, `PrismPull`) → elastic buffer. They all share the "pull-like, slack when compressed" semantics (verify against `Interval::is_pull_like` at `src/fabric/interval.rs`).
- For each interval: `alpha/omega` become the dense joint indices. `ideal` comes from `Span::Fixed.length` or `Span::Approaching.target_length` or similar (see next bullet). `k` comes from material stiffness × fabric dimensions.
- **`Span::Approaching` and `Span::Measuring`:** do NOT try to run these on the shader in phase 2. Handle them on the CPU side — each frame, before dispatching, recompute the approaching interpolated length and `gpu.write_elastic_ideal_at(...)` into the per-interval ideal buffer. `Span::Measuring` (vulcanize) is rare; assert it's absent in phase 2 and add support later.
- Store the joint-key ↔ GPU-index map in `FabricGpuHandle` for readback.

### Phase 3 — Readback + parity harness (1 day)
- `src/physics_gpu/readback.rs`: function `fn sync_positions_back(handle: &FabricGpuHandle, gpu: &GpuPhysics, fabric: &mut Fabric)` that writes GPU positions back into `Joint::location` (and velocities into `Joint::velocity` if desired).
- `src/physics_gpu/parity_test.rs`: the acceptance harness.
  - Build a known Fabric (start with a single baked brick — `SingleTwistLeft` from `baked_bricks.rs`, placed at origin as a seed).
  - Clone it into two Fabrics (`fabric_cpu`, `fabric_gpu`).
  - CPU: run `fabric_cpu.iterate(physics)` 500 times.
  - GPU: upload `fabric_gpu` to GPU, dispatch 500 iterations, read back into `fabric_gpu`.
  - Compare `Joint::location` pairwise. Assert max per-joint position error < 1e-3 (tune empirically — start loose, tighten as issues surface).
  - Also compare interval strains (recomputed from final positions on both sides).
  - Start with the smallest possible fabric (1 push + 3 pulls + 6 joints) and work up.
- **This phase defines success for the whole port.** Until parity passes on the single-brick case, do not proceed to §4 (Claw).

### Phase 4 — Scale up (1 day)
- Parity test on OpenClaw after the build has completed on CPU (i.e., use CPU to build the fabric, then upload the completed fabric and step both). This decouples "does GPU stepping match CPU stepping?" from "does building produce the same fabric?".
- Parity test on Column3.
- Parity test on Triped.

### Phase 5 — Live stepping + incremental building (2 days)
- Hook GPU physics into the application loop. Add a runtime flag (CLI arg or env var) `TENSEGRITY_GPU=1`. When set, swap the per-frame `fabric.iterate()` call for the GPU path.
- Handle incremental building: when the oven/builder adds new joints or intervals mid-run, call `gpu.append_joints` / `append_elastic` / `append_push` and update the handle's dense-index map.
- Critical: the oven's 60-iteration settle loop inside baking stays CPU. GPU stepping is only active for the main application loop outside baking.
- Side-by-side visual test: launch with `TENSEGRITY_GPU=0` and `TENSEGRITY_GPU=1` on OpenClaw. They should look identical.

### Phase 6 — Cleanup and docs (half a day)
- Update this doc with phase-outcome notes, wgpu 25→28 API change list, known divergences and their causes.
- Add a short section to the project README about the backend flag (only if users need it).
- Decide fate of `../chopstix`: archive, delete, or keep as shader scratch pad.

## 7. Role mapping: chopstix shaders vs tensegrity-lab semantics

Chopstix shaders know only two interval kinds: **elastic** (slack when compressed) and **push** (symmetric spring, no slack). Tensegrity-lab has 9 roles. The mapping for phase 2:

| tensegrity-lab Role | chopstix bucket | notes |
|---|---|---|
| `Pushing` | push | struts |
| `Pulling` | elastic | primary cables |
| `Springy` | elastic | soft cables |
| `Circumference` | elastic | face perimeter |
| `BowTie` | elastic | vulcanize diagonal |
| `FaceRadial` | elastic | centroid→corner radials |
| `Support` | elastic | external |
| `GuyLine` | elastic | external |
| `PrismPull` | elastic | prism radials |

This mapping is lossy in one way: tensegrity-lab's materials distinguish stiffness between these roles, and the shaders don't know about material. The CPU adapter must bake material stiffness into the per-interval `k` value when uploading, so the shader's Hooke-law force is already correct for that role. Confirm by checking `src/fabric/material.rs` and how `Fabric::iterate` folds material into force magnitude.

Chopstix also has SHAKE/RATTLE constraint entry points for rigid struts. **Tensegrity-lab's pushes are springs, not rigid struts.** Skip SHAKE/RATTLE entirely. Only port the spring-push path (`use_spring_push = true` in chopstix terminology).

## 8. Known risks and traps

1. **Integrator order parity.** Tensegrity-lab's `Fabric::iterate` does: half-kick → drift → force reset → force compute → half-kick-2 (with damping + surface). Chopstix's shaders do: half-kick+drift → elastic_forces → push_forces → second_half_kick (with drag+gravity) → ground_collision. These look like the same Velocity Verlet pattern but subtle ordering differences (e.g., when damping is applied, whether drag multiplies before or after gravity is added) will cause divergence. **Phase 3 will surface this.** Expect to spend time here. The fix is to make the shader match CPU, not the other way around.
2. **Atomic int force accumulation.** Chopstix's shaders use `atomic<i32>` force buffers with a `force_scale` quantization. This is why forces are accumulated as integers: so multiple shader threads can atomically add to the same joint without races. The quantization introduces a small numeric error vs CPU's f32 accumulation. This is bounded and fine, but it's why parity tolerance is ~1e-3, not machine-epsilon.
3. **Drag formulation.** Tensegrity-lab uses `velocity *= exp(-drag * dt)` style exponential damping (verify). Chopstix may use linear damping. Match whatever the CPU side does.
4. **Surface interaction.** Tensegrity-lab supports rich `Surface` types (`src/fabric/physics.rs`). Chopstix has `surface_character: u32` with values 0-4 (absent/bouncy/frozen/sticky/slippery). The mapping is probably clean but needs verification before phase 5. Until verified, parity tests should run with `surface = None`.
5. **Accumulated mass.** Tensegrity-lab's `Joint::accumulated_mass` is computed each iteration from incident intervals. Chopstix does the same via the `rigid_mass` entry point — but that's for rigid struts. For spring-push mode, chopstix computes half-mass at interval creation time and writes it to `push_half_mass_buffer`. The adapter needs to compute this from material × length × linear_density on the CPU side before upload and keep it in sync if intervals change.
6. **Span::Approaching.** Not a shader concern — handled by CPU code updating the GPU's ideal buffers per frame. But the adapter must iterate all approaching intervals each frame and call `write_elastic_ideal_at` / `write_push_ideal_at`. That's a hot path; make sure it's not per-frame slow.
7. **Span::Measuring.** Vulcanize bow ties. Rare. Assert absent in phase 2 and revisit.
8. **`Joint::frozen` semantics.** Chopstix has a `frozen_buffer` that freezes simulation when any joint exceeds speed_limit. Tensegrity-lab has `Fabric::frozen: bool` set when max velocity > 1000 m/s (`src/fabric/mod.rs:~660`). Map these; they're close but not identical.
9. **wgpu 25 → 28 API churn.** Chopstix is on `wgpu = "0.25"` (verify from its `Cargo.toml`), tensegrity-lab is on `=28.0.0`. Between these are breaking changes to `Features`, `Limits`, `PollType`/`Maintain`, buffer usage flags, possibly pipeline layout descriptors. Budget a day for this in phase 1.
10. **Do not break the existing `wgpu/` module.** Compute and render share the `wgpu::Device`/`wgpu::Queue`. Use separate command encoders per pass (chopstix's pattern) and submit them in order: compute first, render second. Do not try to share an encoder between compute and render passes.

## 9. Verification strategy summary

At every phase boundary, confirm all three of:

1. **`cargo test` still green** — every existing tensegrity-lab test passes unchanged.
2. **`cargo build --release` clean** — no new warnings.
3. **Phase-specific parity** — phase 1: isolated port compiles and runs; phase 3: single brick CPU vs GPU match within tolerance; phase 4: OpenClaw/Column/Triped match; phase 5: visual side-by-side at runtime.

If any of these regresses, stop. Do not "fix forward." Revert the phase's changes, investigate, then re-apply.

## 10. Out of scope for this port

- GPU baking (running the oven on GPU). Explicitly deferred — it would invalidate baked_bricks.rs calibration.
- wasm/webgl support for GPU physics. Phase 1 is native only.
- Replacing the CPU path. The CPU path is the reference; it stays forever.
- Refactoring `Fabric`, `Joint`, `Interval`, `Role`, `Span`, or the oven. None of these need to change for this port to work.
- Porting chopstix's Sphere/Klein/Möbius demos. Those live in chopstix's `src/tensegrity.rs` / `src/sphere.rs` / etc. and use a different buffer setup (`TensegritySphereBuffers`). Not relevant.
- Fixing chopstix's `build/brick/face/executor` drift. That code is being retired, not fixed.

## 11. Open questions for Gerald

Pose these early, before phase 1 starts:

1. Should GPU physics be a compile-time feature flag (`--features gpu`) or a runtime env var (`TENSEGRITY_GPU=1`)? Runtime is more flexible; compile-time is safer for regression isolation. I lean runtime with a default of `false`.
2. How close does "parity" need to be in practice? I proposed 1e-3 max per-joint position error after 500 frames. If Gerald has a specific visual or numeric criterion (e.g., "settled strain within 1%"), use that instead.
3. For wasm: is this a near-term goal, or strictly native-first? Plan assumes native-first.
4. What's the fate of the chopstix repo after this port completes? Archive, scratch pad, or delete?

## 12. Where this doc came from

Written by Claude (Opus 4.6) on 2026-04-11 in a chopstix session after:
- Fixing chopstix's brick baking to apply tensegrity-lab's strain formula (`ideal = actual / (1 + strain)`) across `brick.rs`, `placement.rs`, `face.rs`, and the build tests. That fix is in chopstix's `dev` branch.
- Discovering that the OpenClaw legs still build incorrectly in chopstix, localizing the cause to placement_transform and face-winding drift in chopstix's reimplementation.
- Gerald's decision to stop fixing chopstix's semantic drift and instead port the GPU work into tensegrity-lab.

The next instance should start by reading this doc, then [gpu-physics-vision.md](gpu-physics-vision.md), then `/Users/fluxe/.claude/projects/-Users-fluxe-RustroverProjects-chopstix/memory/MEMORY.md` (auto-memory pointers include reference to this project relationship and Gerald's testing preferences). Then confirm with Gerald before starting Phase 0.