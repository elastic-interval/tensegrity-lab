# Crane lift: where the lifting eyes go

Headless Hug is to be built lying on its belly without its arms, have its
shoulders raised by the crane while the arms are built hanging, and then be
stood up and lifted into place. This note says which joints need a lifting
eye for that, from simulations of each lift (2026-11-15, scale 0.75,
332 kg). Taking it down is the same sequence backwards, through the same
poses, so the same numbers hold.

These are simulations of self-weight on a still day, with no load factors.
The rigging (eyes, shackles, slings, the dynamic factor for a crane) is the
rigger's and the engineer's to approve.

## Recommendation

Agreed with Gerald on 2026-11-15: L25 and R25, no second pair.

**Lifting eyes on L25 and R25**, the two highest joints, at the back of the
shoulders, 2.63 m apart. One pair of eyes, two sling legs to one hook, does
every lift:

| Stage | Pitch | Each leg | Leg to its strut | Worst cable | Worst strut margin |
|---|---:|---:|---:|---:|---:|
| Standing as built, for comparison | 0° | | | 5.1 kN | 1.49 |
| No arms, shoulders just off (eyes 1.6 m up) | 91° | 0.13 kN | 85° | 3.1 kN | 3.36 |
| No arms, eyes 3.0 m up | 75° | 0.52 kN | 58° | 3.4 kN | 2.95 |
| No arms, eyes 4.5 m up, on its feet alone | 62° | 0.76 kN | 45° | 4.7 kN | 1.86 |
| No arms, eyes 6.0 m up | 50° | 0.77 kN | 40° | 4.7 kN | 1.99 |
| Arms on, eyes 4.8 m up | 61° | 1.25 kN | 45° | 4.9 kN | 1.56 |
| Arms on, eyes 6.0 m up | 50° | 1.29 kN | 38° | 4.7 kN | 1.86 |
| Arms on, eyes 7.0 m up | 39° | 1.35 kN | 31° | 4.4 kN | 2.11 |
| Hanging free | 17° | 1.68 kN | 21° | 4.1 kN | 1.93 |
| On its feet, hook brought back over it | 4.5° | 0.48 kN | 17° | 4.5 kN | 2.08 |

Pitch is how far the figure leans forward from the way it stands; 90° is
face down. Strut margin is a strut's ideal buckling load over its force (as
in `docs/parts.md`). At no stage does a cable or strut carry more than it
does standing, and no cable passes the 5.1 kN working load.

What follows from the numbers:

- **The eyes carry 1.7 kN each at most**, 170 kg, hanging free on 4 m legs.
  The figure weighs 332 kg, 233 kg without its arms.
- **The load is not along the bolt.** It comes at 85° to the strut when the
  shoulders first leave the ground and 21° when hanging. A plain eye nut is
  not made for a sideways pull; a swivelling lifting point on the M12 bolt
  is. This is for the rigger.
- **Hanging free it leans 17° forward**, because L25 and R25 are 0.8 m
  behind its centre of gravity. Its feet stay lowest and touch down first.
- **The hook alone cannot stand it upright.** With the hook straight above
  the eyes it lifts off at 17° rather than coming upright. Once the feet are
  down, the crane brings the hook back over the figure as it lowers: that
  takes it to within 5° of upright with 0.5 kN left in each leg, and the
  guy lines take it from there.
- **Lying, the legs pass close to a cable.** On 4 m legs each comes within
  27 mm of cable R17–R6 (L17–L6) until the shoulders have risen about a
  metre; from there the nearest member is 500 mm away. Longer legs or a
  spreader for the first metre avoid it.
- **Lying without arms it rests on L3, R3, Z2, L29 and R29** (chest and
  thighs), with the shoulders already 1.6 m off the ground, when its feet
  can't slide; free to slide it sags until the fronts of the shoulders
  touch too (see the strut-by-strut build below). As the shoulders
  rise it rolls onto L16, R16, L32, R32, L46 and R46, and from about 65° it
  stands on its feet alone.
- **The arms can hang from about 70°**, eyes roughly 3.5 m up: the hands
  then clear the ground. At 60° they are about 2 m up.

### A level flight, if wanted: a second pair on L35 and R35

Hanging from L35 and R35 (the first strut of each arm, so only once the arms
are on) the figure hangs within 1° of upright, each leg 1.7 kN at 37° to its
strut, worst cable 4.5 kN, strut margin 1.71. With these it can be set down
square on both feet. (Run as four equal legs with L25 and R25, whose legs
then hang slack: L35 and R35 carry it all.) The legs pass 205 mm from cable L25–L8 (R25–R8).

