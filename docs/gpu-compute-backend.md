# GPU Compute Backend: Frozen Fabrics, Parallel Trials

**Audience:** a future Claude instance picking this up cold.
**Date written:** 2026-04-11. Revised same day after Gerald's clarification.
**Status:** Design approved by Gerald. No code written yet in this project.
**History:** Supersedes `gpu-physics-vision.md` (March 2026) and the first revision of this doc (April 2026, port-only). Both predated the decision to keep all building on the CPU and use the GPU strictly for parallel stepping of frozen fabrics.

## 1. The vision in one paragraph

Tensegrity-lab's CPU build pipeline (Tenscript DSL, brick library, oven, build/shape/pretense/converge phases) is mature and correct. Don't touch it. Build whatever fabrics you want, however you want — one fabric, a thousand mutated variants, ten unrelated structures, anything constructed by familiar CPU code — and hand the whole collection to a GPU backend that **freezes each one and steps them all forward in time in a single dispatch**. The CPU stays the authority for every kind of fabric construction and mutation. The GPU is purely a parallel stepping engine: feed it any slice of `Fabric`s and they all advance lockstep through the same passage of time. The point is speed — thousands of trials per generation, all stepping at GPU bandwidth, with no GPU-side knowledge of where the fabrics came from or whether they're related.

## 2. What was rejected, and why

Two earlier designs are explicitly retired:

- **`gpu-physics-vision.md` (March 2026)** proposed a greenfield project with SOA buffers, rigid SHAKE/RATTLE push constraints replacing the spring-push model, and GPU-side scaffold forces. This was the inspiration for the chopstix experiment. Chopstix succeeded as a *physics* experiment but its reimplementation of build/brick/face/placement drifted semantically from tensegrity-lab — OpenClaw legs went the wrong way, Single-Left and Single-Right bricks turned asymmetric. Lesson: rebuilding the build layer was the expensive mistake. Do not repeat it.
- **The first cut of this document (April 2026)** proposed a minimal port that supported live, incremental building on the GPU — `append_joints`/`append_elastic`/`append_push` mid-run, dense-index map updates, per-frame `Span::Approaching` interpolation. All of this is unnecessary if building is finished before upload. The freeze-then-ship model deletes the entire incremental-mutation API surface.

What survives from those documents: the chopstix shaders themselves (they work), the role mapping, the integrator-parity concerns, the wgpu 25→28 API churn warning, and the batch-trial section from the vision doc (now the centerpiece, not a sidebar).

## 3. Architecture

```
tensegrity-lab/src/
├── fabric/               # unchanged — CPU authority for building
├── build/                # unchanged — DSL, oven, plan executor
├── wgpu/                 # unchanged — rendering only
└── physics_gpu/          # NEW — frozen fabrics + parallel batches
    ├── mod.rs            # GpuPhysics facade
    ├── shaders/
    │   └── physics.wgsl  # ported from chopstix/src/gpu/physics.wgsl
    ├── frozen.rs         # FrozenFabric — immutable snapshot of one built fabric
    ├── batch.rs          # GpuBatch — collection of N independent FrozenFabrics on GPU
    ├── params.rs         # PhysicsParams uniform struct
    ├── readback.rs       # GPU → host: per-fabric positions, per-fabric fitness scalars
    └── parity_test.rs    # CPU vs GPU numeric parity (single fabric)
```

The data flow:

```
DSL or algorithm → Vec<Fabric>           (CPU build, oven, pretense, converge — and any
                                          mutation/variation done in familiar CPU code)
                   ↓ GpuBatch::from_fabrics(&gpu, &fabrics)
                 GpuBatch                 (per-fabric topology + state slots, padded to max)
                   ↓ step(iterations)
                 GPU compute pipeline    (Verlet: half-kick / drift / forces /
                                           second-half-kick / ground, run on every slot)
                   ↓ readback()
                 host: per-fabric positions, per-fabric fitness scalars
```

Nothing outside `physics_gpu/` imports from it except the eventual evolution driver. No call site inside `Fabric`, the oven, or the build phases ever touches the GPU module. That isolation is the regression firewall.

## 4. The freeze step

`Fabric::freeze() -> FrozenFabric` is a pure conversion applied to a single fabric. After freezing:

- **Joint set is fixed.** No additions or removals. Joints get a dense `u32` index in iteration order; the SlotMap key → index map is stored on the FrozenFabric for any later host-side lookups.
- **Interval set is fixed.** Partitioned into `push` and `elastic` buckets per the role mapping in §6.
- **All spans are concrete `f32` lengths.** `Span::Approaching` and `Span::Measuring` are resolved to their current ideal length at freeze time. No interpolation runs on the GPU. Asserting this during freeze catches misuse.
- **Per-interval `k` is precomputed.** Material stiffness × geometric factors are folded into a single scalar per interval, so the shader's Hooke law is correct for each role without knowing about materials.
- **Per-interval `half_mass` is precomputed.** Linear density × length / 2, written once.
- **Initial joint positions and velocities are captured.** This is the seed state the batch copy of this fabric starts from.

The freeze is destructive only in the sense that you should not keep mutating the source `Fabric` after freezing it. Cloning is fine. Each variant in a batch is frozen independently — there is no shared ancestor on the GPU.

## 5. The collection step

`GpuBatch::from_fabrics(&gpu, &[Fabric]) -> GpuBatch` accepts an arbitrary slice of CPU-side fabrics and uploads them as a single batch. They can be:

- **N copies of one fabric.** Identical topology, identical state. Trivial special case — the GPU treats them like any other batch. If you want them to diverge, perturb something on the CPU first.
- **N mutated variants of one parent.** Produced by familiar CPU code: cable shortening, strut swapping, role changes, structural edits, joint additions/removals — anything the existing `Fabric` API can do. Each variant is just a regular `Fabric` after the edit; the GPU never sees that it was a variant.
- **N entirely unrelated fabrics.** Different topologies, different sizes, different histories. Also fine. The GPU stepper doesn't know or care.

Internally, `from_fabrics` calls `Fabric::freeze()` on each entry and uploads all of them into a flat per-fabric layout. **Each fabric carries its own topology** — there is no shared-topology optimization, because mutations are expected to change connectivity in the general case (see §13).

**Buffer layout (pad to max):**
- **Per-fabric topology slot:** dense joint indices for each interval, role bucket, ideal length, `k`, `half_mass`. Sized to the largest fabric in the batch; smaller fabrics leave the tail unused.
- **Per-fabric state slot:** positions (vec3), velocities (vec3), forces (atomic int, accumulated and reset each iteration), accumulated mass. Same pad-to-max sizing.
- **Per-fabric metadata:** actual joint count and interval count for this slot. The shader uses these to early-out for padded threads.
- **Per-fabric fitness output:** scalar(s) the trial writes (e.g., final altitude, max strain over the run, displacement from initial centroid).

**The lockstep guarantee.** Every dispatch advances every fabric in the batch by the same number of iterations. They all experience the same passage of time. There are no cross-fabric reads, no synchronization between slots — every slot is an independent simulation that just happens to share a dispatch with N-1 others. If a particular fabric in the batch is the same as another, it produces the same result; if it's wildly different, that's also fine. The GPU module has no opinion.

**No `Mutator` API.** Mutations live entirely in the CPU build/edit code that already knows how to manipulate `Fabric`s. The GPU module has nothing to say about how fabrics were constructed; it just steps whatever it's handed.

**Memory cost.** At N=1000, a typical 2000-interval / 500-joint fabric, pad-to-max with ~10% headroom: ~70 MB total. Trivial for any modern GPU. Cache hit rate on the per-slot topology buffers is lower than the shared-topology version would have been, but the per-fabric data still fits comfortably in L2 for typical sizes.

Compute dispatches use a 2D workgroup layout: one dimension over batch slots, one over joints (or intervals) within a slot. The shader does `global_idx = slot_idx * max_joints + local_joint_idx` and early-outs if `local_joint_idx >= per_slot_joint_count[slot_idx]`. Off-by-one bugs here are silent and produce convincing-looking garbage; the bring-up test in phase 4 (a batch of N identical fabrics, every slot must match the single-fabric run) catches them.

## 6. Role mapping

Chopstix shaders know two interval kinds: **elastic** (slack when compressed) and **push** (symmetric spring, no slack, no rigid SHAKE). Tensegrity-lab's 9 roles map as:

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

Material stiffness differences are baked into the per-interval `k` at freeze time. The shader doesn't know about materials. Rigid SHAKE/RATTLE is **not** ported — pushes are springs, matching the CPU semantics exactly.

