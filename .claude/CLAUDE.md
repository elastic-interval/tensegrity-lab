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

### Sizes: Large, Medium, Small

`Size` (`src/fabric/dimensions.rs`, `--size`) says how big a fabric is made,
from one plan for all three (`FabricPlan::at_size`):
the plan is first built Large unseen up to its pretensing, once for each
fabric, so it is known how far the fabric will reach (`FabricDimensions::reach`)
and the camera keeps that far back from the start of the build, with
nothing to make up when it is done.
- **Large** — the plan as designed, with its connectors.
- **Medium** — about 1.5 m tall (`MEDIUM_HEIGHT`), built as the 2021 Headless
  Hug was: 8×1 mm aluminium tube, an M6 bolt set per strut end, 1.5 mm
  Dyneema, hinges made by hand, no connectors, bridles or node plates
  (`Hardware::MEDIUM`). It is built small from the start: how much smaller,
  and how big its table, come from building it Large unseen up to its
  pretensing, once for each fabric (`FabricPlan::at_size`). From pretensing
  on it is simulated in its own materials; it falls for less time, as
  things do the smaller they are, and is pretensioned less than its plan
  says (`MEDIUM_PRETENSION`), as a piece tied by hand is. The struts are a
  third as stiff as real: the 50 µs tick can't follow short light ones at
  full stiffness. It is built over its wobble table (`Fabric::quake`) and
  mounted on it once landed. Small and Medium start building in
  fast-forward.
- **Small** — a model to build from: the Large fabric, its lengths shown at
  `--model-scale` (1:10 by default), with caliper readings.

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
  is a fork rod (`ConnectorSystem::link_length`). A plate taking more
  than three cables (`Z(n).bridle_across(..)`) is a tube plate: the
  hanger through a hole, each cable's fork on a cross-tube welded on a
  boss (`Part::TubePlate`). At a hinge pin
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
becomes `Fabric::connector` at construction. Once built, the hardware always
shows at real scale and never on a model (`--model-scale`). `None` (the
default) means connectors play no role anywhere: no slot assignment, no
rendering. Slot assignments and part shapes are committed when
the fabric enters Viewing (and for export) — not incrementally during
build, and not again afterwards: animation keeps them.

### Time

- One iteration = 50 µs of fabric time (`TICK_DURATION` / `TICK_SECS` in `src/lib.rs`).
- `Age::iteration_duration()` returns that value; `Age::iterations_per_second()` returns its inverse (20000).
- The app runs in real time. `application.rs` steps the physics in fixed
  10 ms updates (up to 3 a frame), each `time_scale × iterations_per_second
  × 10 ms` iterations, so fabric time keeps pace with the wall clock at any
  frame rate; the frame rate is only displayed. `time_scale` is 1 except
  while a Small or Medium build is fast-forwarded (it goes back to 1 when
  the build ends), or at Large while Fast-forward is pressed (ten times,
  `FAST_FORWARD`) during the build, an animation or taking apart; Slow
  goes back to real time. A frame over 100 ms drops that time rather than
  catching up, and a frame spends at most about 25 ms on physics beyond its
  first update (`PHYSICS_BUDGET`), so fast-forward goes only as fast as the
  physics allows.
- Don't introduce hardcoded iteration counts.

### Crucible — lifecycle manager

`src/crucible.rs`. `Stage` enum:
- `RunningPlan(FabricPlanExecutor)` — runs the whole build/shape/pretense pipeline.
- `Viewing` — settled, idle.
- `Animating(Animator)` — runs DSL-defined actuators. `Animator` needs only
  a `Fabric` and `Physics`, so animations also run headless, in tests.
- `Assembling(Assembly)` — Headless Hug or Open Claw taken apart strut by
  strut on a crane hook, then put together again (`src/build/assembly.rs`,
  `docs/crane-lift.md`). While it runs `Fabric::site` is set, `None` in
  every build like the wind.