## The others tried

Whole figure hanging free on 4 m legs:

| Eyes | Pitch | Each leg | Worst cable | Strut margin | Nearest member to a leg | Verdict |
|---|---:|---:|---:|---:|---:|---|
| L25 R25 | 17° | 1.68 kN | 4.1 kN | 1.93 | 500 mm | recommended |
| L27 R27 | 18° | 1.63 kN | 4.4 kN | 1.62 | 614 mm | as good, but only 1.1 m apart: less hold against rolling |
| L17 R17 | 31° | 1.67 kN | 4.4 kN | 1.69 | 680 mm | leans too far; best aligned for the first lift (6° to its strut) |
| L19 R19 | 40° | 1.70 kN | 5.0 kN | 1.35 | 474 mm | leans far too much |
| L24 R24 | 17° | 1.63 kN | 5.6 kN | 1.27 | 300 mm | four cables over 5.1 kN |
| L20 R20 | 2° | 1.71 kN | 4.5 kN | 1.72 | 104 mm | level, but the legs run through the shoulder |
| Z4 alone | −1° | 3.22 kN | 5.6 kN | 1.31 | 343 mm | level, but four cables over 5.1 kN, and one point: no hold against turning |
| L35 R35 | −1° | 1.70 kN | 4.5 kN | 1.71 | 205 mm | the level pair, arms on only |

Hinge joints (L6, L8, L10, Z5, Z6 and their twins) were left out: nothing
may end on a hinge.

Raising the shoulders without arms, L17 R17, L27 R27 and L19 R19 do as well
as L25 R25: legs under 1.2 kN, nothing above its standing force.

## Reproducing it

`src/crane_lift.rs`, test-only:

```bash
# The whole figure hanging free from each set of eyes
cargo test --release --lib hang_headless_hug -- --ignored --nocapture

# No arms, face down, shoulders raised until it stands on its feet
cargo test --release --lib raise_headless_hug_shoulders -- --ignored --nocapture

# Arms on, from 60° forward up to its feet, hook brought back, then lifted clear
cargo test --release --lib stand_headless_hug_up -- --ignored --nocapture

# Every joint from the top down, for choosing other eyes
cargo test --release --lib list_headless_hug_joints -- --ignored --nocapture
```

How it works: the figure is built and settled as usual, its guy lines taken
off. A hook joint is held in place above the eyes after every tick, with a
sling leg (6 mm cable stiffness) to each eye; the crane driver is assumed to
keep the hook straight above the eyes. The ground is the building site's
(`src/fabric/site.rs`, `Fabric::site`, `None` in every build), which holds
a joint that presses on it and lets go of one that lifts: the frozen
surface of the builds never lets go of a joint. The hook rises at
0.25 m/s and holds 10 s at each station; forces are averaged over 20 ms.

What it leaves out:

- The figure without arms is the finished figure with the arms' joints
  removed, so the torso's cables are at their final lengths. Lying, it is
  the standing shape turned over and let settle, not built up strut by
  strut.
- Wind, a swinging load, and the jolt of the crane starting and stopping.
- The feet are held where they touch. Really they slide or are chocked as
  the figure rotates on them.

## Strut by strut: assembly and disassembly

`src/build/assembly.rs` runs the whole sequence with the agreed eyes:
first taking the standing figure apart, then putting it together again.
In the app (Headless Hug or Open Claw, once it stands) that is the
**Take apart** button: it is taken apart and then, straight away, put
together again. It runs fast-forwarded, up to 10 times real time, as
fast as the physics allows while the picture keeps moving; **Slow**
brings it to real time, and the same button, now **Fast-forward**,
hurries it again. The tests in
`src/assembly.rs` run the same and print the worst cable and strut at
every step:

```bash
# From standing down to the net hanging on the hook
cargo test --release --lib assembly::take_headless_hug_apart -- --ignored --nocapture

# Taken apart, then from the hanging net to standing on its guy lines
cargo test --release --lib assembly::put_headless_hug_together -- --ignored --nocapture
```

### How it is done

- **Only struts come out.** All the cables and node plates stay, with
  the struts' caps and ring stacks on them: a connected net that lies
  slack where its struts are missing. Everything
  is at its final length.
