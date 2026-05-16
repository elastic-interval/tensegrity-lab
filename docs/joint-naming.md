# Joint Naming

How every joint in a fabric — and especially every joint in the engineer's CSV
— gets a unique, symmetry-revealing label. This is the canonical reference for
joint identifiers; see `docs/csv-handoff.md` for the broader CSV format and
`src/fabric/joint_path.rs` plus `src/fabric/mod.rs::Fabric::joint_label` for
the implementation.

## The two label shapes

Every joint name is one of two shapes:

1. **Path joint** — a joint that lives in a brick *added during construction*
   (legs, prisms, hubs grown from a seed face). Looks like `AX4YZ1`,
   `BX1Z4`, `YZ0`, `CX2Z3`.
2. **Seed joint** — a joint that is one of the original 12 (for the
   OmniSymmetrical seed) created at fabric-build time. Looks like `BAA`,
   `BOC`, `TAA`, `TOC` — short, 3 characters.

The two share no overlap; you can always tell which kind a label is by
inspection.

## Path joints (`AX4YZ1`-style)

Read left-to-right as a recipe for how the brick containing this joint was
reached from the seed:

- **`A` … `V`** — a face choice. The construction path branched off this
  face of the previous brick. The first three face choices in a plan get
  the letters `A`, `B`, `C` (in order of definition in the plan's
  `.faces([...])` call), so for OpenClaw `A` = OmniBotX-grown leg,
  `B` = OmniBotY-grown leg, `C` = OmniBotZ-grown leg.
- **`X` + integer** — column step run. `X4` means "four consecutive
  column-bricks stacked along this direction." A `1`-step run is still
  written `X1` (so the parser is unambiguous).
- **`Y`** — a prism was added on top of the last brick.
- **`Z` + integer** — the local joint index *inside* the brick at the end
  of the path. For a single-twist brick that's `Z0..Z5` (six joints); for an
  OmniSymmetrical brick `Z0..Z11`.

Examples:

| Label | Read as |
|---|---|
| `AX4Z1` | from the seed, leg A face, four column-bricks, local joint 1 of the last brick |
| `AX4YZ1` | … same, but a prism on top: local joint 1 of the prism brick |
| `BX1Z4` | leg B, one column-brick, local joint 4 |
| `YZ0` | no face-letter path: a prism directly on top of the seed (the OpenClaw apex prism), local joint 0 |

Uniqueness is mechanical: two distinct joints differ either in branch (path)
or in local index. The test `test_open_claw_joint_paths_unique` asserts this
at every build.

## Seed joints (`BAA`-style)

The 12 joints of the OmniSymmetrical seed are the only joints with an empty
construction path. Naming them `Z0..Z11` (their raw local indices) hid the
3-fold symmetry the rest of the structure inherits. So seed joints get a
symbolic label that exposes their place in the symmetry.

Each label is 3 characters: a **category** and a **leg letter**.

**Category** (2 chars) — derived from the joint's role in the brick's
symbolic definition (`src/build/dsl/brick_library/omni.rs`). Reflects axial
position along the strut tube the joint lives on:

| Code | Meaning | Altitude band |
|---|---|---|
| `BA` | BotAlpha — bottom of a "bot" strut | floor |
| `BO` | BotOmega — top of a "bot" strut | upper-mid |
| `TA` | TopAlpha — bottom of a "top" strut | lower-mid |
| `TO` | TopOmega — top of a "top" strut | apex |

Note that the altitudes are a *consequence* of the brick's twist geometry —
not an input. The categories are pure symbolic identifiers.

**Leg letter** (1 char) — `A`, `B`, or `C`, derived from the brick's
**cyclic axis order** declared at the orientation. OmniSymmetrical with
`Seed(1)` declares `[X, Y, Z]`, so axis X → A, Y → B, Z → C (matching the
leg-face order OpenClaw uses).

The 12 seed labels for OpenClaw + Seed(1):

| Local index | JointName (brick-symbolic) | Label |
|---|---|---|
| 0 | BotAlphaX | **BAA** |
| 1 | BotOmegaX | **BOA** |
| 2 | TopAlphaX | **TAA** |
| 3 | TopOmegaX | **TOA** |
| 4 | BotAlphaY | **BAB** |
| 5 | BotOmegaY | **BOB** |
| 6 | TopAlphaY | **TAB** |
| 7 | TopOmegaY | **TOB** |
| 8 | BotAlphaZ | **BAC** |
| 9 | BotOmegaZ | **BOC** |
| 10 | TopAlphaZ | **TAC** |
| 11 | TopOmegaZ | **TOC** |

