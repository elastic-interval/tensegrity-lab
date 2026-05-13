# Articulating Tensegrity — Notes Toward Lightweight Robotics

**Status:** Design notes. Not implemented. Captured for future work; intentionally
not yet codified into the DSL or fabric/brick libraries.

## Goal

Find tensegrity structures that simultaneously
- **Stand stably when unperturbed**, holding a definite equilibrium pose under
  pretension, and
- **Articulate naturally** under modest muscle action, returning to the rest
  pose when the muscle releases.

The interest is in a lightweight, low-parts-count form of robotics: no point
hinges, no bearings, no stepper motors. Movement comes from contractible cables
("muscles") biasing the pretension equilibrium, and from the structure's own
flexibility — not from any concentrated rotating part.

Pre-determining what such structures look like is hard. The expectation is that
GPU-parallel evolution over many candidates per generation will surface
non-obvious solutions.

## The mechanical hypothesis: brick-to-brick articulation

The existing brick system (`src/build/dsl/brick_library.rs`) yields
internally-rigid tensegrity primitives (Single Twist, Omni, Torque, …) that
attach face-to-face by **merging the three joints of one triangular face with
the three joints of another**. The result is rigid: all 6 relative-pose
degrees of freedom between the two bricks are pinned.

**Key observation:** the number of joints merged between two bricks literally
determines the kinematic type of the connection between them.

| Joints merged | Constraints | Relative DOFs remaining | Joint type |
|---:|---:|---:|---|
| 3 (full face) | 9 | 0 | **Rigid** (current behaviour) |
| 2 (face edge) | 6 | 1 | **Hinge** — rotation around the merged edge |
| 1 (face corner) | 3 | 3 | **Ball joint** — rotations around the merged point |

Every triangular face on every existing brick already contains all three forms
for free: three edges (potential hinges), three corners (potential ball joints).
**No new primitive is needed.** The DSL's face-merge operation generalises to a
partial-merge with 1, 2, or 3 joint identifications.

This places articulation **between bricks**, not within them. Each brick stays
internally rigid; the soft modes live at the inter-brick connections.

## Closing the loop on articulation DOFs

A bare partial-merge is a frictionless joint with no restoring force — a
hinge-merged pair of bricks can fold to any dihedral angle. To behave like a
biological joint, each articulation needs:

- A **pretensioned passive cable** crossing the articulation (e.g., between the
  unmerged corner of brick A's face and the unmerged corner of brick B's face).
  Stretches as the angle moves away from rest; provides the restoring force.
- An antagonistic **muscle** (contractible pull, `Role::Pulling` with
  time-varying rest length) along a parallel path. Contracts to drive the angle
  away from rest; releases to let the passive cable restore it.

That's the biological pattern: bone + bone + joint contact + ligament + muscle.
The brick gives the bone-and-contact for free.

For ball joints, multiple cables in different planes are needed to bias each
rotation axis. Three antagonistic muscle pairs is full controllability; fewer
gives partial.

## What happens to the unmerged face cables

A brick's triangular face has 3 cables along its edges. When two bricks
partial-merge:

- The cables along the *merged* edge (hinge) or *merged* corner (ball) merge
  cleanly with their counterparts in the other brick.
- The cables along the *unmerged* parts hang off the joint into the inter-brick
  space.

Best default: **drop the unmerged face cables and let evolution add explicit
ligament/muscle cables instead.** Keeping them would over-constrain the
articulation in a way that's hard to reason about per connection.

## The stiffness picture

Frame stability and articulability as eigenvalues of the linearised stiffness
matrix at equilibrium:

- **6 zero modes** — rigid-body motion of the whole structure. Unavoidable.
- **A small handful of small positive eigenvalues** — articulation modes. Soft;
  muscles can drive the structure along these without large force.
- **A long tail of large eigenvalues** — rigid modes. Hold the shape.

A useful articulating structure has a **clean gap** between the soft modes and
the rigid modes. Too few soft modes: stiff and uninteresting. Too many:
floppy, no defined pose. Evolution should explicitly reward the spectrum
shape, not just terminal behaviour like "walks N metres".

## First concrete experiment

Before any evolution: build by hand and confirm the mechanic works at all.

**Setup.** Two Single Twist bricks. Partial-merge them on one edge of one face
each, leaving them able to hinge around the shared edge. Add:

1. One passive pretensioned cable between the two free corners of the merged
   faces, crossing the hinge axis.
2. One antagonistic muscle (a `Pulling` interval with a sinusoidal `Approaching`
   rest length) along a parallel path on the opposite side.

**Pass criteria.**
- With the muscle at rest length, the pair stands at a defined dihedral angle
  under pretension alone.
- Contracting the muscle drives the dihedral angle through a range; releasing
  it lets the passive cable restore the rest position.
- Releasing the muscle from a stretched position settles within a few seconds
  without oscillation that grows.

If those three hold, the whole programme has a working foundation. Everything
else (quadruped, evolution, GPU-parallel fitness eval) builds on this.

## Evolution direction

Once the basic articulation mechanic works:

- **Genome split.** Structural genome (which bricks, how merged) vs actuation
  genome (where muscles go, with what phase under a global clock). Mutate at
  different rates.
- **Mutation operators that preserve stability by construction.** Add brick by
  partial-merge (choose 1/2/3 joint count and which face joint(s)). Add
  ligament cable across an articulation. Add muscle. Shift muscle phase.
  Remove a leaf brick.
- **Initial fitness components.**
  1. **Stand fitness:** height maintained, low kinetic energy, low orientation
     drift over T seconds with no actuation.
  2. **Walk fitness:** horizontal displacement of centroid over T seconds with
     muscles driven by a global sinusoidal clock at frequency f.
- **Stiffness-spectrum fitness (later):** explicitly reward
  small-but-nonzero eigenvalues separated from the rigid bulk.
- **GPU-parallel evaluation.** Use the existing `physics_gpu` batch infrastructure
  (`src/physics_gpu/batch.rs`) to evaluate the whole population in one
  dispatch. Each individual = one slot.
- **Start with a quadruped.** A central Omni brick with four Single Twist legs,
  each leg attached by an edge-merge (knee). Roughly 5 bricks, ~30 joints.
  Small enough to evolve, articulated enough to walk.

## Open questions to revisit before coding

1. **Partial-merge in the DSL.** How is `Spin` chirality handled when only an
   edge or corner is merged? Edge merges line up two oriented edges; corners
   merge a single joint with no orientation. Probably needs DSL surface, not
   just a `Fabric` API.
2. **Vulcanize behaviour.** Does the existing vulcanize step (which adds
   reinforcing intervals to "stiffen" the structure) need to be suppressed at
   articulation joints? Otherwise it may freeze articulations that should stay
   soft.
3. **`Role::Approaching` for muscles.** Muscles need a time-varying rest length
   under a control signal. The existing `Approaching` span interpolates over
   time, but the animation phase (`animate_phase.rs`) already does muscle-like
   driven contraction. Probably reuse rather than reinvent.
4. **Surface model for walking.** A "sticky" frozen surface (current default)
   prevents sliding once a foot touches; that's the wrong model for walking
   evaluation. A bouncy or frictional surface is needed. Verify
   `surface_bouncy()` behaves usefully under repeated contact, or add a
   frictional surface mode.
5. **Closed loops of partial-merges.** Three bricks edge-merged in a triangle
   would form a closed kinematic loop with internal compatibility constraints.
   May or may not be useful; needs experiment to see what equilibria exist.
6. **Whether to keep "evolution" as a separate concept.** The current
   `src/build/evo/` framework is generic and trait-based. The articulating-
   structure work may need a more bespoke evolver (because mutations preserve
   stability by construction, the framework's generality may be overhead). To
   decide once the first experiment is running.

## Why not just keep the old `evo` branch

The `evo` branch's walking-evolution work (`src/build/evo/walking/`) is
relevant in spirit but predates the brick-articulation framing here. It used
random pulls between arbitrary joint pairs to find walkers, which produced a
huge unguided search space. The author labelled the final commit "attempt at
walking" — i.e. it didn't quite get there.

The articulation-via-partial-merge approach is structurally different: every
mutation produces a stable structure by construction, the search space is
smaller and better-conditioned, and the genome decomposes cleanly into
structural + actuation parts. Code-wise, almost nothing from the evo branch
ports usefully; **the design vocabulary** (genome split, mutation operator set,
multi-component fitness) is what's worth reading there. See
`docs/evolution.md` for the current dev framework, which already supplies
trait scaffolding for a fitness/population system.
