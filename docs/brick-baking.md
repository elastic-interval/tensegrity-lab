# Brick Baking: From Logical Description to Solid Tensegrity

How a brick goes from its DSL description to the `BakedBrick` that fabrics
are built from, and the guarantees enforced along the way. Source of truth:
`src/build/dsl/brick_library/equilibrium.rs` (the pure baker, which produces
every production brick at startup) and `src/build/oven.rs` (the visual
`--bake-bricks` mode).

## The pipeline

1. **Describe** — a `BrickPrototype` (`src/build/dsl/brick_library/*.rs`)
   declares joints, pushes per axis, pulls, and faces with role aliases.
   The description is logical: topology plus nominal member lengths.

2. **Form-finding** — `bake_brick_pure` places same-axis push endpoints at
   `±axis·ideal/2`, then minimises spring energy with L-BFGS (slack members
   contribute nothing). An outer bisection scales all member rest lengths
   until mean *face-radial* strain hits `TARGET_FACE_STRAIN` — face radials
   have **fixed absolute rest lengths**, which anchors the brick's physical
   size and keeps every brick's faces at the standard size that
   face-to-face attachment relies on.

3. **Shrink-wrap (pretension by construction)** — the form is now known;
   this pass sets the forces. Every member's rest length is re-derived from
   its settled length at the designed pretension band
   (`SHRINK_WRAP_PRETENSION`, currently 5%): pulls end 5% stretched, pushes
   5% compressed. Re-settle, repeat to a fixed point (a few rounds). Face
   radials are exempt — they keep anchoring size and standard face
   geometry.

4. **Reorient + symmetrize** — the brick is rotated onto its `max_seed`
   orientation; bricks with a declared 3-fold symmetry are orbit-averaged
   onto the symmetric manifold and verified.

5. **Final band pin** — symmetrizing moves positions slightly, so rests are
   re-derived once more (no re-solve) from the final symmetric geometry.
   Every member's stored strain is then *exactly* ±5%, identical across
   rotational triples — which is what keeps fabric-level symmetry guards
   (e.g. OpenClaw's 0.1 mm triple tolerance) satisfiable.

6. **Validate** — `validate_pretension` enforces the definition of a solid
   tensegrity: every push strictly in compression, every pull strictly in
   tension, nothing within `SLACK_EPSILON` (0.5% strain) of slack. On
   violation the bake **panics with a per-member report** naming offenders.
   The test `all_bricks_are_solid_tensegrities` bakes every brick on every
   `cargo test`.

## Why the shrink-wrap exists (a cautionary tale)

Before it, the bake tuned one global scalar against the face-strain proxy
and nothing constrained the actual members. The Torque brick's long z pair
demonstrated the failure mode: its corners are positioned by the face
network, so the realized span (~5.5 m) was independent of the strut's rest
length — any rest below the span left the strut **slack**, a rattling loose
member, and the rest-length "knob" was silently dead (rest ratios 1.25 and
φ baked byte-identical bricks). The shrink-wrap makes every member engage
at a known prestress no matter what the description said, and the validator
makes the failure mode loud if it ever returns.

Two consequences worth knowing:

- **Member rest lengths in the prototype steer the *form-finding* only.**
  After shrink-wrap, forces are uniform by construction. To change a
  brick's shape, change its topology or its pull structure — those move
  the settled form; the strut rests then follow the spans they occupy.
- **Prototype numbers live in the face-anchored absolute regime.** Push
  rests far below the face-anchored size (e.g. normalizing to unit
  lengths) collapse the form-finding into a degenerate all-slack state.
  Keep member ideals in the ~3 m regime the face rests imply.

## Proportions: the golden mean

When a brick mixes strut lengths they relate by powers of φ
(`PHI` in `brick_dsl.rs`): default strut 1, next longest φ, then φ², at the
face-anchored base (so `3.0` and `3.0 × PHI`). This is declared intent for
the form-finding; whether the *realized* geometry exhibits the ratio
depends on the topology (see the Torque z pair above). Realized-proportion
targeting — "make the settled span φ × the base span" solved through the
pull network — is future work (force-density form-finding is the classical
route if we ever want it).

## The two baking paths

- **Pure** (`bake_brick_pure`) — produces all production bricks, cached at
  startup, milliseconds per brick. Includes shrink-wrap + validation.
- **Oven** (`--bake-bricks`) — visual mode: watches a prototype settle
  under time-stepped physics, tunes scale by the same face-strain target,
  and reports the converged scale (update `initial_scale()` in
  `baked_bricks.rs` when it drifts). It is a design instrument; its output
  is not consumed. Prototype fabrics get a ~1 mm deterministic per-joint
  nudge at creation (`to_fabric`) because time-stepped physics — unlike
  the pure solver — cannot handle the coincident joints that same-axis
  push placement produces.

## Tweaking a brick

1. Edit the prototype (`torque.rs` etc.) or its params
   (`baked_bricks.rs::brick_params`).
2. `cargo run --release -- --bake-bricks` to watch it settle; note the
   printed scale.
3. `cargo test --release all_bricks_are_solid_tensegrities` — the validator
   tells you immediately if the result isn't a real tensegrity.
4. Fabric-level guards (`test_minimal_man_mirror_symmetry`, the OpenClaw
   symmetry/collision suite) catch downstream geometry consequences.

⚠ Changing brick pretension or geometry shifts **every fabric built from
that brick** — for OpenClaw that means the engineering CSV. The 2026-07-28
pretension overhaul grew OpenClaw ~4.4% (height 11159 → 11646 mm) and moved
the connector-collision count from 24 to 36. Regenerate and re-hand-off the
CSV after any such change; never mix parts produced from CSVs on opposite
sides of a brick change.