- `PhysicsTesting(PhysicsTester)` — real-time gravity test, offered only for
  fabrics without a plan (sphere, Klein, Möbius).
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
  the plan's hinges, omits and bridles by those names. Plans name joints
  by twin set, typed, each entry standing for all its images:
  `LR(2) - LR(3)`, `LR(5) | LR(2)` across the mirror, `ABC(2) >> ABC(1)`
  a sector on (`joint_names.rs`, `docs/joint-naming.md`). A step naming a
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
- `parts_view.rs` — the Parts button: the parts full screen, click one in the
  strip to see it large, with its mass (`Part::mass`). It takes the window and
  pointer from the scene.
- `button.rs` — on-screen buttons for mouse and touch: `Buttons` lays a row
  of labelled ones out and draws them, or takes any rect its owner draws
  (the parts view's strip); a button acts on release over it. The one key
  is Esc (native only), stepping back out of the parts view or a selection
  (`Application::step_back`): `ButtonBar` (`src/button_bar.rs`) registers
  every button with its label, the stages it shows in and the event it
  sends. Only the buttons that apply show. Quake only in Viewing, Stop
  quake and Demo in its place while the wobble table quakes, with no Start
  animation or Choose meanwhile. Choose (in Viewing) swaps the bar for a
  button per fabric, and Back: one builds that fabric at the size the app
  runs at; there is no Reload.
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
the in-app live-physics mode (no longer reachable from the app; see `docs/gpu-compute-backend.md`). Architecture and the
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
├── button_bar.rs       # ButtonBar: every button, its stages and event
├── pointer.rs          # Mouse/touch → PointerChange
├── caliper.rs          # Caliper readings for model-scale display
├── units.rs            # Meters/Seconds/Grams etc. newtypes
├── animation_export.rs # JSON export for Blender (native only)
├── gallery.rs          # --gallery: Medium pieces quaking in a room, walked with the arrows
├── gallery/sensor.rs   # A movement sensor on a joint: feels each turn of its swing (placed by the ignored place_sensors)
├── gallery/sound.rs    # The gallery's music: a groove played by each piece's sensors, heard near it
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
│   ├── chain.rs        # A slack cable hung as a chain so it drapes (Fabric::drape, the assembly)
│   ├── symmetrize.rs   # symmetrize_mirror (exact mirror shape for connectors)
│   ├── dimensions.rs   # FabricDimensions (incl. optional connector config)
│   ├── interval.rs     # Interval, Role, Span
│   ├── joint.rs, joint_path.rs
│   ├── face.rs, brick.rs, material.rs
│   ├── physics.rs      # Physics struct, presets
│   ├── physics_tester.rs
│   ├── vulcanize.rs
│   ├── wind.rs         # Gusting wind: drag on every member (Wind button, wind tests)
│   ├── quake.rs        # The wobble table under a Medium piece (Quake button)
│   ├── cue.rs          # DMX cues for the table's three steppers, and their evolution
│   ├── site.rs         # Building site: ground that lets go, pinned struts, crane hook (Fabric::site)
│   └── fabric_sampler.rs
│
├── build/
│   ├── animator.rs     # Animation actuators
│   ├── assembly.rs     # Headless Hug and Open Claw taken apart and put together strut by strut (Take apart button)
│   ├── oven.rs         # Brick baking
│   ├── settler.rs
│   ├── algo/           # Algorithmic generators (sphere, klein, mobius)
│   ├── evo/            # Evolution
│   └── dsl/            # Tenscript DSL builders + executors
│
├── fabric_json.rs      # A fabric as built, as JSON (Headless Hug)
├── headless_hug_package.rs # The handoff package folder (test-only)
├── bouwboek.rs         # Draft Bouwboek: fills docs/bouwboek/headless-hug.md (test-only)
├── omission_scan.rs    # Buildability test + bridle/omission search tools
├── crane_lift.rs       # Headless Hug on a crane hook: lifting eyes (test-only, docs/crane-lift.md)
├── assembly.rs         # Tests of build/assembly.rs: every step's forces (test-only, docs/crane-lift.md)
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
  `headless_hug_stands_upright`: its centre of gravity within 20 mm of the
  line between its feet, every guy line taut, and at rest when the settle
  ends. The ignored `search_headless_hug_guy_lines` retunes the guy lines.
- `src/open_claw_symmetry.rs` — `test_open_claw_threefold_symmetry` (asserts
  every rotational triple is symmetric in length and slot) and
  `test_open_claw_cable_triples`.
- `src/open_claw_test.rs` — `test_open_claw_base_triangle`,
  `test_open_claw_foot_positions`.
- `src/build/animator.rs` — `animations_run_headless`: each plan's
  animation finds its joints, moves and stays finite through one period.
- `src/fabric/wind.rs` — `headless_hug_leans_in_the_wind`; the ignored
  `measure_headless_hug_in_wind` reports apex sway, peak forces and slack
  cables with the wind from five directions. `Fabric::wind` is `None` in
  every build, so the wind never changes one.
- `src/fabric/quake.rs` — `medium_headless_hug_rides_the_wobble_table`:
  Quake pressed once, it evolves and keeps more than one cue, unlike one
  another; Demo plays them while evolving waits; pressed again the drums
  go back to their middles and the piece stands where it stood. The ignored
  `evolve_wobble_cues` (`PROBE` the fabric, `TRIES` how many) prints the
  kept cues as DMX loops for the electronics.
  The table is a hexagonal deck hung on three ties, so it sways and twists
  but can't tilt. At the other three corners (`DRUM_CORNERS`, one before
  the piece and two behind) a stepper on the base turns a drum, and a
  Dyneema loop from the drum round a pulley either side to a lug under the
  deck draws the deck along its edge (`drum_along`), either way, through a
  spring chosen for the deck to sway on them by itself at `DECK_SWAY`,
  above the piece's swings, so it follows smoothly. All three drawn alike
  twist the deck; drawn adding up to nothing, they sway it. (Twisting lifts
  the deck on its ties too, but well under a millimetre at these reaches,
  so that isn't modelled.) The deck is `ROOM` (1.3) times the
  smallest hexagon holding the feet and guy anchors, at least `SMALLEST`:
  estimated from the lowest joints before the build, fitted to where they
  land (`Quake::fit`); unless the plan chooses its table, in its own
  `scale`s so it grows with the piece (`FabricDimensions::with_table`:
  Headless Hug, Open Claw, Quadruped; the ignored `measure_tables` tells
  how far each piece's feet reach). The steppers play cues
  (`src/fabric/cue.rs`): loops of DMX frames, 44 a second, each three
  values, one a drum, 128 its middle, `PER_VALUE` of cable each (20 mm
  either way); a drum goes from one to the next no faster than
  `TOP_SPEED`. A cue is made and changed in the deck's own ways of moving,
  side, fore and twist (`Cue::drums` turns them into the drums').
  It evolves them: from seeds rocking side to side, fore and aft, on a
  diagonal and twisting, circling, swaying and twisting by turns, and
  rocking a few cycles then holding still while the piece rings on
  (`Cue::pulsing`), at a spread of speeds, by bumps in one way, stretches,
  widening, sliding one way's timing against the others, turning the sway's
  direction, and crossing (a part of the loop blended in, or one way
  whole), none the drums couldn't follow (`Cue::followable`), out of sight: each tried
  `TRYING` on a copy of the piece as it stood when Quake was pressed,
  `ROUND` side by side on their own threads (one after another on the
  web), about eight tries a second. It watches every joint, no sensors
  chosen: each is scored for the joint swinging most swinging about
  `EXPRESSIVE` against the deck, less for the drums' work (`EFFORT`) and
  for quick sway (`HASTE`, against every joint's speed over how far it
  moves), so the slower sways win; it
  keeps the best of each kind of swing (`Quake::alike` under `DISTINCT`,
  every joint's way through the loop, wherever it starts), at most
  `KEPT`, evolving on until Quake is pressed again. The piece itself only
  plays: each find (a new kind, or one `NOTABLE` better) `SHOWING`, else
  the kept in turn, `SHOWING` each, every one from rest: between cues the
  piece and the deck are put back as they stood when Quake was pressed
  (`Rest`), still. Stopping, the drums go back to their middles gently
  (`RETURNING`). Demo, shown only while it quakes,
  plays the kept in turn, `PLAYING` each, evolution waiting meanwhile, then
  evolves on. After every round that changes what is kept, the kept cues are written to
  `cues/<fabric>.txt` (native only; `quake::cues_file`), one a line with the
  DMX frames after `|`, and tried first the next time Quake is pressed, so
  evolution carries on from them. The kept cues are named A to F by where they are kept; the
  subtitle names the one playing, with what the drums are told
  (`Cue::describe`), the joint it moves most and which way (side, up,
  fore, or two of them: `R62 side+up`). While Quake runs nothing can be
  selected, and the right of the screen tells how the evolution goes
  (`Quake::report`): the round, how many tried, rounds since a find, the
  cues kept with score, swing and joint, and what plays. A cue reads:
  `A 34f side 8 twist 5` is a 34-frame loop moving the deck 8 mm either
  side and twisting it 5 mm either way at the drums.
  The piece's feet go with the deck and pull on it. It is drawn with its
  mechanism showing: at alternate corners (`HANG_CORNERS`, one behind the
  piece and two before it, so the table has its mirror symmetry and it
  faces between two) a hook drops from the deck and an arm rises from the
  base, a tie between them; at the others a lug drops from the deck, its
  loop running to the pulleys either side and down to its stepper. Only a Medium piece has one (`Fabric::quake`, there from the start
  of its build); the Quake button is shown only for Medium. Open Claw and
  Halo by Crane are built turned (`FabricDimensions::table_turn`) to stand
  a foot over each hanger.
- `src/fabric_smoke_test.rs` — `test_all_fabrics_build_medium`: every plan
  builds at Medium too, about 1.5 m tall, on its table.
- `src/quadruped.rs` — `quadruped_is_buildable`: Quadruped's connectors
  have no collisions or short cables, none slack, none past 5.1 kN;
  `quadruped_stands_on_four_feet`. Ignored tools: load path, cable forces,
  spacer sweep, connector problems, design candidates.
- `src/physics_gpu/parity_test.rs` — CPU vs GPU numeric parity.
- `src/connector/dimensions.rs` (inline `pivot_geometry_tests`) — derived dimension formulas.

## Entry Points

```bash
# Native:
cargo run --release -- --fabric "Halo by Crane"

# Medium, on the wobble table (Quake starts its motor):
cargo run --release -- --fabric "Headless Hug" --size medium

# The gallery: Open Claw, Headless Hug and Quadruped at Medium, a quarter taller, each on its
# table playing its cues in turn, evolved at its size (`cues/gallery/`, by the ignored
# `evolve_gallery_cues`; to swing about 60 mm, slowly; Open Claw keeps a fiftieth of its pretension, the others a quarter, to sway; `measure_gallery_motion` tells how each moves),
# in a dark room 18 × 22 m walked through smoothly, as on a gimbal (two-finger swipe walks and turns, pinch rises and sinks, drag looks, or the arrows), tables skirted in
# velvet, a spot in each deck throwing the piece's shadow up on the
# ceiling (shadow maps, `wgpu/shadow_renderer.rs`), each playing a groove from
# movement sensors on its joints, heard near it (`gallery/sound.rs`); Esc quits:
cargo run --release -- --gallery

# Headless Hug as built, as JSON (writes HeadlessHug-<date>.json):
cargo test --release --lib export_headless_hug_json -- --ignored

# Laser-cutting DXFs for Headless Hug (writes HeadlessHug-parts-<date>/):
cargo test --release --lib export_headless_hug_dxf -- --ignored

# Headless Hug handoff package: JSON, RFEM script, DXFs, welding doc, figures,
# Bouwboek (writes the folder HeadlessHug-<date>/; see docs/fabric-json.md):
cargo test --release --lib export_headless_hug_package -- --ignored

# The Bouwboek from that package as a PDF, beside its Markdown (needs weasyprint):
python3 scripts/bouwboek/pdf.py HeadlessHug-<date>/HeadlessHug-bouwboek-<date>.md

# Web (--release: unoptimized WebAssembly runs the physics at half speed):
trunk serve --release
```

## Key Insights and Gotchas

1. **Iterations per update are computed, not constant.** Don't hardcode
   iteration counts; let the outer loop do `time_scale ×
   iterations_per_second × update interval`, which keeps real time.

2. **Slot assignment is committed, not incremental.** Connector slot
   assignments and part shapes are settled once on entering Viewing
   (`Crucible::finalize_to_viewing`), and for export. Toggling
   the hardware on (when the fabric is built) only re-marks collisions. Don't trigger rebuilds
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
