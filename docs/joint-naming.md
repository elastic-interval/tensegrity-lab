# Joint Naming

Names exist to make the physical build understandable. Someone holding a
strut, a cable or a disc should be able to tell from its name what it is,
roughly where it goes, and which other parts are its twins. Names are cheap
to change; a confusing name costs time on site. So the scheme follows the
structure's symmetry, and is free to change when the structure teaches us
something.

This document describes the naming process, for bilateral fabrics
(Headless Hug: mirror symmetry) and trilateral ones (Open Claw: threefold
rotation) alike, then how the code does it.

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

### 3. One number per set of twins, counted bottom to top

Every set of twins gets one number, shared by all its members, counted from
the ground up by the set's mean height: number 1 is the lowest set. Because
the number belongs to the set, every member gets the same number however
its own height rounds. Sets at equal heights keep their build order.

### 4. The name

`<member><number>`, for example:

| Fabric | Name | Reads as |
|---|---|---|
| Headless Hug | `L12` / `R12` | the 12th set of twins from the ground, left / right |
| Open Claw | `A12` / `B12` / `C12` | the 12th set from the ground, sectors A / B / C |

That's two or three characters: Headless Hug runs to `L62`, Open Claw to
`C30`. Arm, leg and torso aren't in the name; the number says roughly how
high, and twins differ only in their letter. Numbers can skip: a hinge
merges two joints and its set's number goes with it.

The full name is also what gets engraved: one name for a part
everywhere, on the steel, in the CSV and in the app.

### 5. Joints the symmetry fixes are Z, numbered bottom to top

A joint on the mirror plane or the rotation axis is `Z<n>`, numbered on its
own from the ground up: `Z1` is the lowest. A Z name says "on the centre
line" and roughly how high.

### 6. Everything derived inherits from its joint

| Thing | Name | Example |
|---|---|---|
| Cable end | `<joint>.<slot>`: the joint, then its connector slot | `L12.2` |
| Cable | its two ends, smaller name first | `L12:R30` |
| Bridle node (plate) | `<host>b`, the joint its swivel hangs from | `L48b` |
| Custom disc | named for its host joint | disc at `Z1` |
| Hinge | the name it keeps, or a Z if it lands on the centre line | `Z2` |

Because every derived name contains a joint name, symmetry carries over:
the mirror of `L48b` is `R48b`.

### 7. Members can be physical markers

The member letter can double as a colour tag or paint mark per side or
sector. Sorting parts for assembly is then a matter of matching letters
and colours.

## How the code does it

Headless Hug and Open Claw are named this way. Fabrics without a symmetry
(Halo by Crane, the algorithmic fabrics) keep one letter per limb
(`A01.1`) or fall back to the raw `JointPath`.

1. **During the build** each joint gets a build-time address: a working
   letter per limb, brick depth and position (`label_seed_joints`,
   `label_off_axis_joints` in `src/build/dsl/build_phase.rs`). The seed
   records each letter's member in `Fabric::limb_names`: under a mirror
   symmetry from the face labels' `Side`; under a cyclic one by going
   clockwise round the legs seen from above, starting with the first leg
   letter.
2. **At the end of the build** (`assign_axial_labels`) leftover joints with
   a strut become `Z<n>`, mirror pairs among them are torso twins, and
   `name_limbs` turns working letters into members (`AL02.3`, `TR00.2`,
   `LB05.1`). `LabelSymmetry` permutes members: `[('L', 'R')]` for Headless
   Hug, `A → B → C` for Open Claw.
3. **When the pretense phase starts** the fabric stands, and names become
   final:
   - `number_twins_by_height` gives every set of twins its short name.
   - The plan's hinges run, by those names.
   - `number_axial_by_height` numbers every `Z`, hinge pins included.
   - The plan's omits and bridles run, by the final names.
4. **Afterwards**, joints added later take the next free `Z` number:
   Headless Hug's two guy-line anchors on the ground are `Z7` and `Z8`,
   above the structure's `Z1`–`Z6`.

Heights are measured when the fabric first stands, before pretensing,
falling and settling. The settled shape keeps nearly the same order, but
not exactly: in Headless Hug 18 of the consecutive twin numbers are out of
height order on the settled shape, in Open Claw 3. Numbering after
settling would be exact, but the plan's hinges and bridles name joints
before then.

Node plates (`<host>b`) and cable ends (`<joint>.<slot>`) follow
their joint. Everything downstream reads names through
`Fabric::joint_label`, and the connector code finds mirror images through
`LabelSymmetry`. Code that parses names by shape: `rotate_label_once` in
`src/open_claw_symmetry.rs` cycles the sector letter, as does
`scripts/build_cable_order.py`. `scripts/build_assembly.py` used the brick
digit to sort struts into hub, column and leg-end prism; short names no
longer carry it, so that step needs another source.

## Decisions

Settled 2026-10-28:

- **Short names:** `<member><number>`; the role letter is dropped.
- **Twin numbers** follow height, counted from the ground up; **Z** is
  numbered on its own, also from the ground up.

Settled 2026-10-22 (the parts still in force):

- **Front:** L/R are the figure's own left and right. Open Claw's
  sectors start with today's leg A and run clockwise seen from above.
- **Z numbering:** bottom to top by height on the finished, standing
  fabric.
- **Engraving:** the full name, e.g. `L12`, and `L12.2` for a cable end.
  One name for a part everywhere.

Which way each figure faces: Headless Hug's plan labels faces with
`Side::Left` / `Side::Right`, and these are the figure's own left and
right.
