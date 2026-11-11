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
- `connector: Option<ConnectorSystem>` — physical connector plugin (see below).

`Fabric::iterate(...)` is the per-tick physics step (Verlet). Use it; don't
reinvent.

### FabricDimensions

`src/fabric/dimensions.rs`. Carries scale, altitude, push/pull radii, joint
mass, push density, and the optional plan-level connector config. Defaults are
in `Default::default()` and are the source of truth. A fabric built from real
parts sets `with_hardware(Hardware::OPEN_CLAW)` instead: telescoping tube mass
per metre of strut, hardware at each strut and cable end, and point masses for
node plates and hinges, with no flat joint mass (Headless Hug).

### Connector (optional plugin) — `src/connector/`

All physical-connector code lives in `src/connector/`, adjacent to the
simulation — nothing there participates in the physics tick. The part itself
(spec in `docs/connectors.md`) is a flat ring turning on an axial bolt with a radial
boss ending in a cross-tube; the cable's fork pivots on a pin through that
tube. Azimuth (ring on bolt) and elevation (fork on pin) are both free, so
every connector is geometrically identical — there are no manufactured bend
angles or per-position variants:
- `dimensions.rs` — `ConnectorDimensions` (ring/washer/cap thickness, pivot
  radius) plus ring/pivot geometry (`pivot_geometry`, `ring_center`), with
  the inline `pivot_geometry_tests`.
- `attachment.rs` — `PullConnections`, slot-assignment optimiser.
- `parts.rs` — the parts list: `Part` (each distinct physical part, its
  material and frame) and `placements(fabric)` (every copy, placed);
  `parts_list` counts them as a bill of materials.
- `fork.rs` — cable-end fork terminal (clevis) estimates + capsule model
  for collision culprit marking.
- `node_plate.rs` — `NodePlate`: a triangular plate at a bridle node
  (`Fabric::bridle`, DSL `.bridle(...)`) hung on a fork rod, which two whole
  cables' terminals pin to, taking two cables off a crowded stack.
  Every plate is one equilateral part; a plate cable too short to make
  is a fork rod (`ConnectorSystem::link_length`). At a hinge pin
  (`.hinge`) a clevis and a rod end screw onto the two struts' bolts, pinned
  where their axes meet. No cable may end on a hinge: one with no strut of
  its own there is split by a plate half-way along it, a leg to each strut's
  stack (`Fabric::split_hinge_cables`). Each stack sits on a strut cap's
  5 mm tapped disc and is closed by a stock washer and lock nut.
- `dxf.rs` — DXF drawings of the laser-cut parts, for ordering.
- `system.rs` — `ConnectorSystem`: owns `ConnectorDimensions`, a
  `SecondaryMap<IntervalKey, PullConnections>` of slot assignments, and
  `culprits` (cable ends whose assemblies physically collide, marked once
  per assignment rebuild and rendered red).

Fabrics opt in per plan via `FabricDimensions::with_connector()`; the config
becomes `Fabric::connector` at construction. Those meant to be built for real
(Open Claw, Headless Hug) also set `with_connectors_shown()`, so they open with
the hardware showing, as if C were pressed. `None` (the default) means
connectors play no role anywhere: no slot assignment, no rendering, and the
C-key toggle is inert. Slot assignments and part shapes are committed when
the fabric enters Viewing (and for export) — not incrementally during
build, and not again afterwards: animation and the C key keep them.

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
- `BakingBrick(Oven)`, `Evolving(...)`, `Articulating(...)` — brick baking
  and the evolution runners.

Transitions go through `finalize_to_viewing()` which also rebuilds the
connector slot assignments (`Fabric::update_all_attachment_connections`).

### Build pipeline

DSL in `src/build/dsl/`:
- `fabric_library.rs` — named fabrics (`OpenClaw`, `Halo by Crane`, …).
- `fabric_plan.rs` + `fabric_plan_executor.rs` — phased execution
  (`Building → Pretensing → Falling → Settling → Complete`).
  Pretensing is a single-shot percentage extension: `Fabric::set_pretenst(pretenst, seconds)`
  drives every push interval through an `Approaching` span from its current
  rest length to `length × (1 + pretenst%)` over the configured duration; the
  stage advances when no intervals are still approaching.
