# Articulation evolution

Evolve **articulating bricks**: small tensegrity cells, structurally like
the static library bricks, that carry their own *actuators* and are tuned
to be **compliant mechanisms with a single stable equilibrium**.

An articulating brick is "fit" when:

1. **Unambiguous rest state** — with its actuators dormant it settles to
   one finite, properly-tensioned equilibrium pose.
2. **High articulation gain** — when an actuator contracts, the structure
   undergoes a *large* deformation for *little* actuator effort.
3. **Reversibility** — when the actuator goes dormant again, the structure
   returns to its rest pose. Elastic, not plastic.

Run with:

```
cargo run --release -- --evolve-articulation 42
```

The existing `--evolve <seed>` flag still runs `GrowthGenome` evolution;
the two paths are parallel.

## Face-centric bricks

The static bricks are *face-centric*: their tension network is the face
radials, not separate internal pulls. `OmniSymmetrical` has six pushes,
**zero internal pulls**, and eight faces — its 8 × 3 = 24 face radials are
the entire tensegrity (exactly a 6-strut tensegrity's cable count).
`SingleTwistLeft` is the exception, with three internal pulls and two
faces.

An articulating brick keeps that self-tensioning face structure and adds
one or more **actuators**: contracting pulls between two faces' centres,
created exactly the way the DSL's joiners/spacers grab a face
(`face.middle_joint`, see `shape_phase.rs`). The faces persist — the
actuator never welds them. Actuators only **contract** (pull); they never
push.

The default seed is `OmniSymmetrical`: it articulates ~2.4× more than
`SingleTwistLeft` for the same actuator effort. Articulating bricks are
expected to grow *richer* than the static ones (more parts, more moving
parts).

## What evolves

`BrickStructure` (`structure.rs`), seeded from a baked static brick:

- `joints: Vec<Vec3>` — structural joint positions in metres
- `pushes`, `pulls: Vec<BrickInterval>` — members with ideal length
- `faces: Vec<BrickFaceSpec>` — triangles (joints / spin / scale / aliases /
  `radial_strain`, the local pretension of that face's three radials)
- `actuators: Vec<Actuator>` — each a face pair plus a contraction fraction

Omni carries **no internal pulls** — its stiffness lives entirely in the
face radials — so `radial_strain` is the primary lever for retuning its
rest tensions and discovering a compliant joint.

`BrickStructure::express()` materialises a `Fabric` directly (bypassing
the DSL): structural joints, pushes, pulls, then each face as a middle
joint + three `FaceRadial` intervals (the `attach_brick` pattern), then
each actuator as a `Pulling` interval between two face centres. It returns
the actuator interval keys and structural joint keys the trial needs.

## Mutations (`Genome::adjacent_possible`)

- **Shift joint**: ±0.06 m along a random axis on one structural joint.
- **Retune member**: free multiplicative jump (~0.75–1.33×) on one push or
  pull rest length — significant, so the search can genuinely reshape the
  structure rather than creep.
- **Retune face**: ±0.04 on one face's `radial_strain`, clamped to a
  load-bearing band — softens/stiffens the tension network locally.
- **Move actuator**: repoint one actuator at a different face pair.
- **Add actuator**: grow a new moving part across an unactuated face pair.
- **Add face / remove face**: topological growth/pruning of the tension
  network (and actuator targets). Grown topologies *can* stay valid
  (~half pass the gate, reaching 9+ faces in `diagnose_growth`), but they
  score below a well-refined omni, so a pure maximiser discards them.

`adjacent_possible` returns the applicable mutations **shuffled**, because
the population strategy applies the *first* variant — without the shuffle
every offspring would get the same operator.

Face geometry follows the joints automatically (midpoints recomputed at
expression), so shifting a joint reshapes its faces too.

> Topological *part* growth (sprout a push, add a face) and composing
> static bricks into richer assemblies are the next levers, scaffolded
> (`actuators` is a `Vec`, `from_baked` takes any `BrickName`) but not yet
> implemented.

## Trial: step / hold / release (`trial.rs`)

A small phase machine, run under heavily-damped `BAKING` physics so each
phase reaches a quasi-static equilibrium (zero gravity, no ringing):

1. **Settle** (actuators dormant) → capture the rest pose `R0`; record
   whether it is finite, low-KE, and properly tensioned.
2. **Hold** (actuators at `rest × contraction`) → capture the deformed
   pose `D` and the actuator effort (summed tension strain at hold).
3. **Release** (actuators back to rest) → capture the returned pose `R1`.

The same machine is stepped per-frame by the visual runner and run to
completion by the headless `evaluate()`.

Deformation and return are measured by `metric::shape_distance` — the RMS
change in every inter-joint distance, over the **structural** joints only.
That is invariant to the free brick's drift/rotation (no SVD, no
correspondence beyond joint ordering) and excludes the actuator's own
commanded stroke from the structural-articulation reading.

## Fitness (`fitness.rs`)

Combined **multiplicatively** (a weighted average would let a structure
win on cheap dimensions while scoring nothing on articulation):

```
score = rest_gate × gain × reversibility
```

| Term | Meaning |
|---|---|
| `rest_gate` (0/1) | settled, finite, and properly tensioned at rest |
| `gain` | `raw / (raw + GAIN_HALF)` where `raw = deformation / (effort + ε)` |
| `reversibility` | `1 − return_error / deformation`, clamped to `[0,1]` |

`gain` is a *ratio* (ease of deformation: large motion for little effort),
so the limp trap — low effort, no motion — can't win. The smooth
`raw/(raw+K)` saturation has **no hard ceiling**, so there is always a
gradient toward a softer, larger-throw joint and the population never
plateaus. `reversibility` is measured relative to how far the brick
deformed, so a structure that barely moves can't claim it for free; an
`EFFORT_EPS` floor stops near-zero effort from blowing the ratio up.

Constants (`SETTLED_KE`, `SLACK_EPS`, `GAIN_HALF`, `EFFORT_EPS`,
`DEFORM_EPS`) are tuned against the seed bricks via the diagnostic tests.

This is the "joint" signature: **easy to deform, springs back to rest**.
An evolved winner reached raw-gain ~6 (deformation ~0.16 m at actuator
effort ~0.006, returning to within 2e-4 m) — a soft, reversible hinge.

## Wiring

- `RunStyle::ArticulationEvolution(u64)` (`src/lib.rs`)
- `--evolve-articulation <seed>` (`src/main.rs`)
- `CrucibleAction::ToArticulating(u64)` (`src/events.rs`)
- `Stage::Articulating(ArticulationVisualRunner)` (`src/crucible.rs`)

`ArticulationVisualRunner` drives a `SimplePopulation` and the
step/hold/release trial directly (it does not reuse the generic
`EvolutionEngine`/`ActiveTrial`, whose continuous-drive model doesn't fit
phased pose capture). The top-subtitle label shows generation, phase
progress, and the live fitness breakdown.

The visual run is seeded from three distinct viable bricks
(`OmniSymmetrical`, `OmniTetrahedral`, `SingleTwistLeft`) so it explores
more than one topology. `TorqueSymmetrical` is omitted — it does not pass
the rest gate standalone (`diagnose_seeds`). Note that a single elitist
population still **converges** toward the best-scoring lineage, so lasting
structural variety needs topological growth or niching (below).

## Verification

- `cargo test --release --lib articulation` — five smoke tests (seed
  expresses with faces + actuator, mutations vary, trial completes finite,
  shape-distance rigid-invariance, evolution beats gen-0 median).
- Diagnostics (`-- --ignored --nocapture`):
  - `diagnose_seeds` — per-seed outcome and fitness breakdown.
  - `diagnose_settle` — KE decay per physics preset.
  - `diagnose_evolution` — best score per generation + the winning genome
    (its radial strains, push lengths, and raw gain).
  - Observed: a steady, non-plateauing climb (~0.45 → 0.76 over 20
    generations on the Omni seed) as it softens faces and retunes lengths
    into a compliant joint, with reversibility held near 1.0.
- Visual: `cargo run --release -- --evolve-articulation 42`.

## Convergence vs. variety

A pure maximum-seeking GA converges on the single fittest structure — a
well-refined omni — and discards valid-but-lower-scoring wild variants,
even though topological growth produces them. So no matter the seeds or
growth operators, the run *looks* like "omni, refined." Seeing a *variety*
of wild articulating bricks needs a different paradigm: **quality-diversity**
(e.g. MAP-Elites — keep the best structure per niche, niches keyed on a
shape descriptor like face/joint count or symmetry) or novelty pressure.
That illuminates many distinct good joints instead of one peak.

## Out of scope (next)

- Quality-diversity / novelty search to surface a zoo of distinct joints.
- Proper brick composition via the real `attach_brick`/`join_faces`
  machinery + read-back (the geometric shortcut explodes; see history).
- Co-evolve actuator parameters (contraction, phase, antagonistic pairs).
- Bake a winning articulating brick into the static library.
- GPU-parallel trial evaluation (see `docs/gpu-compute-backend.md`).
```
