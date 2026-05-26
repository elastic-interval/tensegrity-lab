# Joint Naming

Every joint in a 3-fold-symmetric fabric — and every joint in the engineer's
CSV — gets a short, self-describing label. The labels are optimised for
engraving onto physical parts: three characters in the common case, every
character carries information, and three rotational copies of a joint
differ only in the leg letter.

This is the canonical reference for joint identifiers. See
`src/build/dsl/labelling.rs` (`SymmetricOrbitLabeller`) for the
implementation and `docs/csv-handoff.md` for the broader CSV format.

## The two label shapes

| Shape                  | Meaning                                              | Examples              |
|------------------------|------------------------------------------------------|-----------------------|
| `<leg><brick><pos>`    | Off-axis joint. `leg ∈ {A, B, C}`; `brick` and `pos` are integers. | `A04`, `A14`, `C52`   |
| `Z<n>`                 | On-axis singleton. `n` is an integer.                | `Z1`, `Z2`            |

The two shapes share no overlap; you can tell at a glance which is which.

## Off-axis labels (`<leg><brick><pos>`)

Three components, each one character in the common case:

- **`leg`** ∈ `{A, B, C}`. Distinguishes the three rotational copies of a
  joint. Assigned topologically:
    - For seed joints (path with empty branches), the leg letter is the
      seed brick's cyclic axis the joint sits on — mapped via the brick's
      `cyclic_axes` declaration (axis 0 → A, axis 1 → B, axis 2 → C).
    - For leg joints (`branches[0] ∈ {0, 1, 2}`), the leg letter is
      `branches[0]` mapped to A/B/C. *This convention requires the DSL
      author to list the three cyclic faces first in `.faces([...])`, in
      the same order as `cyclic_axes`.*
- **`brick`** is a digit naming which sub-brick of the leg the joint sits
  in:
    - `0` = the seed brick (the root hub everything grows from).
    - `1..N` = column-bricks along the leg, with `1` being the column-brick
      closest to the seed and `N` the furthest along the column.
    - `N+1` = the leg-end prism if any (in OpenClaw built with
      `column(4).prism(...)`, this is brick `5`).
  Computed from the path: count of `COLUMN_MARKER` + `PRISM_MARKER` in the
  branches past the leg letter.
