# Brick evolution — plan

Branch `evolution`, cut from `dev` at `18e6060`. Its one purpose: grow the
current bricks from a simpler specification plus rules, by Darwinian
evolution, so they appear rather than being typed in. The current bricks are
the measurable goal. Something new (an elliptical compression brick) comes
only after they can be reproduced.

This is a proposal. Step 0 is done; its results are below the plan.

## What the targets are, measured

Baked on `dev` by `bake_brick_pure` (`brick_library/equilibrium.rs`):

| Brick | Struts | Pulls | Faces | Joint radius | Push strain | Pull strain |
|---|---|---|---|---|---|---|
| SingleTwistLeft | 3 × 3.017 m | 3 × 2.217 m | 2 | all 1.535 m | −5.3% | +11.6% |
| OmniSymmetrical | 6 × 3.110 m | 0 | 8 | all 1.739 m | −4.7% | — |
| TorqueSymmetrical | 7 × ~3.08, 2 × 6.11 m | 8 × 2.206 m | 8 | 1.56 – 3.16 m | −3.1 … −4.9% | +10.7% |

Every face radial sits at 1.100 m (rest 1.0, the 10% face strain the
bisection aims for), except Torque's, which come out 1.16 / 1.03 / 1.10:
its faces are not equilateral.

Things this shows:

- **There are three targets, not four.** On `dev` the pure baker ignores
  face scaling (`face_scaling = ScaleMode::None` in `build_from_prototype`),
  so OmniTetrahedral bakes to Omni's geometry; its large and small faces
  only come in when a fabric attaches it. Nothing to evolve there.
- **The size is not a free number.** Face radials have absolute rest
  lengths, and the bisection scales everything else until they are 10%
  stretched. So what a brick's numbers really set is ratios: push to face,
  pull to push.
- **Single and Omni put every joint on one sphere.** That is a natural,
  checkable property, and a hint at what "fit" means.
- The handwritten numbers are few: Single 3.204 / 2.0, Omni 3.271, Torque
  3.0 / 3.0 / 6.0 / 1.86, plus each brick's guessed `initial_scale`. The
  rest of each prototype is topology (who connects to whom) and naming
  (face aliases, `Seed(n)`, `face_twist`).

## 1. Distance from an evolved brick to a current one

Two layers, because the search needs a smooth signal and the verdict needs
an exact one.

**Signature distance (smooth, needs no correspondence).** Compare:
counts of struts, pulls and faces; the sorted strut lengths and sorted pull
lengths, each divided by the face radial length; the sorted pairwise joint
distances, divided the same way; sorted member strains per role. Sum of
relative differences, zero for an identical brick, blind to rotation,
translation and relabelling. Cheap enough to log for every individual.

**Match (exact).** Find the joint correspondence by graph isomorphism of
the struts + pulls + faces graph (at most 18 joints, so brute force with
pruning by degree is fine), then Kabsch-align the joints and report the
RMS joint offset divided by the brick's radius, plus the worst member
length difference. A brick "lands" on a target when the topology is
isomorphic and the RMS is under a tolerance to agree on, say 2%. The
correspondence is also what lets the face names and roles of the current
brick be copied onto the evolved one.

Rule: **distance to a target is the scoreboard, never the fitness.** If it
were the fitness, evolution would only be curve-fitting to numbers we
already have, and nothing would appear naturally.

## 2. The simple specification and the rules

**Specification** (the genome), a handful of facts:

- the symmetry group: C3 for Single and Omni; Torque's group to be read
  off its baked geometry (a twofold group, by the look of it);
- chirality (left; right comes from `BakedBrick::mirror` as now);
- one representative per orbit: each strut orbit, each face orbit (three
  strut ends, one spin), each pull orbit. The symmetry turns these into the
  whole brick, so Omni is 2 strut orbits + 4 face orbits (the two faces
  on the axis, and two triples), and every
  offspring is symmetric by construction;
- at most a few ratio genes (push to face, pull to push), and ideally none,
  if the rules below make them unnecessary.

**Rules** (fixed, not evolved):

1. Equilibrium is the existing pure baker: L-BFGS on spring energy, scaled
   until face radials are 10% stretched. No time-stepped physics, no
   gravity: a brick is a zero-gravity object, and baking takes
   milliseconds, so thousands of trials a second.
2. A face is three strut ends from three different struts.
3. **Solid tensegrity**: every strut compressed and every pull and radial
   stretched by more than a slack margin (0.5%). Fails → not a brick.
4. **No mechanism**: apart from the six rigid motions, the stiffness
   matrix at equilibrium has no near-zero modes. This is the rigidity
   screening the evaporation experiment lacked.
5. **Clearance**: no two struts closer than a set distance.

**Fitness** for what survives the rules, candidates to test:

- uniform stress: low spread of strain within each role (the current
  bricks are nearly uniform; Torque less so);
