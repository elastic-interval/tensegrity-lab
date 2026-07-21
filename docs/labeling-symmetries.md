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
- **Letters** (`seed_face_letters`): assigned A, B, C, … in *plan
  declaration order* to each seed face whose subtree contains a `Label`.
- **HeadlessHug's mirror symmetry was emergent, not declared**: A/B
  (feet) and C/D (upper hubs) pair up only because of declaration order
  plus the `Side::Left`/`Side::Right` face labels — until `LabelSymmetry`,
  the pairing was recorded nowhere.
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

1. **The group acts by pure letter permutation on `JointLabel::OffAxis`.**
   No string parsing — `JointLabel` is the substrate.
2. **`JointLabel::Axial` (`Z<n>`) labels are fixed points** under every
   operation (apex/on-axis joints are their own orbit).
3. **`Cyclic.letters` follows the seed brick's `cyclic_axes` order, not
   plan declaration order.** For OpenClaw these coincide (X/Y/Z → A/B/C),
   but the distinction matters: the cycle is a geometric fact of the
   brick, the declaration order an accident of the plan.
4. **Orbit key = minimum label of the orbit** — engraving-friendly:
   every member of {A03.2, B03.2, C03.2} keys to `A03.2` (think
   "A03.2 ×3" on a parts list).

Derivation rules (in order):

- **Cyclic** when the seed declares `cyclic_axes` for the resolved role
  and the lettered seed faces map one-to-one onto those axes.
  (Propeller has six lettered faces over three axes → no match → falls
  through, correctly: its symmetry is richer than a single letter cycle.)
- **Mirror** when every lettered seed face has a distinct partner whose
  subtree `FaceLabel` set is identical with `Side` flipped
  (HeadlessHug/MinimalMan: `(A,B)` feet, `(C,D)` upper hubs).
  `mirrored_face_label` must be extended when new `Side`-carrying
  `FaceLabel` variants are added.
- **None** otherwise (HaloByCrane, Flagellum, Diamond, …) — absence means
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
`LabelSymmetry::map_label`, but it works, is test-covered, and the CSV
byte-identity guard makes churn there expensive — migrate only when that
file is next touched for other reasons.
