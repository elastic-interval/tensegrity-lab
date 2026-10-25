# Joint Naming

Names exist to make the physical build understandable. Someone holding a
strut, a cable or a disc should be able to tell from its name what it is,
roughly where it goes, and which other parts are its twins. Names are cheap
to change; a confusing name costs time on site. So the scheme follows the
structure's symmetry, and is free to change when the structure teaches us
something.

This document describes the general naming process, for bilateral fabrics
(Headless Hug: mirror symmetry) and trilateral ones (Open Claw: threefold
rotation) alike, then how far the code has got toward it.

## The idea: one symmetry, two parts to every name

Every symmetric fabric has a symmetry group G acting on its joints: a
reflection for a bilateral fabric, a 120° turn for a trilateral one. G
sorts the joints into orbits, sets of joints that are copies of each
other. A name therefore has two jobs:

- **Which kind of joint**: the orbit, the same for all copies.
- **Which copy**: the member, which differs between copies.

Twins share everything except the member, so you can spot them at a
glance, cut cables in sets, and check one side against the other. A joint
that G maps onto itself (on the mirror plane, or on the rotation axis) is
an orbit of one and has no member.

## The process

### 1. The seed fixes the symmetry and the members

The seed brick and its orientation determine G and its generator. The
face labels in the plan (`Hand(Left)`, `Foot(Right)`, the cyclic faces
listed first) tell which faces map to which. From that orientation:

- **Bilateral:** members are **L** and **R**: the figure's own left and
  right, as the sculpture itself would say it, not as a viewer facing it
  sees it.
- **Trilateral:** members are **A**, **B**, **C**, clockwise seen from
  above, starting with the leg that is A today.

Which way the figure faces is recorded once per fabric in its plan.

### 2. Orbits come from the brick tree, not from positions

G maps limbs onto limbs (whole subtrees of bricks) and, within a brick,
joint positions onto joint positions. The existing twist shift already
lines up twin joints' positions. So every joint's orbit and member follow
from its place in the build tree: no measuring, no tolerances, the same
result every run.

### 3. A joint's address within its orbit

- **Role:** what the limb is, as a body word: **A** arm, **L** leg,
  **T** torso for the seed brick itself. Open Claw's three limbs are legs,
  so leg is L in both fabrics.
- **Depth:** which brick along the limb, 00 for the seed, counting
  outward.
- **Position:** which joint within the brick. Twins get the same number.

### 4. The name

`<role><member><depth>.<position>`, for example:

| Fabric | Name | Reads as |
|---|---|---|
| Headless Hug | `AL02.3` / `AR02.3` | arm, left / right, brick 2, joint 3 |
| Headless Hug | `LL01.4` / `LR01.4` | leg, left / right, brick 1, joint 4 |
| Headless Hug | `TL00.2` / `TR00.2` | torso, left / right, joint 2 |
| Open Claw | `LA02.3` / `LB02.3` / `LC02.3` | leg, sectors A / B / C, brick 2, joint 3 |
| Open Claw | `TA00.1` / `TB00.1` / `TC00.1` | torso, sectors A / B / C, joint 1 |

The full name is also what gets engraved: one name for a part
everywhere, on the steel, in the CSV and in the app.

### 5. Joints the symmetry fixes are Z, numbered bottom to top

A joint on the mirror plane or the rotation axis is `Z<n>`, numbered by
height from the ground up, measured on the finished, standing fabric. A Z
name says "on the centre line" and roughly how high. Torso joints that
lie on the centre line are Z, not T.

### 6. Everything derived inherits from its joint

| Thing | Name | Example |
|---|---|---|
| Cable end | `<joint>.<slot>`: the joint, then its connector slot | `AL02.3.2` |
| Cable | its two ends, smaller name first | `AL02.3:LL01.4` |
| Cable through clamps | its two ends, via the clamps in order | `AL02.3:TL00.2b:LL01.4` |
| Bridle node (plate or clamp) | `<host>b`, the joint its shackle hangs from | `LL02.4b` |
| Custom disc | named for its host joint | disc at `Z9` |
| Hinge | the smaller of its two names, or Z if it lands on the centre line | `Z13` |

Because every derived name contains a joint name, symmetry carries over:
the mirror of `LL02.4b` is `LR02.4b`.

### 7. Members can be physical markers

The member letter can double as a colour tag or paint mark per side or
sector. Sorting parts for assembly is then a matter of matching letters
and colours.

## How the code does it

Headless Hug and Open Claw are named this way. Fabrics without a symmetry
(Halo by Crane, the algorithmic fabrics) keep one letter per limb
(`A01.1`) or fall back to the raw `JointPath`.

1. **During the build** each limb is labelled with a working letter, one
   per seed face, plus brick depth and position (`label_seed_joints`,
   `label_off_axis_joints` in `src/build/dsl/build_phase.rs`). The seed
   records each letter's role and member in `Fabric::limb_names`: under a
   mirror symmetry from the face labels (`Hand` → arm, `Foot` → leg, and
   their `Side`); under a cyclic one every limb is a leg.
2. **At the end of the build** (`assign_axial_labels`):
   - Leftover joints with a strut become `Z<n>`.
   - Under mirror symmetry, Z joints that are each other's reflection are
     torso joints, `TL00.n` / `TR00.n` (`pair_mirrored_axial_labels`).
   - `name_limbs` turns working letters into role and member: brick 0 is
     torso, other bricks take their limb's role. For a cyclic fabric the
     sectors are assigned clockwise seen from above, starting with the
     first leg letter. `LabelSymmetry` is rewritten to permute members:
     `[('L', 'R')]` for Headless Hug, `A → B → C` for Open Claw.
3. **When the pretense phase starts**, the fabric stands. The plan's hinges
   run first, then every `Z` is renumbered bottom to top by height
   (`number_axial_by_height`), and only then do the plan's omits and
   bridles run. So plans name `Z` joints by their final numbers.
4. **Afterwards**, joints added later take the next free `Z` number:
   Headless Hug's two guy-line anchors on the ground are `Z7` and `Z8`,
   above the structure's `Z1`–`Z6`.

Node plates and clamps (`<host>b`) and cable ends (`<joint>.<slot>`) follow their joint.
Everything downstream reads names through `Fabric::joint_label`, and the
connector code finds mirror images through `LabelSymmetry`. Code that
parses names by shape: `rotate_label_once` in `src/open_claw_symmetry.rs`
cycles the sector (second character), as do `scripts/build_cable_order.py`
and `scripts/build_assembly.py`.

## Decisions

Settled 2026-10-22:

- **Role letters:** **A** arm, **L** leg, **T** torso (the seed brick).
  Body words, and leg reads the same in both fabrics.
- **Front:** L/R are the figure's own left and right. Open Claw's
  sectors start with today's leg A and run clockwise seen from above.
- **Z numbering:** bottom to top by height on the finished, standing
  fabric.
- **Engraving:** the full name, e.g. `AL02.3`, and `AL02.3.2` for a cable
  end. One name for a part everywhere; the three-character form Open Claw
  once used is retired.

Which way each figure faces: Headless Hug's plan labels faces with
`Side::Left` / `Side::Right`, and these are the figure's own left and
right.