## 7. Source material (paths are authoritative)

### In chopstix (`/Users/fluxe/RustroverProjects/chopstix`)
- `src/gpu/physics.wgsl` — single WGSL file, multiple entry points. Workgroup size 64. Port these passes: `half_kick_and_drift`, `elastic_forces`, `second_half_kick`, `ground_collision`, `push_forces`. Skip `shake_constraints`, `rigid_mass`, `rattle_constraints` (rigid path, not used here).
- `src/gpu/growable.rs` — owns all GPU buffers, bind groups, pipelines. Primary port target. Adapt the buffer layout to add the batch dimension.
- `src/gpu/physics.rs` — older `PhysicsCompute` wrapper. Don't port; `GrowablePhysics` is the better starting point.
- Skip the `TensegritySphereBuffers` family entirely.

### In tensegrity-lab (`/Users/fluxe/RustroverProjects/tensegrity-lab`)
- `src/fabric/mod.rs:377-390` — `Fabric` struct.
- `src/fabric/mod.rs:590-670` — `Fabric::iterate(physics)`. CPU reference for parity testing.
- `src/fabric/joint.rs:63-69` — `Joint`.
- `src/fabric/interval.rs:486-496` — `Interval`.
- `src/fabric/interval.rs:316-331` — `Span::{Fixed, Approaching, Measuring}`.
- `src/fabric/interval.rs:371-381` — `Role` enum with discriminant values.
- `src/fabric/physics.rs` — `Physics` struct (drag, gravity, surface, dt) maps onto the `PhysicsParams` uniform.
- `src/wgpu/mod.rs:78-88` — `Wgpu` struct, single device + queue. The compute pipelines share this device.
- `Cargo.toml:27-28, 38` — `wgpu = "=28.0.0"`. Chopstix is on wgpu 25; expect API churn.

## 8. Phased plan

Each phase ends with a commit and `cargo test --release`. No phase proceeds until the previous is green.

### Phase 0 — Scaffolding (half a day)
Create `src/physics_gpu/` behind `cfg(not(target_arch = "wasm32"))`. Empty module. `cargo build` clean.

### Phase 1 — Port shaders, single-copy (1-2 days)
Copy chopstix's `physics.wgsl` and `growable.rs`. Adapt for wgpu 28. Single-copy mode (batch size = 1). Headless test: tiny fabric, dispatch, read back positions. Just proving the port runs.

### Phase 2 — `FrozenFabric` adapter (1 day)
Implement `Fabric::freeze()` and `FrozenFabric` upload to a single-copy `GpuBatch`. Walk joints in SlotMap order; partition intervals by role; resolve all spans to fixed lengths; precompute `k` and `half_mass`.

### Phase 3 — Parity harness (1 day)
**This phase defines success.** Build a known fabric (start: a single baked brick). Clone into `fabric_cpu` and `fabric_gpu`. Step both 500 iterations. Compare joint positions pairwise; assert max error < 1e-3. Then OpenClaw, Column3, Triped. Until single-brick parity passes, no batch work.

### Phase 4 — Batch dimension, identical fabrics (2 days)
Generalize the GPU buffers to N slots in pad-to-max layout. Implement `GpuBatch::from_fabrics` accepting a slice of `Fabric`s. First test: hand it N **identical** fabrics. Verify every slot produces the same result, equal to the single-fabric run from phase 3. This validates the slot-indexing arithmetic without introducing per-slot variation as a second variable.

### Phase 5 — Batch dimension, varied fabrics (1-2 days)
Hand `GpuBatch::from_fabrics` a slice of **different** fabrics — start with two unrelated fabrics in one batch (e.g., a brick and a small claw), then a parent fabric plus several CPU-mutated variants. For each slot, run the same fabric independently on CPU and verify the GPU slot's final positions match within tolerance. This proves padding, per-slot metadata, and per-slot topology indexing all work.

### Phase 6 — Fitness readback (1 day)
Per-fabric scalar output buffers. Shader writes simple per-trial scalars (final altitude, max strain over the run, displacement from initial centroid — exact metric TBD with Gerald). Host reads back and ranks. This is the minimal evolution loop.