- **Slack cables drape.** A cable stays a straight, stiff cable while
  either of its caps holds a strut. With both struts out it hangs as a
  chain of segments about 60 cm long, as in Open Claw's take-down, and
  with a strut back at either end it is a cable again, drawn to its
  length. Fork rods stay rigid. The net alone, hung as chains, weighs
  158 kg.
- **A strut is drawn in, then taken out.** It shortens, by a tenth of its
  length at most, until it carries nothing. Before it goes back in, its
  two caps are drawn, over 3 s, to where they were when it came out,
  over the ground as a crew would, the rest of the net left as it lies.
  A hanging arm is brought whole, the hook going along to where its legs
  are just taut over the eyes: its caps alone, moved where the arm cannot
  follow, stretched its cables to 40 kN. Brought whole, the cables L55–L50
  and R55–R50 still reach 13.9 kN at the end of one carry, held there
  until L55–L56 and R55–R56 go in. Then the
  strut goes in at the length its caps span, never more than its own, and
  is lengthened to its own over 4 s; the site holds it at its length all
  the while. A strut that still carries when drawn in (Z3–Z4 in the
  pelvis) goes back as it was. An arm strut goes in only with both its
  ends off the ground.
- **No jolts.** Moved in one tick, as at first, the net jumped: joints up
  to 260 m/s, most of it the sling legs snapping the eyes when the hook
  was let go over a net that had moved. Drawn over 3 s, no joint moves
  faster than 8 m/s while a strut goes in. The test prints any step where
  a joint is seen to move faster than 2 m/s.
- **Twins go together**, both drawn in and removed in one step. Struts on
  the mirror plane (L1–R1, L2–R2, Z1–Z2, Z3–Z4) go singly.
- **The order**, putting together: the pelvis (L1–R1, L2–R2, L3–L4 and
  R3–R4, Z1–Z2, Z3–Z4), then outwards brick by brick, legs before
  shoulders at the same distance; 44 struts make the figure without arms.
  Then the 20 arm struts from the shoulder out to the hands. Taking apart
  is exactly that backwards.
- **The hook stays on** L25 and R25 from the first step of taking apart to
  the last. It hangs slack while the figure lies on the ground.
- **The ground is slippery.** Nothing on it is held but the feet, and
  those only while they are on their mounts (base plates, or truss
  towers).
- **An arm strut comes off within reach, 1 m from the ground.** The hook
  only lowers, as a crane would, from 7.6 m to 4.9 m over the arms, until
  the lower end of the next arm strut is 1 m up, or the legs go slack;
  the crew can reach a strut there. Lowered until the struts were within
  15 cm of the ground, every arm strut from the hands in lay almost flat
  before it came out; holding one end just touching took the hook up and
  down between pairs, which a crane would not do.
- **A strut goes back in where it came out.** Lengthening struts in the
  net as it happens to lie does not work in the simulation: nothing there
  says which side of a cable a strut goes, and it tangles (cables at
  hundreds of kN). So each strut's caps are drawn to where they were when
  it came out. On site that is the numbers written on the caps.

### Taking apart

| Step | Struts | Leans | Worst cable | Worst strut margin |
|---|---:|---:|---:|---:|
| Hook on, guy lines still on | 64 | 0° | 5.1 kN | 1.50 |
| Guy lines off, feet still on their mounts | 64 | 6° | 4.2 kN | 2.0 |
| Feet let go: it hangs, feet sliding | 64 | 18° | 4.3 kN | 2.2 |
| Let down, hands out (L61–L62, R61–R62) | 62 | | 4.7 kN | 1.9 |
| Arms half out | 54 | | 3.6 kN | 2.4 |
| Arms out | 44 | | 4.5 kN | 1.8 |
| Shoulders let down, lying, hook slack | 44 | 95° | 4.5 kN | 1.76 |
| Feet struts out (L53–L54, R53–R54) | 42 | | 2.0 kN | 2.8 |
| Down to 26 struts | 26 | | 0.8 kN | 5.8 |
| Down to the pelvis | 6 | | 0 | |
| Every strut out, net lifted clear | 0 | | 0.4 kN | |

The net alone weighs 154 kg with its hardware. Lifted by L25 and R25
until no joint touches the ground, it is ready to go into the trailer.

### Putting together

It starts from that hanging net. Without its struts the net is a rope
about twice as long as the figure, and dragging it over the ground hardly
works, so it has to be drawn apart while it hangs and come down where it is
to lie:

- **The legs are drawn apart while it is let down.** A line runs to each
  foot (L54, R54) from a stake 4 m beyond where that foot lay, out from
  the middle of the net along the leg. It is reeled in at up to 1.5 m/s
  while it pulls less than about 3.8 kN, and stops with the foot 0.75 m
  past its mark: without its struts a leg is longer than it lay, and drawn
  that far it lies straight where it lay. Stopped on the mark, the feet
  arrived in 3 s and the legs between them fell in towards the middle,
  the knees 0.7 to 1.0 m from their places; drawn 0.75 m past, the knees
  land within 0.3 m.
- **The hook travels forward as it comes down**, over 30 s, to where it
  held the shoulders, so the net is laid from the feet towards the hands:
  legs, pelvis, chest. The feet come down about 15 s in, the knees after
  them, and the joints slide 0.03 m over the ground on average.
- **The arms are drawn apart the same way, spread eagle.** A line to the
  tip of each hand (L62, R62) runs from a stake 10 m beyond where the hand
  lay, out along the arm from the shoulders, and is reeled in as hard as
  the foot lines, stopping with the hand 3 m past its mark. Drawn only
  0.75 m past, the arms lay near their places but the shoulders between
  them fell in towards the middle, where the hook holds the eyes: the
  upper body landed 0.48 m inward of where it lay on average, 0.9 m at
  worst. Drawn 3 m past, it lands 1.25 m wider than it lay on average;
  only L4 (0.77 m) and Z6 (0.64 m) still land inward. Only how far past
  the mark matters: stakes 6 m or 10 m out, pulling 3.8 kN or three times
  that, give the same. The struts' caps are then drawn in to their places
  as the struts go in. Aimed from the middle of the net rather than the
  shoulders, the lines pulled the hands forward as well as out.
- **No net-wide carry.** The net used to be lifted and carried, every cap
  at once over 8 s, to where it lay: a jump that is not physics. Splayed
  as it now is, the net is left where it lands, and only each strut's
  two caps are drawn over the ground into place as the strut goes in.

So the hand tips want an eye each, as L25 and R25 have: **eyes on L62 and
R62** for the laying-out lines. The feet have their rod ends.

Then the struts go in, the same stages backwards
with the same forces, up to the arms:

| Step | Struts | Leans | Worst cable | Worst strut margin |
|---|---:|---:|---:|---:|
| Lying without arms, hook slack | 44 | 95° | 4.5 kN | 1.76 |
| Arms complete, hands on the ground | 64 | 75° | 5.0 kN | 1.68 |
| Raised until L25 and R25 are 7 m up, feet drawn to their mounts | 64 | 40° | 4.0 kN | 2.1 |
| Hook brought back over it | 64 | 2° | 3.8 kN | 2.1 |
| Upright on the hook | 64 | 0° | 4.0 kN | 1.95 |
| Guy lines on, still on the hook | 64 | 0° | 4.5 kN | 1.80 |
| Hook off, standing | 64 | 0° | 5.1 kN | 1.49 |

### What it shows

- **Nothing passes the working load.** The worst cable anywhere is the
  5.1 kN the thigh cables L45–L30 and R45–R30 carry standing, which is the
  working load. The worst moment on the way is the arms complete with the
  hands on the ground, 5.0 kN.
- **It ends as it began.** Standing again it leans 0.0°, and no member
  differs from the figure as the plan builds it by more than 0.4 kN.
- **The feet and the guy line anchors have to go back where they were.**
  The feet are let go once the hook carries the figure, and slide as it
  comes down. Putting together, they are drawn back to the places they
  stood on before taking apart, and the guy lines go back on the same
  anchors. Fixed instead wherever they touch down, the feet end up wider
  and the standing figure has 6.0 kN in its thigh cables and leans 1.6°.
  So: mark the feet and the anchors before taking it down, or keep the
  mounts in place.
- **With the net left whole the half-built figure is tighter.** Lying
  without arms the thigh cables are at 4.5 kN: the arms' cables are
  already there and pull. The figure only tightens with its feet struts
  (L53–L54, R53–R54), which take the worst cable from 2.0 to 4.5 kN.
  Those two are the ones to fit short and extend.
- **On the ground, up to the knees, almost nothing is under load.** With
  26 struts in no cable passes 1 kN. The pelvis alone is not a
  tensegrity: its six struts are a loose bundle in the net until the
  legs' and shoulders' first bricks are in.
- **Let down for the arms it rests on them, and its feet come off the
  ground**, up to a metre. Lying without arms it rests on L3, R3, Z2,
  L29 and R29, and on the slack net of the arms.
