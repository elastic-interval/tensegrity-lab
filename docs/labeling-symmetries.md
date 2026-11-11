# Labeling strategies for all symmetries

Groundwork for a future generic Packer (assembly/disassembly sequencing).
The OpenClaw build used a threefold-specific pipeline; new structures need
the same capabilities for *any* seed symmetry. This document records what
the seed knows today, the `LabelSymmetry` contract that generalizes it,
and the sketch of how a Packer will consume it.

## What the seed knows today

- **`BrickPrototype.cyclic_axes`** (`brick_dsl.rs`): per-role declaration of
  the cyclic axis order under rotation. Only the omni brick declares one —
  `[X, Y, Z]` for `Seed(1)`. Position 0/1/2 in the list becomes letter
  A/B/C on seed joints.
- **`BrickPrototype::symmetry(role)` → `BrickSymmetry`** (`brick.rs`):
  geometric rotation (axis + permutation) derived from `cyclic_axes`.
  **Order is hardcoded to 3** and requires exactly three axes. Used only
  during brick baking (symmetrize + verify); not retained on the fabric.
  Known limitation — lift it when a non-threefold seed appears.
- **`face_twists`**: empirical 3-fold twist units per brick face, consumed
  by `label_off_axis_joints` so mirror-partner joints share numeric
  suffixes.
- **Working letters** (`seed_face_letters`): assigned A, B, C, … in *plan
  declaration order* to each seed face whose subtree contains a `Label`.
  They only last the build: at its end `name_limbs` turns each into a
  role and a member (`Fabric::limb_names`) — L/R from the face labels'
  `Side` under a mirror, A/B/C clockwise seen from above under a rotation
  — and short names follow when pretensing starts
  ([joint-naming.md](joint-naming.md)).
- **HeadlessHug's mirror symmetry was emergent, not declared**: working
  letters A/B (feet) and C/D (upper hubs) pair up only because of
  declaration order plus the `Side::Left`/`Side::Right` face labels —
  `LabelSymmetry` is what records the pairing.
- **OpenClaw's rotation map** (`open_claw_symmetry.rs::rotate_label_once`)
  is a hardcoded A→B→C string substitution, local to that module.

## The `LabelSymmetry` contract

`src/build/dsl/labelling.rs` defines the fabric's label-level symmetry
group, stored as `Fabric.label_symmetry: Option<LabelSymmetry>` and derived
once at build time (`build_phase.rs::seed_label_symmetry`):

```rust
pub enum LabelSymmetry {
    Cyclic { letters: Vec<char> },        // n-fold: letters[0] → letters[1] → …
    Mirror { pairs: Vec<(char, char)> },  // involution
}
```

Operations: `order()`, `map_letter()`, `map_label(JointLabel)`,
`orbit(JointLabel)`, `orbit_key(JointLabel)`.

Invariants:

1. **The group acts by pure letter permutation** on `JointLabel::OffAxis`
   and, after naming, on `Twin` (`L12` ↔ `R12`) and their bridle forms.
   No string parsing — `JointLabel` is the substrate. Once `name_limbs`
   has run, the letters it permutes are members: `[('L', 'R')]` for
   Headless Hug, `A → B → C` for Open Claw.
2. **`JointLabel::Axial` (`Z<n>`) labels are fixed points** under every
   operation (apex/on-axis joints are their own orbit).
3. **`Cyclic.letters` follows the seed brick's `cyclic_axes` order, not
   plan declaration order.** For OpenClaw these coincide (X/Y/Z → A/B/C),
   but the distinction matters: the cycle is a geometric fact of the
   brick, the declaration order an accident of the plan.
4. **Orbit key = minimum label of the orbit**: every member of
   {LA03.2, LB03.2, LC03.2} keys to `LA03.2`. The orbit is what
   `number_twins_by_build_order` gives one number, so the three become
   `A3`, `B3`, `C3`.

Derivation rules (in order):

- **Cyclic** when the seed declares `cyclic_axes` for the resolved role
  and the lettered seed faces map one-to-one onto those axes.
  (Propeller has six lettered faces over three axes → no match → falls
  through, correctly: its symmetry is richer than a single letter cycle.)
- **Mirror** when every lettered seed face has a distinct partner whose
  subtree `FaceLabel` set is identical with `Side` flipped
  (HeadlessHug: `(A,B)` feet, `(C,D)` upper hubs, by working letter).
  `mirrored_face_label` must be extended when new `Side`-carrying
  `FaceLabel` variants are added.
- **None** otherwise (HaloByCrane, Diamond, …) — absence means
  no symmetry machinery runs anywhere.

## Future Packer sketch (not implemented)

The OpenClaw packer (`open-claw-after` branch, `src/build/packer.rs`)
hardcoded threefold assumptions. Its generic successor consumes
`LabelSymmetry` instead:

- **Orbit grouping:** struts group by the `orbit_key` of their endpoint
  labels; one physical part description per orbit, quantity = orbit size.
- **Disassembly order:** sort orbits by (mean joint height descending,
  brick depth, position); process one orbit representative at a time,
  applying `map_label` to act on all members symmetrically.
- **Tethers/feet:** the number of anchor tethers = `order()` for `Cyclic`
  (three legs → three corner tethers), 2 for `Mirror`.
- Port guide for the rest (bendable cables, crane phases, tuning
  constants): `git diff e25d57c open-claw-after -- src/build/packer.rs`.

## Migration note

`rotate_label_once` in `open_claw_symmetry.rs` could delegate to
`LabelSymmetry::map_label`, but it works and is test-covered — migrate
only when that file is next touched for other reasons.
