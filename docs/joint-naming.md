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

### 3. One number per set of twins, counted in build order

Every set of twins gets one number, shared by all its members, counted in
the order the structure grows: by brick, from the seed outward, then by
limb and position within the brick. Number 1 is in the seed. The numbers
come from the brick tree alone, never from where joints happen to be, so
reshaping, pretensing or settling the fabric can't change them. They
change only when the build itself does.

### 4. The name

`<member><number>`, for example:

| Fabric | Name | Reads as |
|---|---|---|
| Headless Hug | `L12` / `R12` | the 12th set of twins built, left / right |
| Open Claw | `A12` / `B12` / `C12` | the 12th set built, sectors A / B / C |

That's two or three characters: Headless Hug runs to `L62`, Open Claw to
`C30`. Arm, leg and torso aren't in the name; a low number is near the
seed, and twins differ only in their letter. Numbers can skip: a hinge
merges two joints and its set's number goes with it.

The full name is also what gets engraved: one name for a part
everywhere, on the steel, in the exports and in the app.

### 5. Joints the symmetry fixes are Z, numbered in build order

A joint on the mirror plane or the rotation axis is `Z<n>`, numbered on its
own in the order it was labelled: at the end of the build, then any hinge
pins. A Z name says "on the centre line".

### 6. Everything derived inherits from its joint

| Thing | Name | Example |
|---|---|---|
| Cable end | `<joint>.<slot>`: the joint, then its connector slot | `L12.2` |
| Cable | its two ends, smaller name first | `L12:R30` |
| Bridle node (plate) | `<host>b`, the joint its hanger hangs from | `L48b` |
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
3. **When the pretense phase starts** names become final:
   - `number_twins_by_build_order` gives every set of twins its short name.
   - The plan's hinges run, by those names.
   - `number_axial_by_build_order` numbers every `Z`, hinge pins included.
   - The plan's omits and bridles run, by the final names. A step naming
     a joint or cable the fabric doesn't have is reported
     (`FabricPlanExecutor::plan_failures`), and the tests require every
     plan to apply cleanly.
4. **Afterwards**, joints added later take the next free number. A guy
   line's ground anchor from a twin joint is a twin too, one number for the
   same line on each side: Headless Hug's four anchors are `L63` / `R63`
   (forward, from `L14` / `R14`) and `L64` / `R64` (backward). An anchor from
   a `Z` joint takes the next free `Z`.

### In a plan

A plan names joints by twin set, the way they're engraved, and every entry
stands for itself and all its images under the fabric's symmetry, so a plan
can't be asymmetric (`src/build/dsl/joint_names.rs`):

| Written | Means |
|---|---|
| `LR(12)` | Headless Hug's twins L12 and R12 |
| `ABC(12)` | Open Claw's twins A12, B12 and C12 |
| `Z(4)` | Z4, which the symmetry maps onto itself |
| `LR(48).b()` | the bridle nodes L48b and R48b |
| `LR(2) - LR(3)` | each with its own member: L2–L3 and R2–R3 |
| `LR(5) \| LR(2)` | across the mirror: L5–R2 and R5–L2 |
| `LR(13) \| LR(13)` | its own image: the one link L13–R13 |
| `Z(4) - LR(5)` | Z4–L5 and Z4–R5 |
| `ABC(2) >> ABC(1)` | one sector on, A → B: A2–B1, B2–C1, C2–A1 |
| `ABC(2) << ABC(3)` | one sector back: A2–C3, B2–A3, C2–B3 |
| `LR(5).bridle(Z(4), Z(2))` | R5's and L5's cables to Z4 and Z2 through bridle nodes |

The types keep the notation honest: `|` exists only between mirror names
and `>>`, `<<` only between threefold ones, so a plan can't cross a mirror
it doesn't have; and mirror names in a threefold fabric fail when the plan
runs. Actuators take links too: `phase(Pct(0.0)).interval(LR(14) - LR(63))`
drives both twins' guy lines, and Open Claw's `between(ABC(15) >> ABC(28))`
adds one actuator per leg. A guy line's ground offset is (outward from the mirror plane,
forward), so it means the same for both twins. The executor applies each
entry's images in the symmetry's order, entries in the order written:
that order shifts the settled stance, so reordering a plan's entries is a
change to the fabric.

Node plates (`<host>b`) and cable ends (`<joint>.<slot>`) follow
their joint. Everything downstream reads names through
`Fabric::joint_label`, and the connector code finds mirror images through
`LabelSymmetry`. Code that parses names by shape: `rotate_label_once` in
`src/open_claw_symmetry.rs` cycles the sector letter.

## Decisions

Settled 2026-10-29:

- **Numbers follow build order,** twins and `Z` alike, not height. Height
  numbering was recounted on every build, so a change to the shape could
  move a name onto another joint and the plan's hinges, omits and bridles
  with it.

Settled 2026-10-28 (the parts still in force):

- **Short names:** `<member><number>`; the role letter is dropped.

Settled 2026-10-22 (the parts still in force):

- **Front:** L/R are the figure's own left and right. Open Claw's
  sectors start with today's leg A and run clockwise seen from above.
- **Engraving:** the full name, e.g. `L12`, and `L12.2` for a cable end.
  One name for a part everywhere.

Which way each figure faces: Headless Hug's plan labels faces with
`Side::Left` / `Side::Right`, and these are the figure's own left and
right.