- **`pos`** is the 1-indexed within-brick position of the joint:
    - For seed joints, this is the `OmniCategory` altitude rank:
      `1 = TopOmega`, `2 = BotOmega`, `3 = TopAlpha`, `4 = BotAlpha` (under
      Seed(1) orientation; both `Omega` ends sit above both `Alpha` ends
      after `down_rotation`).
    - For column-brick and prism joints, this is `local_index + 1`
      (the prototype's oven-creation order within the brick).

The result: rotational triples are visually obvious. The three rotational
copies of a cable from a `<leg>14` joint (column-brick 1, position 4) to a
`<leg>35` joint (column-brick 3, position 5) read:

```
A14:A35
B14:B35
C14:C35
```

For three identical cables, the engineer cuts three of one length and
engraves each with a different leg letter — `brick` and `pos` are
identical.

### Reading a label at a glance

- `A04` = leg A, **seed brick** (`brick=0`), position 4 → `BotAlpha`.
- `A11` = leg A, **column-brick 1**, position 1.
- `A35` = leg A, **column-brick 3**, position 5.
- `A52` = leg A, **leg-end prism** (`brick=5` for OpenClaw), position 2 →
  the foot of leg A.

## Cable-end labels (`<joint>.<slot>`)

A cable has two ends; each end is engraved with a label of the form
`<joint>.<slot>` — the joint it terminates at, dot, the slot on that
joint's connector stack. Examples: `A04.2`, `B15.3`, `Z1.1`. Apex
singletons get cable ends with slots `1`/`2`/`3` (three slots on the
single apex push), so the three rotational copies of an apex-attached
cable get distinct labels like `Z2.1:B01.1`, `Z2.2:A01.1`, `Z2.3:C01.1`.

This is the form the engineer engraves on each of the 360 cable ends
(180 cables × 2 ends). It appears in the cable-order CSV's `Members`
column, in the cables-by-length labelling worksheet at the start of the
assembly PDF, and as the cable target shown next to each disc on the
per-strut pages of the assembly PDF.

## On-axis labels (`Z<n>`)

Joints built off an axis-fixed face (e.g. OpenClaw's apex prism on the
`OmniTop` face) are singletons under the 120° rotation — they map to
themselves. Each gets the prefix `Z` and an integer index. For OpenClaw
there are exactly two: `Z1` (the topmost apex joint) and `Z2` (directly
below it). These are the endpoints of the central apex push.

Cables landing on a `Z<n>` joint don't form clean rotational triples by
length: three rotational copies converge on the same push end and therefore
occupy three different connector slots, so their lengths differ by the disc
step. The CSV doesn't try to enforce mean-length collapse on apex-attached
triples (see `is_apex_axis_label` in `src/open_claw_symmetry.rs`).

## Non-structural joints (fallback)

A few joints in the fabric aren't endpoints of any push interval — face
midpoints that exist only to define a face's centre. They get no orbit
label; `joint_label` falls back to the raw `JointPath::Display`. These
joints never appear in the engineer's CSV, so the fallback shape is
internal-only.

For fabrics with no installed labeller (algorithmic fabrics — sphere,
klein, mobius), every joint uses the `JointPath::Display` fallback. The
orbit labeller is only installed when the build path uses a seed brick.

## Why this scheme

Three constraints from the OpenClaw build:

1. **Engraving cost.** Doubling label length doubles engraving time per
   part. Most labels are 3 chars (2 chars for axis singletons), the same
   length as the previous orbit-numbered scheme they replaced — but more
   informative.
2. **Symmetry-aware fabrication.** Cables are cut in triples. The leg-letter
   pattern (`A` ↔ `B` ↔ `C`, same `brick` and `pos`) lets the engineer
   visually pattern-match triples in the CSV.
3. **Self-describing for downstream tools.** The middle digit tells any
   reader (and the assembly PDF generator) which brick a joint lives in,
   without needing an external "orbit-index range" table. Twist mates are
   simply joints with the same `(leg, brick)` modulo the leg letter.

The earlier short-orbit scheme (`A1`, `A12`, `Z1`) was the most compact but
ambiguous — `A12` could be in any brick, and the assembly script had to
hard-code orbit-index ranges (5-28 = columns, 29-30 = prism) to recover the
brick. The brick digit restores that information without adding character
cost in the common case.

## How brick and position are assigned

The `SymmetricOrbitLabeller` is installed on the fabric when the root seed
brick is attached (see `src/build/dsl/build_phase.rs`). The whole scheme is
**topological** — joint positions are never read. The label is a
deterministic function of the joint's `JointPath` plus the seed brick's
prototype declarations.

On the first label request:

1. Collect all joints that are endpoints of any `Pushing` interval (face
   midpoints are filtered out).
2. Classify each joint:
    - **Empty branches**: seed joint. Decode `local_index → JointName` via
      the brick prototype's `joints` and `pushes`, then
      `JointName::omni_decode()` → `(OmniCategory, Axis)`. Brick = 0;
      position = altitude rank of the category; leg letter from
      `cyclic_axes.position(axis)`.
    - **`branches[0] ∈ {0, 1, 2}`**: leg joint. Leg letter from
      `branches[0]`; brick = count of `COLUMN_MARKER` + `PRISM_MARKER` in
      the rest of the branches; position = `local_index + 1`.
    - **`branches[0] ≥ 3` or `branches[0] == PRISM_MARKER`**: axis
      singleton. Gets a `Z<n>` label.
3. For axis singletons, sort by `(branches lex, ¬local_index)` — the
   bit-not on `local_index` puts an upward-facing prism's outer end (omega,
   local 1) at `Z1`. Assign `Z1`, `Z2`, … in order.
4. For leg labels, format `<leg><brick><pos>` directly from the orbit info
   — no global counter needed.

The result is cached and reused; the cache invalidates only when the
fabric's joint or interval count changes (so labels stay stable for the
entire CSV export pass even though physics may shift joint positions).

## Symmetry rotation in code

`src/open_claw_symmetry.rs` defines `rotate_label_once(label)` and
`canonical_push_key(a, b)` against this scheme:

- `rotate_label_once("A14") == "B14"`
- `rotate_label_once("B14") == "C14"`
- `rotate_label_once("C14") == "A14"`
- `rotate_label_once("Z1")  == "Z1"` (axis singletons are fixed points)

These power `apply_threefold_symmetry()` (the slot-assignment paste step
applied after the generic per-push algorithm) and the
`canonical_push_key()`-based grouping that drives both the symmetry
assertions and the CSV's group-mean length collapse.

## What the engineer sees in the CSV

The `AlphaJoint` and `OmegaJoint` columns of the engineering CSV quote the
joint labels directly. With the new scheme:

```
A14:A35     leg A, column 1 pos 4 → column 3 pos 5
B14:B35     leg B, same triple
C14:C35     leg C, same triple
```

The three rows for one cable triple are spaced apart in the CSV (rows
sorted by length, then by index), but the eye picks them out easily: same
brick digits, same position digits, only the leg letter cycles. For a
fabricator preparing 60 cable triples, that's three identical cuts per
brick+position.

## Downstream scripts

- `scripts/build_cable_order.py` rotates the leg letter only; `brick` and
  `pos` are unchanged under rotation.
- `scripts/build_assembly.py` reads the brick digit to classify each
  strut: `brick == 0` is hub, `brick > 0` is column-twist or leg-prism
  (distinguished by counting push intervals per brick: 3 = column, 1 =
  prism). Twist mates are struts sharing the same `(leg, brick)`. The
  same script also emits the cables-by-length labelling worksheet at the
  start of the assembly PDF.
