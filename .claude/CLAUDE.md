# Tensegrity Lab - Claude Code Context

## Working Principles

Behavioral guidelines to reduce common LLM coding mistakes. These bias toward caution over speed. For trivial tasks, use judgment.

### 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them — don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

### 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

### 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it — don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: every changed line should trace directly to the user's request.

### 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:

```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

**These guidelines are working if:** fewer unnecessary changes in diffs, fewer rewrites due to overcomplication, and clarifying questions come before implementation rather than after mistakes.

---

## Project Overview

Tensegrity Lab is a Rust application for designing, simulating, and physically
building tensegrity structures using Elastic Interval Geometry (EIG).
Tensegrities are spatial systems of compression elements (struts) and tension
elements (cables) that maintain shape through balanced push-pull forces.

**Dual targets:**

- **Native (WGPU)** — design focus, file watching, GPU compute, animation export.
- **Web (WASM)** — build/inspection focus. WebGL backend, no compute shaders.

## Core Architecture

### Fabric

`src/fabric/mod.rs` defines `Fabric` — the central data structure:
- `joints` (SlotMap): position, velocity, mass.
- `intervals` (SlotMap): push/pull connections with role, stiffness, strain.
- `faces` (SlotMap): triangular surfaces.
- `age`: simulated time (one tick = 50 µs).
- `dimensions: FabricDimensions` (see `src/fabric/dimensions.rs`).

`Fabric::iterate(...)` is the per-tick physics step (Verlet). Use it; don't
reinvent.

### FabricDimensions and HingeDimensions

`src/fabric/dimensions.rs` defines both. `FabricDimensions` carries scale,
altitude, pull-radius, hinge geometry, joint mass, push density. `HingeDimensions`
is a sub-struct for the physical hinge mechanism (push radius, disc/cap
thicknesses, hinge hole, bend-magnitude inventory). Defaults are in their
`Default::default()` impls and are the source of truth.

### Time

- One iteration = 50 µs of fabric time (`TICK_DURATION` / `TICK_SECS` in `src/lib.rs`).
- `Age::iteration_duration()` returns that value; `Age::iterations_per_second()` returns its inverse (20000).
- `iterations_per_frame` is computed live each frame in `application.rs`:
  `time_scale × iterations_per_second / fps`.
- Don't introduce hardcoded iteration counts.

### Crucible — lifecycle manager

`src/crucible.rs`. `Stage` enum:
- `RunningPlan(FabricPlanExecutor)` — runs the whole build/shape/pretense pipeline.
- `Viewing` — settled, idle.
- `Animating(Animator)` — runs DSL-defined actuators.
- `PhysicsTesting(PhysicsTester)` — real-time gravity test.

Transitions go through `finalize_to_viewing()` which also recomputes hinge bend
magnitudes (see `Fabric::recompute_bend_magnitudes`).

### Build pipeline

DSL in `src/build/dsl/`:
- `fabric_library.rs` — named fabrics (`OpenClaw`, `Triped`, `Halo by Crane`, …).
- `fabric_plan.rs` + `fabric_plan_executor.rs` — phased execution
  (`Building → ZeroGPretensing → Falling → Settling → GravPretensing → Complete`).
  Pretensing iteratively extends symmetric groups of pushes until target
  compression is reached. The shared loop logic lives in
  `FabricPlanExecutor::pretense_step`.
- `plan_runner.rs` — drives Initialize → Build → Shape inside the executor.

Snapshot moments (`SnapshotMoment` in `src/events.rs`): `Slack`, `Pretenst`,
`Settled`, `GravPretenst`. Engineer-facing CSVs are exported at these
moments; see `docs/csv-handoff.md`.

### Physics presets

`src/fabric/physics/presets.rs`:
- `CONSTRUCTION` — build/shape with damping.
- `PRETENSING` — zero-G pretension settling.
- `FALLING` / `SETTLING` — gravity, surface contact.
- `VIEWING` — frozen.
- `PHYSICS_TEST` — real-time (1:1) gravity test.

Don't tweak physics in place — pick a preset, or compose with `Tweak*` if you
need to adjust mass or rigidity multipliers at runtime.

### Rendering

`src/wgpu/`:
- `cylinder_renderer.rs` — push/pull intervals as cylinders.
  - The "fabric pipeline" boilerplate is centralised in
    `Wgpu::create_fabric_pipeline`; cylinder and hinge renderers both use it.
- `hinge_renderer.rs` — connector geometry when attachment points visible.
- `sphere_renderer.rs` — joints.
- `sky_renderer.rs`, `surface_renderer.rs`, `text_renderer.rs` — chrome.
- `shader.wgsl` — shared WGSL for fabric pipelines.

UI/render state separation:
- `src/control.rs` — UI state (`ControlState`, `RenderStyle`, `Appearance`,
  `IntervalDetails`, `JointDetails`, `PointerChange`).
- `src/events.rs` — event types (`LabEvent`, `CrucibleAction`, `StateChange`,
  `SnapshotMoment`, `Radio`).
- Both are re-exported at the crate root so `use crate::ControlState` etc.
  still work.

### GPU compute (native only)

`src/physics_gpu/` parallelises CPU-built fabrics on the GPU for evolution and
the in-app live-physics mode (G key in Viewing). Architecture and the
WASM/WebGPU caveat are documented in `docs/gpu-compute-backend.md`.

## File Structure

```
src/
├── lib.rs              # Module declarations, Age, RunStyle, re-exports
├── main.rs             # CLI entry (native)
├── application.rs      # Main event loop, time scaling, native side-effects
├── control.rs          # UI state types
├── events.rs           # Event types, Radio
├── crucible.rs         # Lifecycle (Stage enum + transitions)
├── crucible_context.rs # Bundles fabric/physics/radio for inner code
├── camera.rs           # Camera (spherical, pick/zoom/approach)
├── scene.rs            # Scene = renderers + camera + render style
├── keyboard.rs         # Key bindings
├── pointer.rs          # Mouse/touch → PointerChange
├── caliper.rs          # Caliper readings for model-scale display
├── units.rs            # Meters/Seconds/Grams etc. newtypes
├── animation_export.rs # JSON export for Blender (native only)
│
├── fabric/
│   ├── mod.rs          # Fabric struct + main impl
│   ├── dimensions.rs   # FabricDimensions, HingeDimensions, hinge_geometry
│   ├── interval.rs     # Interval, Role, Span
│   ├── joint.rs, joint_path.rs
│   ├── face.rs, brick.rs, material.rs
│   ├── physics.rs      # Physics struct, presets
│   ├── physics_tester.rs
│   ├── attachment.rs   # PullConnections, HingeBend, attachment points
│   ├── bend_optimizer.rs  # K-center optimiser for bend magnitudes
│   ├── vulcanize.rs
│   ├── csv_export.rs   # Engineering CSV format
│   └── fabric_sampler.rs
│
├── build/
│   ├── animator.rs     # Animation actuators
│   ├── oven.rs         # Brick baking
│   ├── settler.rs
│   ├── algo/           # Algorithmic generators (sphere, klein, mobius)
│   ├── evo/            # Evolution
│   └── dsl/            # Tenscript DSL builders + executors
│
├── wgpu/               # Rendering (see Rendering section)
└── physics_gpu/        # GPU compute (native; see docs/gpu-compute-backend.md)
```

## Testing

Run with release mode:

```bash
cargo test --release
```

Important integration tests:
- `src/build/dsl/plan_runner_test.rs` — `test_open_claw_base_triangle`,
  `test_open_claw_foot_positions`, `test_triped_*`.
- `src/physics_gpu/parity_test.rs` — CPU vs GPU numeric parity.
- `src/fabric/hinge_geometry_tests` (inline in `mod.rs`) — derived dimension formulas.
- `src/fabric/bend_optimizer.rs` (inline tests) — k-center DP correctness.

## Entry Points

```bash
# Native:
cargo run --release -- --fabric "Halo by Crane"
cargo run --release -- --fabric "Open Claw" --snapshot all