### Phase 7 — Wire into evolution driver (2-3 days)
Replace `evolution.rs`'s per-trial CPU loop with a batched GPU pass when a runtime flag (`TENSEGRITY_GPU=1`) is set. The driver builds and mutates fabrics in CPU code as it does today, then submits the whole population as one `GpuBatch::from_fabrics` call per generation. Keep the CPU path as default and reference. Side-by-side test on a small population.

### Phase 8 — Cleanup and docs
Update this doc with phase outcomes, the wgpu 25→28 changelist, and any divergences. Decide the fate of chopstix.

## 9. Zero-regression principles

Carried forward from the previous revision, still non-negotiable:

1. `Fabric::iterate()` is not modified. CPU reference path stays.
2. `Joint`, `Interval`, `Role`, `Span` struct layouts are not modified.
3. `physics_gpu/` is additive. Nothing else imports from it (until phase 7, behind a flag).
4. The oven stays CPU. Baked bricks are calibrated to the CPU integrator; changing it invalidates them.
5. The render loop is not restructured.
6. Existing tests stay green at every phase boundary.
7. No "while we're here" refactors.
8. Numeric parity (≤1e-3 max per-joint position error after 500 iterations) is the acceptance gate before any batch or mutation work.

## 10. Known risks

1. **Integrator order parity.** CPU does half-kick → drift → reset → forces → half-kick-2 (with damping + surface). Chopstix shaders look the same but subtle ordering differences (when damping multiplies, when gravity adds) will cause divergence. Phase 3 will surface this. Fix the shader to match CPU, not the other way around.
2. **Atomic int force accumulation.** Chopstix uses `atomic<i32>` force buffers with a `force_scale` quantization. Bounded numeric error vs CPU `f32` accumulation. This is why parity tolerance is ~1e-3, not machine epsilon.
3. **Drag formulation.** Verify whether CPU uses exponential or linear damping and match it.
4. **Surface interaction.** Run parity tests with `surface = None` until the chopstix `surface_character` mapping is verified against tensegrity-lab's `Surface` enum.
5. **Accumulated mass.** CPU recomputes `Joint::accumulated_mass` from incident intervals each iteration. The freeze step precomputes this once per fabric (since topology is fixed within a slot). Verify the values match.
6. **`Joint::frozen` semantics.** CPU sets `Fabric::frozen` when max velocity exceeds a threshold. In a batch this becomes per-slot state. Decide whether one slot freezing halts the dispatch or just that slot — leaning toward "just that slot" so one diverging trial doesn't poison the whole generation.
7. **wgpu 25 → 28 API churn.** Chopstix is on `wgpu = "0.25"`, tensegrity-lab on `=28.0.0`. Breaking changes to `Features`, `Limits`, `PollType`/`Maintain`, buffer usage flags. Budget time in phase 1.
8. **Compute and render share the device.** Use separate command encoders, submit compute first then render. Don't share encoders between passes.
9. **Slot indexing arithmetic.** With pad-to-max layout `[slot_0 | slot_1 | ... | slot_N]`, every shader access becomes `slot_idx * max_joints + local_joint_idx`, plus an early-out against the per-slot metadata count. Off-by-one errors are silent and produce convincing-looking garbage. Phase 4's "N identical fabrics, all slots equal" test catches them.
10. **Pad-to-max waste.** If one fabric in a batch is much larger than the rest, padding wastes memory and dispatch threads. For typical evolutionary populations the variance is small (±10-20% from a parent). If variance ever gets large enough to matter, swap to an offset-table layout — but not in this port.

## 11. Open questions for Gerald

1. **What's a trial fitness scalar?** Phase 6 needs at least one concrete metric. Final altitude? Survives N seconds without freezing? Distance traveled? Deviation from a target shape? You'll know better than I do which one matches the evolutionary work you have in mind.
2. **How big is N in practice?** 100? 1000? 10000? GPU memory is not the constraint at any of these for typical fabrics, but it shapes the workgroup layout decisions in phase 4.
3. **How much fabric-size variance per batch?** If most batches are "one parent + N variants ±10% in size," pad-to-max is a clear win. If batches routinely mix tiny and huge fabrics, an offset-table layout becomes worth doing earlier.
4. **What gets visualized during a batch run?** One representative slot? A wireframe overlay of all slots? Nothing — just final fitness scores? Affects the readback strategy.
5. **Compile-time feature flag or runtime env var?** Lean runtime, default off.
6. **Fate of `../chopstix`?** Archive, scratch pad, or delete?