- `plan_runner.rs` — drives Initialize → Build → Shape inside the executor.
- When pretensing starts the executor names the joints
  (`number_twins_by_build_order`, `number_axial_by_build_order`), then runs
  the plan's hinges, omits and bridles by those names. A step naming a
  joint or cable the fabric doesn't have is a `PlanStepFailed` event
  (`FabricPlanExecutor::plan_failures`); `test_all_fabrics_build` requires
  every plan to apply cleanly.

### Joint names

Symmetric fabrics name joints `<member><number>`: `L12`/`R12` (mirror),
`A12`/`B12`/`C12` (threefold), `Z<n>` on the symmetry plane or axis, and
`<host>b` for a bridle node. Numbers follow build order, never geometry, so
reshaping a fabric doesn't rename its joints. Full scheme and the code
path: `docs/joint-naming.md`.

Engineering export: there is no CSV any more (Open Claw's is archived on
an old branch). `src/fabric_json.rs` writes a fabric standing, settled under gravity, with
every member's rest length and force (physical facts only, no FEA package's
terms), and
`scripts/rfem/build_rfem.py` turns that into an RFEM model, holding every
modelling choice; see `docs/fabric-json.md`.

### Physics presets

The `presets` module in `src/fabric/physics.rs`:
- `CONSTRUCTION` — build/shape with damping.
- `PRETENSING` — zero-G pretension settling.
- `FALLING` / `SETTLING` — gravity, surface contact.
- `VIEWING` — frozen.
- `ANIMATING` — actuator-driven animation.
- `BAKING` — brick baking.

Don't tweak physics in place — pick a preset, or compose with `Tweak*` if you
need to adjust mass or rigidity multipliers at runtime.

### Rendering

`src/wgpu/`:
- `fabric_renderer.rs` — bundles the interval cylinders and the connector
  hardware.
- `cylinder_renderer.rs` — push/pull intervals as cylinders.
  - The "fabric pipeline" boilerplate is centralised in
    `Wgpu::create_fabric_pipeline`.