- **It has to be let go upright.** The guy lines hold only the hips (L14,
  R14, 3.6 m up); above them the figure balances on its feet. With the
  hook straight above the eyes it can lean 6° before the legs come taut,
  and let go like that it folds forward (37°, cables over 11 kN). So the
  hook goes on the line from the feet through the eyes, 12 m up and 1.2 m
  behind the feet: there the legs are shortest when the figure is upright
  and hold it both ways. Guy lines go on while it hangs like that, drawn
  in over 10 s; then the hook comes off.

## Open Claw the same way

The Take apart button works on Open Claw too, on the same machinery
(`Assembly::of(fabric, FabricName::OpenClaw)`). Open Claw is built hanging,
so its sequence is the arms' part of Headless Hug's and nothing else:

```bash
cargo test --release --lib assembly::open_claw_apart_and_together -- --ignored --nocapture
```

- **One eye, on the top: Z2**, the upper end of the strut on the axis, on a
  4 m sling leg. The feet are A30, B30 and C30. There are no guy lines.
- **Taking apart:** hook on, feet let go, lifted until nothing touches the
  ground, then for each set of three: let down until the strut's lower end
  is 0.15 m from the ground, drawn in until it carries nothing, taken out.
  The feet struts A29–A30, B29–B30, C29–C30 come out first and Z1–Z2 last,
  16 steps for 46 struts. Then the net is lifted clear.
- **Putting together starts from the net hanging with no struts.** As it
  is let down, a line to each foot from a stake 4 m beyond where that foot
  stood is reeled in, at up to 1.5 m/s while it pulls less than about
  3.8 kN. The legs come apart in the air: 4 s in, the feet are 5.4 m from
  the middle and 1.5 m up, and hardly anything touches the ground for
  12 s. Let down, the feet lie 6.3 m from the middle, wider than the 3.7 m
  they stood at, and the legs lie out straight along the lines. Nothing
  pulls during the take-down.
- **With the net on the ground the lines come off.** Z1–Z2 goes in, and
  the hook is raised step by step as each three go in, down to the feet,
  drawing the legs in as their struts go back. Then it is stood on the
  ground, the feet drawn to where they stood, the hook taken off.
- **A strut goes in only with both its ends off the ground.** Before each
  three the hook raises the net until they are clear, and their caps are
  carried, over 3 s, to where the pose they came out in has them, fitted
  to the part already in. Drawn apart, the legs are not where they were
  when the struts came out, and the part already in hangs shifted and
  turned, so the old pose is moved and turned about the upright to lie
  best on it and the eye. While only Z1–Z2 is in, its caps are on the
  axis and show no turn, so none is taken.
- **Why the caps are carried.** Left where they hang, the first ring of
  three after Z1–Z2 can twist the figure either way: once it was built as
  Open Claw's mirror image, which fits the standing figure mirrored to
  0.02 m, and tangled when its feet were drawn to their mounts (435 kN).
  Moved there in one tick the caps jumped up to 1.6 m and the fabric blew
  up. Carried over 3 s the cables they hang on stretch on the way: at two
  steps the worst reaches 46–50 kN, against 37 kN standing, in simulation
  forces. Headless Hug's net, laid on the ground, still goes all the way
  back to its pose.
- **The order is by joint number**, which follows the build outwards from
  the top, so the same order holds if the shape changes. Strut heights
  would give nearly the same order.
- **It ends where it began:** Z2 within 1 mm and the feet on their marks,
  no member more than 0.2 kN from the figure as the plan builds it.

The forces the test prints for Open Claw are not to be read as loads. Its
plan has no hardware masses and its stiffness is the simulation's default,
so it stands at 37 kN in its tightest cable where the real one carried far
less; the working load and buckling margins in the listing are Headless
Hug's tubes and cables. Only the comparisons hold: the forces fall as
struts come out and return to what they were.

## Next: the movie

Not built yet:

1. **The movie** by the existing route: `--record <seconds>` writes frames
   in fabric time (`src/animation_export.rs`),
   `scripts/tensegrity_fast_import.py` brings them into Blender,
   `scripts/render.sh` renders. That is how the Open Claw construction movie
   was made (December 2025, re-rendered April 2026). The export skips
   `Support` intervals; the sling legs need their own look, and members
   that appear part-way need checking in the importer.
2. **The crane travelling.** The hook follows the figure sideways as the
   feet are drawn to their mounts; a real crane would carry it there.

Open questions for Gerald:

- How are the first struts held before the pelvis and first bricks make a
  shape: on trestles, or by hand?