## 12. Live GPU physics in the application

Press **G** while in **Viewing** mode to switch the current fabric to GPU-accelerated physics. This is a one-way transition: the fabric's positions are now driven by the GPU compute pipeline, and the CPU fabric becomes a read-only mirror updated via position readback each frame.

### How it works

1. `GpuBatch::parallelize` uploads the settled fabric's joints, intervals, and physics to the GPU (using the main wgpu device shared with the renderer).
2. Each frame, `batch.step()` runs the GPU compute pipeline for the iteration count demanded by the current time scale.
3. `batch.read_positions()` copies joint positions back to CPU (~1.5 KB for an OpenClaw).
4. The application writes these positions into `Joint::location` on the CPU fabric.
5. The renderer picks up the updated locations and draws as usual.

### What goes stale after the switch

Once GPU physics is active, the following CPU-side fields are **not updated** because they are computed inside `Fabric::iterate()` or `Interval::iterate()` which no longer runs:

| Field | Where | Impact |
|---|---|---|
| `Interval::strain` | `interval.rs` | Interval strain display (click-to-inspect) shows the last CPU value, not the current GPU-computed strain |
| `Interval::unit` | `interval.rs` | Direction vector used by CPU force calculation; irrelevant since forces are now on GPU |
| `Fabric::stats` (IterationStats) | `fabric/mod.rs` | Max speed, average strain, kinetic energy — all stale |
| `Fabric::age` | `fabric/mod.rs` | The CPU age counter stops ticking; GPU time is tracked only by iteration count |
| `Joint::velocity` | `joint.rs` | CPU velocities are stale; GPU has the real velocities but they aren't read back (only positions are) |
| `Joint::force` | `joint.rs` | Same — forces live on GPU only |
| `Joint::accumulated_mass` | `joint.rs` | Mass accumulation happens on GPU; CPU value is stale |

**For interactive viewing** (watching the structure move, camera control, rotation), none of these matter — rendering depends only on `Joint::location`.

**For detailed inspection** (clicking an interval to see strain, reading stats), the stale values would be misleading. A future enhancement could recompute strains from readback positions on the CPU, or read back velocities alongside positions. Neither is needed for the initial GPU experience.

### Returning to CPU physics

There is no "switch back" — once GPU physics is active, the CPU fabric's velocities, forces, and mass state are stale. Pressing **Enter** (rebuild fabric) clears the GPU batch and rebuilds from scratch on the CPU.

## 13. Out of scope

- GPU baking. Oven stays CPU.
- GPU-side building, brick assembly, or DSL execution. CPU does all of this.
- Mid-run topology changes. Parallelize means freeze topology.
- Rigid push (SHAKE/RATTLE). Spring-push only, matching CPU.
- wasm/webgl backend for GPU physics. Native first.
- Replacing the CPU path. CPU is reference, forever.
- Refactoring `Fabric`, `Joint`, `Interval`, `Role`, `Span`, or the oven.

## 14. Where this doc came from

Written by Claude (Opus 4.6) on 2026-04-11. The doc went through three drafts in one session as Gerald refined what he actually wanted:

1. **Draft 1 (incremental-build port).** A minimal port of chopstix's GPU shaders that supported live, incremental building on the GPU — `append_joints`, dense-index map updates, per-frame `Span::Approaching` interpolation. Discarded once Gerald clarified that all building stays on the CPU.
2. **Draft 2 (freeze + N copies of one fabric).** Centered on a single `FrozenFabric` with N parallel copies sharing topology buffers, perturbed by a `Mutator`. Discarded once Gerald pointed out that mutations will routinely change interval structure, so depending on shared topology is unsafe in the general case.
3. **Draft 3 (this one).** The GPU module accepts an arbitrary slice of CPU-built `Fabric`s and steps them all in lockstep. Each fabric carries its own topology in its own slot. There is no `Mutator` API — mutation is whatever the CPU code does to a `Fabric` before handing it over. Identical fabrics in a batch are a degenerate special case, not a privileged one.

The earlier `gpu-physics-vision.md` (March 2026) was deleted once it was clear it had been superseded. Its rigid-push and SOA-rebuild ideas led to the chopstix experiment, which produced excellent compute shaders but a drifted build layer. The shaders are what we keep; the rebuild is what we don't repeat.