The 6 seed struts read very cleanly in this scheme:

- Bot struts: `BAA↔BOA`, `BAB↔BOB`, `BAC↔BOC`
- Top struts: `TAA↔TOA`, `TAB↔TOB`, `TAC↔TOC`

## Why this exposes 3-fold symmetry

Rotating OpenClaw by 120° about its central axis maps leg A → B → C → A.
Under that rotation:

- A path joint like `AX1Z2` rotates to `BX1Z2` and then to `CX1Z2`
  (only the leading leg letter rotates).
- A seed joint like `BAA` rotates to `BAB` and then to `BAC` (only the
  trailing leg letter rotates).

So a triple of rotationally-equivalent cables is *visually obvious* — every
joint name in the triple is the same string with the leg letter rotated.
Example triple:

```
BAA ↔ AX1Z2
BAB ↔ BX1Z2
BAC ↔ CX1Z2
```

The three cables in such a triple are identical by symmetry, so they should
have identical lengths up to floating-point residue. The test
`test_open_claw_cable_triples` groups all 180 OpenClaw cables into 60 such
triples and asserts the per-triple length spread stays under 5 mm; the
actual worst-case is well under 1 mm.

This is what makes the cable-fabrication order tractable: **three of each
of 60 lengths**.

## Where the naming comes from (no coordinates involved)

Every step is symbolic, derived from DSL declarations:

1. The brick declares its symbolic joints via `pushes_x/_y/_z` (e.g.
   OmniSymmetrical has joints `BotAlphaX`, `BotOmegaX`, … `TopOmegaZ`).
   Local indices `0..11` come from creation order: explicit joints first,
   then `(alpha, omega)` per push.
2. The brick orientation (e.g. `Seed(1)`) declares its **cyclic axis
   order** via `.cyclic_axes_for(role, [axes])` on the prototype. For
   OmniSymmetrical's `Seed(1)`: `[X, Y, Z]`.
3. When the root Hub is processed during build (`build_phase.rs`), the DSL
   wraps `(brick_name, role)` into an `OmniSeedLabeller`
   (`src/build/dsl/labelling.rs`) and installs it as
   `fabric.labeller: Option<Arc<dyn JointLabeller>>`.
4. `Fabric::joint_label(key)` dispatches to the installed labeller. The
   labeller walks: local index → `JointName` via the brick's `joints` and
   `pushes`; `JointName.omni_decode()` → `(category, axis)`; cyclic axes →
   leg letter; concatenate. If no labeller is installed (algorithmic
   fabrics), `joint_label` falls back to `JointPath::Display`.
5. The CSV export, picking display, etc. call `joint_label` for every joint
   identifier they write.

No floating-point geometry is consulted at any step. Different runs of the
same plan produce byte-identical names. A different plan that uses the same
seed gets the same seed-joint names.

## Extending to other bricks / orientations

For a (brick, orientation) pair to participate in symbolic seed naming:

1. The brick prototype must declare its 3-fold cyclic axis order under that
   orientation via `.cyclic_axes_for(role, [axes])`.
2. Either reuse `OmniSeedLabeller` (works for any Omni-shaped joint set —
   BotAlpha/Omega × TopAlpha/Omega × X/Y/Z that `JointName::omni_decode`
   recognises) or write a new `impl JointLabeller` and have the build path
   install it on the fabric.

If no labeller is installed (e.g. a non-OmniSymmetrical seed, or any of the
algorithmic fabrics — sphere, klein, mobius, evolution), `joint_label` falls
back to `JointPath`'s `Display`, which renders seed joints as `Z<n>`.

## What the engineer sees in the CSV

Push and pull rows quote joint labels in the `AlphaJoint` and `OmegaJoint`
columns. With the symmetric naming, the seed-end of every cable starts with
a `BA`/`BO`/`TA`/`TO` prefix, and its leg letter (`A`/`B`/`C`) lines up with
the leg letter at the destination end. Cable triples for the order list are
identifiable purely by rotating leg letters.

The rendering rows (`axial`, `radial`, `hinge`) still describe a single
joint at both ends; the symbolic name appears in both columns identically
when the link is within a seed-joint's stack.