- regular faces: equilateral, because attaching face to face relies on it;
- joints on a sphere (Single and Omni do this exactly);
- stiffness per member: lowest non-rigid eigenvalue of the stiffness
  matrix divided by the number of members, so economy is rewarded.

Which mix is right is the open question, and step 0 below answers it
before any evolution runs.

## 3. The evolutionary loop

- **Founder**: the bare spec, n strut orbits and no tension, or one random
  face orbit; deterministic from a ChaCha8 seed (already a dependency).
- **Mutations**: add or remove a face orbit; move one vertex of a face to
  another strut end; add or remove a pull orbit; add or remove a strut
  orbit; nudge a ratio gene by a log-normal factor.
- **Express**: orbit genome → full `BrickPrototype` → baker → rules.
- **Selection**: (μ + λ) with elitism, keeping the best per topology
  signature (struts, pulls, faces) so one lineage does not take over: the
  articulation evolution showed a plain elitist population converges on
  one shape and throws the rest away.
- **Each generation logs**: best fitness, and each individual's signature
  distance to the three targets; any match is reported.
- **Freezing**: a winning genome is written out as code, like the
  prototypes are now. Swapping a current brick for its evolved twin changes
  every fabric built from it, Headless Hug included, so that is Gerald's
  call when it comes, and never part of this branch by default.

Determinism: single-threaded, ordered collections, seeds only, same as
every build (gotcha 7).

## Order of work

0. **Is each current brick a fixed point?** Bake each target, then every
   one-step mutation of it, and score them all under the candidate
   fitness. If the current brick does not beat all its neighbours, the
   rules or the fitness are wrong, and no evolution run would find it.
   Also sweep the ratio genes: if Single's and Omni's shapes hardly move
   with push and pull length (the classic three-strut prism twist does not
   depend on lengths), those numbers can leave the specification.
   Needs: `bake_brick_pure` to accept a prototype instead of looking it up
   by name, with output for the current bricks unchanged.
1. The distance measure, with tests: each target is at distance zero from
   itself, from its mirror after mirroring, and from a rotated copy.
2. Orbit genome → prototype, with a test that the genomes for Single, Omni
   and Torque express to bricks that match the targets exactly.
3. The loop, headless, as an ignored test that prints per generation.
   Success: from a bare founder, Single and Omni found in most seeds;
   Torque, whose middle strut hangs only by pulls, is the hard one.
4. Only then a visual mode, if wanted (`Stage::Evolving` exists).

## Step 0 results (2026-11-16)

Code: `src/build/evo/brick/` — `measure.rs` (rules and candidate
measures), `neighbours.rs` (one-step changes, named by joint so a change can
be carried round a symmetry), `fixed_point.rs` (the reports, ignored tests).
`equilibrium::settle` is the baker's scale bisection, taken out of
`bake_brick_pure` so any prototype can be baked; output for the current
bricks is unchanged.

```bash
cargo test --release --lib current_bricks_against_their_neighbours -- --ignored --nocapture --test-threads 1
```

```bash
cargo test --release --lib current_bricks_with_their_numbers_changed -- --ignored --nocapture
```

The changes tried: remove a strut, pull or face; add a pull (resting 10%
short of its span) or a face; move one face vertex to another strut end.
Each alone, and for Single and Omni also carried round their threefold
symmetry. Single has 26 single and 21 symmetric neighbours, Omni 394 and
246, Torque 1137 (it declares no symmetry, so none symmetric).

**The rules work as gates.** Every removal fails: with a member gone the
baker lets the brick fold until struts pass through each other (the baker
has no contact), caught by the clearance rule. Most rewiring fails on slack
members or mechanisms. Additions mostly pass. Two rules were added on the
way: the face radials must reach the target strain (otherwise the bisection
found no scale, see below), and struts at least 0.1 face radials apart. The
mechanism count needed the rigid motions projected out of the stiffness
matrix: the baker's equilibrium is approximate, so turning the brick was not
quite free.

**Uniform stress is the one candidate under which all three current bricks
win.** No viable neighbour, single or symmetric, has a smaller spread of
strain within a role, for any of the three. Every other candidate is beaten
somewhere:

| Candidate | Single (1 / sym) | Omni (1 / sym) | Torque (1) |
|---|---|---|---|
| uniform stress | 0 / 0 | 0 / 0 | 0 |
| stiffness per member | 0 / 6 | 0 / 30 | 394 |
| clearance | 0 / 4 | 0 / 0 | 4 |
| face regularity | 0 / 3 | 0 / 0 | 0 |
| joints on a sphere | 0 / 2 | 0 / 0 | 8 |

(viable neighbours that beat the current brick). Stiffness per member is a
poor fitness: adding three pulls stiffens Omni and Single. Single and Omni
are uniformly stressed almost exactly (spread 1e-4); Torque is not (0.13),
yet nothing one step away does better.

**The numbers:**

- **Omni needs none.** Its push length from ×0.8 to ×1.25 changes the
  shape by at most 0.15% of a face radial. Omni is pure topology.