# Web:
trunk serve
```

## Key Insights and Gotchas

1. **Iterations per frame is computed, not constant.** Don't hardcode iteration
   counts; let the outer loop do `time_scale × iterations_per_second / fps`.

2. **Convergence is build, not testing.** Hinge bend magnitudes are recomputed
   on entering Viewing (`Crucible::finalize_to_viewing`). Don't trigger that
   from elsewhere.

3. **Coordinate systems.** Simulation is Y-up. CSV export converts to Z-up via
   `sim_to_csv()` in `csv_export.rs`. The Blender import pipeline expects
   meters; see `scripts/tensegrity_fast_import.py`.

4. **Units.** Joint locations are stored in meters. The `units` newtypes
   (`Meters`, `Seconds`, `Grams`, …) catch mismatches at compile time — use
   them at boundaries, unwrap with `.f32()` only when interfacing with raw math.

5. **Physics presets.** Different phases need different presets. To tweak at
   runtime, send a `TweakParameter` via the radio; don't mutate presets
   directly.

6. **WASM ≠ native.** Compute shaders aren't available on WebGL. GPU physics
   (`src/physics_gpu/`) is native-only; gated by `cfg(not(target_arch = "wasm32"))`.
   Native-only state on `Application` is grouped under a single `NativeState`
   struct.

7. **Re-exports at the crate root.** `lib.rs` does `pub use control::*;
   pub use events::*;` and `fabric/mod.rs` does `pub use dimensions::*;`.
   External imports use the short path (`use crate::ControlState`) but the
   types live in their domain modules.

## Contact

Project: https://github.com/elastic-interval/tensegrity-lab
Related: https://pretenst.com/
