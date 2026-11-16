# Brick evolution — plan

Branch `evolution`, cut from `dev` at `18e6060`. Its one purpose: grow the
current bricks from a simpler specification plus rules, by Darwinian
evolution, so they appear rather than being typed in. The current bricks are
the measurable goal. Something new (an elliptical compression brick) comes
only after they can be reproduced.

This is a proposal. Nothing below is built yet.

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