- `connector_renderer.rs` — connector hardware when attachment points are
  visible, drawn from the parts list (`connector::parts`): one mesh per
  `Part` (`part_meshes.rs`, metres, in the part's own frame), instanced at
  every `Placement`. A new kind of hardware is a new `Part` variant.
- `parts_view.rs` — the P key: the parts full screen, click one in the strip to
  see it large, with its mass (`Part::mass`). It takes the window, pointer and
  keyboard from the scene.
- `sky_renderer.rs`, `surface_renderer.rs`, `text_renderer.rs` — chrome.
- `shader.wgsl` — shared WGSL for fabric pipelines.

UI/render state separation:
- `src/control.rs` — UI state (`ControlState`, `RenderStyle`, `Appearance`,
  `IntervalDetails`, `JointDetails`, `PointerChange`).
- `src/events.rs` — event types (`LabEvent`, `CrucibleAction`, `StateChange`,
  `Radio`).
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
├── connector/          # Physical connector plugin (optional per fabric)
│   ├── dimensions.rs   # ConnectorDimensions, pivot_geometry, pivot_geometry_tests
│   ├── attachment.rs   # PullConnections, slot assignment
│   ├── fork.rs         # Terminal, rod fork, plate dimensions + collision capsules
│   ├── node_plate.rs   # NodePlate: the equilateral bridle plate
│   ├── parts.rs        # Parts list: Part, placements, parts_list
│   ├── strut.rs        # Telescoping tube sets each strut is made from
│   ├── welding.rs      # Welding document: seams, effort, cost estimate
│   └── system.rs       # ConnectorSystem (slots, plates, links, culprits)
│
├── fabric/
│   ├── mod.rs          # Fabric struct + main impl
│   ├── bridle.rs       # Fabric::bridle, tighten_bridles, hang_bridles
│   ├── symmetrize.rs   # symmetrize_mirror (exact mirror shape for connectors)
│   ├── dimensions.rs   # FabricDimensions (incl. optional connector config)
│   ├── interval.rs     # Interval, Role, Span
│   ├── joint.rs, joint_path.rs
│   ├── face.rs, brick.rs, material.rs
│   ├── physics.rs      # Physics struct, presets
│   ├── physics_tester.rs
│   ├── vulcanize.rs
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
├── fabric_json.rs      # A fabric as built, as JSON (Headless Hug)
├── headless_hug_package.rs # The handoff zip (test-only)
├── omission_scan.rs    # Buildability test + bridle/omission search tools
├── fabric_smoke_test.rs # Every fabric builds and its plan applies cleanly
├── open_claw_symmetry.rs # OpenClaw threefold-symmetry enforcement
├── open_claw_test.rs   # OpenClaw build/geometry tests (base triangle, feet)
├── wgpu/               # Rendering (see Rendering section)
└── physics_gpu/        # GPU compute (native; see docs/gpu-compute-backend.md)
```

## Testing

Run with release mode:

```bash
cargo test --release
```

Important integration tests:
- `src/fabric_smoke_test.rs` — `test_all_fabrics_build`: every fabric
  builds to Complete and its plan applies with no failed steps.
- `src/omission_scan.rs` — `headless_hug_is_buildable`: Headless Hug's
  connectors have no collisions and no cable too short to make. The
  ignored `greedy_headless_hug_bridles` searches for bridles that help.
- `src/open_claw_symmetry.rs` — `test_open_claw_threefold_symmetry` (asserts
  every rotational triple is symmetric in length and slot) and
  `test_open_claw_cable_triples`.
- `src/open_claw_test.rs` — `test_open_claw_base_triangle`,
  `test_open_claw_foot_positions`.
- `src/physics_gpu/parity_test.rs` — CPU vs GPU numeric parity.
- `src/connector/dimensions.rs` (inline `pivot_geometry_tests`) — derived dimension formulas.

## Entry Points

```bash
# Native:
cargo run --release -- --fabric "Halo by Crane"

# Headless Hug as built, as JSON (writes HeadlessHug-<date>.json):
cargo test --release --lib export_headless_hug_json -- --ignored

# Laser-cutting DXFs for Headless Hug (writes HeadlessHug-parts-<date>/):
cargo test --release --lib export_headless_hug_dxf -- --ignored

# Headless Hug handoff zip: JSON, RFEM script, DXFs, welding doc
# (writes HeadlessHug-<date>/ and .zip; see docs/fabric-json.md):
cargo test --release --lib export_headless_hug_package -- --ignored

# Web:
trunk serve
```

## Key Insights and Gotchas

1. **Iterations per frame is computed, not constant.** Don't hardcode iteration
   counts; let the outer loop do `time_scale × iterations_per_second / fps`.

2. **Slot assignment is committed, not incremental.** Connector slot
   assignments and part shapes are settled once on entering Viewing
   (`Crucible::finalize_to_viewing`), and for export. Toggling
   attachment points ON only re-marks collisions. Don't trigger rebuilds
   from elsewhere.

3. **Coordinate systems.** Simulation is Y-up. The JSON export converts to
   Z-up via `z_up()` in `fabric_json.rs`. The Blender import pipeline
   expects meters; see `scripts/tensegrity_fast_import.py`.

4. **Units.** Joint locations are stored in meters. The `units` newtypes
   (`Meters`, `Seconds`, `Grams`, …) catch mismatches at compile time — use
   them at boundaries, unwrap with `.f32()` only when interfacing with raw math.
   One type per quantity, holding its named unit: `Newtons` is the only
   force type, in real newtons. The simulation's masses are grams, so inside
   the tick its stiffnesses (`spring_constant`) and forces are raw `f32`
   millinewtons. Convert only where values cross that boundary, with
   `Newtons::to_millinewtons()` / `from_millinewtons()`: a conversion inside
   the tick changes the rounding, and that alone leans Headless Hug's
   settled stance by about 1° (gotcha 7).

5. **Physics presets.** Different phases need different presets. To tweak at
   runtime, send a `TweakParameter` via the radio; don't mutate presets
   directly.

6. **WASM ≠ native.** Compute shaders aren't available on WebGL. GPU physics
   (`src/physics_gpu/`) is native-only; gated by `cfg(not(target_arch = "wasm32"))`.
   Native-only state on `Application` is grouped under a single `NativeState`
   struct.

7. **Builds are deterministic; keep them so.** A settled fabric is very
   sensitive to floating-point rounding: summing in a different order once
   moved joints by up to 0.4 m. Anything that feeds the physics must not
   depend on hash-map iteration order. Even renaming joints can shift the
   settled shape slightly, since `symmetrize_mirror` visits joints by name.

8. **Re-exports at the crate root.** `lib.rs` does `pub use control::*;
   pub use events::*;` and `fabric/mod.rs` does `pub use dimensions::*;`.
   External imports use the short path (`use crate::ControlState`) but the
   types live in their domain modules.

## Contact

Project: https://github.com/elastic-interval/tensegrity-lab
Related: https://pretenst.com/