- **Single needs one: the ratio of pull to push.** Scaling push by f does
  close to what scaling pull by 1/f does. The shape moves about 0.3 face
  radials per 10%, and stress stays uniform throughout, so uniform stress
  does not pick the ratio. Somewhere between 0.66 and 0.69 (now 0.624) the
  faces cannot reach 10% strain
  at any scale and the bisection runs to its clamp at 10: no brick.
- **Torque's four numbers matter, and they are not optimal.** Struts along
  x ×1.1 lowers the strain spread from 0.13 to 0.10. Several 20% changes
  (x ×0.8, pull ×0.8, y or z ×1.25) give it a mechanism. Torque is the
  fragile one.

What this means for the plan:

- Fitness: uniform stress first, with the rules as gates. Start the
  evolution with Single and Omni.
- Specification: Omni drops its number; Single keeps one ratio gene until a
  rule for it turns up; Torque's numbers become genes, and an evolved
  Torque will probably not match today's, since today's is not its own
  optimum.
- Not yet covered: adding a strut; Torque's symmetry; strut contact in the
  baker (a removal now ends in struts passing through each other rather
  than in a mechanism).

## The census (2026-11-16)

For bricks this small, trying everything beats evolving: the space under
threefold symmetry is small enough to enumerate, and a census gives a
guarantee evolution cannot. So the order of work is now: **census first,
then evolution tested against the census**, then evolution alone on bricks
too big to count (the elliptical brick, combinations).

Code: `src/build/evo/brick/census.rs`. A class is its struts, placed as the
prototypes place them, in groups of three along X, Y and Z; one turn of the
symmetry carries X to Y to Z. A candidate is a set of face orbits and pull
orbits under that turn, in which every strut end is held by a face or a
pull. Each is baked, the rules applied, and the viable ranked by uniform
stress, fewer members first among equals. Topologies are compared up to
renaming struts and swapping their ends (graph isomorphism), so
relabellings of one brick count once.

| Class | Tried | Viable | Winner | Its strain spread | Runner-up's |
|---|---|---|---|---|---|
| 3 struts, up to 4 face and 4 pull orbits | 236 | 96 | SingleTwistLeft (2 faces, 3 pulls) | 0.00000 | 0.015 (1 face, 6 pulls) |
| 6 struts, faces only, up to 4 face orbits | 353,672 | 23,602 | OmniSymmetrical (8 faces) | 0.00000 | 0.152 (7 faces) |

**Both current bricks appear, first in their class, by a wide margin.**
Nobody typed their topology: the struts, the symmetry and the rules are
enough. The census takes half a second for Single's class and five minutes
for Omni's.

**Single's number does not appear.** Across pull/strut ratios 0.40 to 0.67
its stress stays uniform; stiffness per member is lowest near 0.52 and
rises both ways; clearance grows steadily to the cliff at about 0.675,
where the faces can no longer reach their strain. Nothing picks today's
0.624, so it stays as Single's one number (any ratio in the range gives a
valid Single; the census tries 0.45 to 0.65 and reports the best).

Tests: `single_appears` runs with the suite; `omni_appears` (five minutes)
and the report are ignored:

```bash
cargo test --release --lib census_of_single_and_omni -- --ignored --nocapture
```

Limits: Omni's class stops at four face orbits (twelve faces), and neither
class allows a strut out of the X, Y, Z placement. Next: Torque, which needs
its own symmetry worked out first, and bricks joined face to face.

## What was taken from `evo-brick`, and what was not

Nothing was merged as code. Its changes would change the bricks, and the
current bricks are the goal:

- **Shrink-wrap pretension** in the baker (every pull +5%, push −5% after
  form-finding): reshapes every brick and so every fabric (it grew Open
  Claw 4.4% and raised its collisions from 24 to 36). Left out.
- **Torque's eight extra pulls and its φ z pair, Minimal Man on a Torque
  seed, the Propeller shortened**: changes to current bricks and fabrics.
  Left out.
- **Face scaling honoured in the baker**: a real fix (OmniTetrahedral),
  but it changes that brick. Noted above, left out.
- **Dropping `brick_exporter` and the Oven's code generation**: unrelated
  cleanup. Left out.
- **`docs/brick-baking.md`**: describes the shrink-wrap pipeline, not the
  one on `dev`. Left out; this document says what the baker does now.
- **`evaporation.rs`** (strut clouds, cable bitmask, height per cable
  under gravity): its goal is free structures, not these bricks, and its
  random founders rarely survived. Left out as code. Kept as ideas: the
  genome is a seed plus discrete choices, ChaCha8 throughout, ordered
  collections, bitwise-reproducible expression tested.
- **Its slack validator** (`validate_pretension`): the idea becomes rule 3,
  written as part of the evolution code rather than as a panic in the
  production baker.

The branch `evo-brick` stays as it is for reference.
