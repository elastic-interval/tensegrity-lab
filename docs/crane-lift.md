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
In the app (Headless Hug, once it stands) that is the **Take apart**
button, then **Put together**; Fast-forward hurries either. The tests in
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
- **A strut is drawn in, then taken out.** It shortens, by a tenth of its
  length at most, until it carries nothing. It goes back in at that
  length, between the same two caps, and is lengthened to its own. A strut
  that still carries when drawn in (Z3–Z4 in the pelvis) goes back as it
  was.
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
- **Arm struts are handled within reach of the ground.** The hook lets
  the figure down until the lower end of the next arm strut is 15 cm up,
  or the legs go slack.
- **A strut goes back in where it came out.** Lengthening struts in the
  net as it happens to lie does not work in the simulation: nothing there
  says which side of a cable a strut goes, and it tangles (cables at
  hundreds of kN). So each stage of putting together starts from where
  taking apart had the net when those struts came out. On site that is
  the numbers written on the caps.

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

It starts from that hanging net, which is laid out by four lines from
stakes in the ground, as it is done on site:

- **The legs are hauled out first.** A line runs to each foot (L54, R54)
  from a stake 8 m beyond where that foot lay, on the line out from the
  middle of the net. Both are hauled in steadily over the first 12 s,
  until 8 m of each is left, so the net comes down with its legs already
  apart, not dragged apart once it is down.
- **The hands are only kept taut.** A line runs to the tip of each hand
  (L62, R62) from a stake 3 m beyond where it lay, and is reeled in at up
  to 0.5 m/s only while it is barely taut, until 3 m is left.
- **The hook lets the net down** over 30 s meanwhile.

It comes down with the feet within 0.2 m and the hand tips within 0.3 m of
where they lay, the rest of the net 0.85 m off on average, 1.6 m at most.

So the hand tips want an eye each, as L25 and R25 have: **eyes on L62 and
R62** for the laying-out lines. The feet have their rod ends.

Then the lines come off and the net is eased the rest of the way, over 5 s,
to where it lay when the last strut came out. That last easing is not
physics: every joint is moved in a straight line, standing in for people
straightening the net. Then the struts go in, the same stages backwards
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

## Next: the movie

Not built yet:

1. **The movie** by the existing route: `--record <seconds>` writes frames
   in fabric time (`src/animation_export.rs`),
   `scripts/tensegrity_fast_import.py` brings them into Blender,
   `scripts/render.sh` renders. That is how the Open Claw construction movie
   was made (December 2025, re-rendered April 2026). The export skips
   `Support` intervals; the sling legs need their own look, and members
   that appear part-way need checking in the importer.
2. **Cables that drape.** The net is kept whole now, but a slack cable is
   still drawn as a straight line between its ends. Open Claw's
   disassembly (branch `open-claw-after`, `src/build/packer.rs`) let the
   loose ones hang as chains, which would look truer.
3. **The crane travelling.** The hook follows the figure sideways as the
   feet are drawn to their mounts; a real crane would carry it there.

Open questions for Gerald:

- How are the first struts held before the pelvis and first bricks make a
  shape: on trestles, or by hand?
