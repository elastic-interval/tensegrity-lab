# GPU Compute Backend: Frozen Fabrics, Parallel Trials

**Audience:** a future Claude instance picking this up cold.
**Date written:** 2026-04-11. Revised same day after the project lead's clarification.
**Status:** Implemented. `src/physics_gpu/` exists with `mod.rs`, `batch.rs`, `params.rs`, `parity_test.rs`, `smoke_test.rs`, `sphere_sweep_test.rs`, and `shaders/physics.wgsl`. Live GPU physics in the application (§12) is wired up behind the `G` key in Viewing mode. Parity tests and a sphere-sweep test are passing.
**History:** Supersedes `gpu-physics-vision.md` (March 2026) and the first revision of this doc (April 2026, port-only). Both predated the decision to keep all building on the CPU and use the GPU strictly for parallel stepping of frozen fabrics.

**Browser deployment caveat (May 2026):** The wasm build of this project still uses `wgpu` with the `webgl` feature (`Cargo.toml`), and WebGL2 has no compute shaders. So `physics_gpu` is currently a **native-only** capability; running GPU physics in the browser would require switching the wasm wgpu feature set from `webgl` to plain WebGPU and rewriting the buffer-readback paths to be `async` (today they call `device.poll(PollType::Wait)` plus `std::sync::mpsc::recv()`, which would deadlock the JS event loop). See `physics_gpu/batch.rs::read_buffer_typed` — the single helper that both `read_all_positions` and `read_frozen` route through — for the readback pattern that needs replacing. Section 13 below already lists wasm/webgl as out of scope; this note records *why*.

## 1. The vision in one paragraph

Tensegrity-lab's CPU build pipeline (Tenscript DSL, brick library, oven, build/shape/pretense/converge phases) is mature and correct. Don't touch it. Build whatever fabrics you want, however you want — one fabric, a thousand mutated variants, ten unrelated structures, anything constructed by familiar CPU code — and hand the whole collection to a GPU backend that **freezes each one and steps them all forward in time in a single dispatch**. The CPU stays the authority for every kind of fabric construction and mutation. The GPU is purely a parallel stepping engine: feed it any slice of `Fabric`s and they all advance lockstep through the same passage of time. The point is speed — thousands of trials per generation, all stepping at GPU bandwidth, with no GPU-side knowledge of where the fabrics came from or whether they're related.

## 2. Design boundaries

Two design choices are baked into the architecture and not up for revisit
without a corresponding doc change:

- **Building stays on the CPU.** No GPU-side `append_joints` /
  `append_elastic` / `append_push` mid-run, no dense-index map maintenance,
  no per-frame `Span::Approaching` interpolation. Building is finished before
  upload. The freeze-then-ship model has no incremental-mutation API surface.
- **Pushes are springs, not rigid constraints.** The shader does symmetric
  spring-push (Hooke's law in both compression and extension). No SHAKE /
  RATTLE / rigid-bar treatment, matching the CPU integrator.

## 3. Architecture

```
tensegrity-lab/src/
├── fabric/               # unchanged — CPU authority for building
├── build/                # unchanged — DSL, oven, plan executor
├── wgpu/                 # unchanged — rendering only
└── physics_gpu/          # NEW — frozen fabrics + parallel batches
    ├── mod.rs            # GpuPhysics facade
    ├── shaders/
    │   └── physics.wgsl
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

The shader recognises two interval kinds: **elastic** (slack when compressed,
spring otherwise) and **push** (symmetric spring, no slack). Tensegrity-lab's
9 roles map as:

| tensegrity-lab Role | shader bucket | notes |
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

Material stiffness differences are baked into the per-interval `k` at freeze
time. The shader doesn't know about materials. Rigid SHAKE/RATTLE is not
implemented — pushes are springs, matching the CPU semantics exactly.

## 7. Source material (paths are authoritative)

GPU side, all under `src/physics_gpu/`:
- `shaders/physics.wgsl` — single WGSL file with one entry point per pass.
  Workgroup size 64. Passes: `half_kick_and_drift`, `elastic_forces`,
  `second_half_kick`, `ground_collision`, `push_forces`. No SHAKE/RATTLE.
- `batch.rs` — owns all GPU buffers, bind groups, and compute pipelines. The
  batch dimension lives here (pad-to-max layout).
- `params.rs` — `PhysicsParams` uniform packing.
- `parity_test.rs`, `smoke_test.rs`, `sphere_sweep_test.rs` — coverage.

CPU reference paths (for parity work):
- `src/fabric/mod.rs` — `Fabric` struct and `Fabric::iterate(physics)`. The
  CPU integrator is the reference the GPU output is compared against.
- `src/fabric/joint.rs` — `Joint`.
- `src/fabric/interval.rs` — `Interval`, `Span::{Fixed, Approaching, Measuring}`,
  `Role` enum.
- `src/fabric/physics.rs` — `Physics` struct (drag, gravity, surface, dt)
  maps onto the `PhysicsParams` uniform.
- `src/wgpu/mod.rs` — `Wgpu` struct, single device + queue. The compute
  pipelines share this device with the renderer.

## 8. Live GPU physics in the application

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

## 9. Out of scope

- GPU baking. Oven stays CPU.
- GPU-side building, brick assembly, or DSL execution. CPU does all of this.
- Mid-run topology changes. Parallelize means freeze topology.
- Rigid push (SHAKE/RATTLE). Spring-push only, matching CPU.
- wasm/webgl backend for GPU physics. Native first.
- Replacing the CPU path. CPU is reference, forever.
- Refactoring `Fabric`, `Joint`, `Interval`, `Role`, `Span`, or the oven.

## 10. Design rationale, briefly

Two earlier shapes of this design were tried and discarded:

1. **Incremental GPU building** — `append_joints` mid-run, dense-index map
   maintenance, per-frame `Span::Approaching` interpolation. Unnecessary
   complexity when building is finished before upload.
2. **Shared-topology batch** — N parallel copies of one `FrozenFabric` over
   shared topology buffers, perturbed by a `Mutator` API. Discarded because
   mutations routinely change interval structure, so shared topology is unsafe
   in the general case.

The current shape: the GPU module accepts an arbitrary slice of CPU-built
`Fabric`s and steps them all in lockstep. Each fabric carries its own
topology in its own slot. Identical fabrics in a batch are a degenerate
special case, not a privileged one. Mutation lives entirely in CPU code; the
GPU module has no `Mutator` API.
