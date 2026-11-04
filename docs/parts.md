# Headless Hug Parts: Designs, Sourcing, Ordering

What it takes to buy and make Headless Hug's hardware: each part's design as
it stands in the code, where off-the-shelf parts might come from, and how the
custom steel could be ordered. Written 2026-11-02 from the current plan.

Nothing here is a final order. Dimensions in the code marked ESTIMATE
(`src/connector/fork.rs`) have not been checked against real parts, and
**none of this has been checked by the engineer.** Supplier facts come from
their public pages and are listed under Sources; confirm them before ordering.

To look at the parts, press P in the running application: click one in the strip
on the left to show it large, drag to orbit, wheel to zoom, P or Esc to return.
Assembled parts show whole.

Regenerate the counts with:

```bash
cargo test --release --lib headless_hug_parts_list -- --nocapture
```

Write the laser-cut drawings, one DXF per part with thickness and quantity in
its name, to `HeadlessHug-parts-<date>/`:

```bash
cargo test --release --lib export_headless_hug_dxf -- --ignored
```

## Bill of materials

The app's parts list (`connector::parts`, shown in the C view) covers the
connector hardware. The second table adds what it doesn't track yet.

| Qty | Part | Kind |
|---:|---|---|
| 464 | Ring with boss and cross-tube | Custom, welded |
| 4 | Custom disc with cross-tubes (four different) | Custom, welded |
| 20 | Node plate | Custom, laser cut |
| 476 | POM washer | Custom or bought |
| 492 | Jaw terminal, on the cables | Bought, swaged on |
| 48 | Rod fork, two per fork rod | Custom, cut and drilled, see below |
| 8 | Hinge: M12 rod end, custom fork, bolt | See below |
| 24 | Outer tube 40×3 × 1350 mm, with 48 inner tubes 33×3 × 850 mm | Aluminium, cut; PROPOSED long set |
| 40 | Outer tube 40×3 × 900 mm, with 80 inner tubes 33×3 × 700 mm | Aluminium, cut; PROPOSED short set |

| Qty | Not on the app's list | Note |
|---:|---|---|
| 128 | M12 bolt or stud at a strut end | Length follows the stack, so several lengths |
| 112 | M12 nut | The other 16 strut ends are held by the hinge fitting |
| 244 | Cable, 6 mm, cut to length | Lengths come from the CSV |
| 24 | M8 threaded rod, cut to length, with 4 nuts | Cut lengths come from the CSV's header; see below |
| 4 | Guy line and ground anchor | Ground fixing not designed |

The cross-tube has no line of its own: it is welded to its ring or disc.

## The designs

### Ring with boss and cross-tube (464)

A flat ring turning on the strut's M12 bolt, with a boss ending in a welded
cross-tube. The cable's terminal pins through the tube.

- Ring: 5 mm plate, Ø40 mm, Ø13 mm bore.
- Boss: 12 mm wide, flat end 22 mm from the bolt axis, seating the tube 1 mm
  into its wall (`BOSS_SHORT_OF_PIVOT`, ESTIMATE). Pin axis 30 mm from the
  bolt axis.
- Tube: 18 × 2.5 mm precision tube (E235+N), 13.5 mm long, axis square to the
  bolt.
- Roughly 60 g each, about 28 kg in all (estimate).

The original drawing and the Blender script in
[connectors.md](connectors.md) show the older 32 mm version; its construction
is the same.

### Custom discs (4)

One disc per stack on a strut lying in the mirror plane, where twin cables
must share a height. Same plate, bore and tubes as the ring, with four bosses.
Each boss is given as angle/reach from the disc's centre line:

| Disc | Bosses |
|---|---|
| 1 | −149°/40, −24°/40, 24°/40, 149°/40 mm |
| 2 | −145°/30, −29°/40, 29°/40, 145°/30 mm |
| 3 | −120°/30, −33°/30, 33°/30, 120°/30 mm |
| 4 | −107°/30, −35°/30, 35°/30, 107°/30 mm |

These shift by a degree or so whenever the plan changes, so order them last.

**The 40 mm bosses are a strength question.** Past the disc's 20 mm rim, a 40 mm
reach leaves 11 mm of flat 5 × 12 mm bar before the cross-tube, and any pull along
the strut bends it the weak way. By hand, as a simple cantilever in S235, discs 1
and 2 start to yield at about 1.3 kN of cable tension. Reach is limited to 40 mm
because 50 mm was about 0.7 kN; 30 mm leaves 4 cable ends colliding. A stub of the
18 × 2.5 mm tube in place of the flat bar would resist this about 8 times better,
and it isn't modelled yet. Needs the engineer.

### Node plate (20)

One part: an equilateral triangle, 60 mm hole to hole, 12 mm thick, corners
R16 mm, so 92 × 84 mm overall and about 0.5 kg each. The plate hangs on a
fork rod (16 plates), or splits a cable that would otherwise end on a hinge
(4 plates: Z5b, Z6b, L10b, R10b). Terminals straddle it in their 14 mm jaw gap.

Holes are Ø10.5 mm for the terminal's 9.7 mm pin (`PLATE_HOLE_DIAMETER`,
ESTIMATE). A hole under Ø12 in 12 mm plate may need drilling or reaming rather than
laser cutting; ask the cutter.

### POM washers (476)

Ø40 × Ø13 × 2 mm in the model: one under every ring and disc, and 8 more stacked
where a ring has to sit higher to clear the forks below it. I found no stock
part. Open Claw's were a custom run: 500 for €185 from Snijlab, from a drawing
named "Pom ring - 36 x 36 x 2", so Ø36 × 2 mm. Ask Snijlab for Ø40.

### Rod fork (48), making 24 fork rods

Every tension connection is a cable or a fork rod: threaded rod cut to length
between two rod forks. The rod is stock like cable, so only the fork is on the
parts list. The rod is drawn as a cable, and the CSV lists the lengths to cut.

16 fork rods are hangers, 82 mm pin to pin, each hanging a node plate from a
strut end's cross-tube. The other 8 stand in for plate cables too short to make.

**The fork** is a U with a hole through its closed end, the web. None of these
figures has been checked by the engineer.

| From the pin's centre | |
|---:|---|
| 17.5 mm | Rod's end, just clear of a plate's R16 corner |
| 26 mm | Inside of the web, after a 6.5 mm load nut |
| 34 mm | Outside of the web, 8 mm thick |
| 40.5 mm | End of the 6.5 mm lock nut |

- 14 mm jaw gap, 3.8 mm jaws, 20 mm wide, Ø9 mm hole for M8 rod, the terminal's
  9.7 mm pin and split ring.
- In tension the web pulls against the **load nut**, inside the U, so it and the
  web carry everything. The **lock nut** outside only keeps it from working loose.
- It could be one laser-cut U from about 20 mm plate, drilled for the rod and
  pin, with no welding. It isn't in the DXF export yet.
- An M8 nut fits the gap but can't turn in it, so the fork's angle is set by
  turning the fork on the thread: at most about 0.6 mm of length.

**Rod to cut:** pin to pin − 35 mm. The shortest is 81 mm pin to pin
(`FORK_ROD_MIN_LENGTH`), with the lock nuts touching.

| Pin to pin | Rod | Qty |
|---:|---:|---:|
| 82 mm (hangers) | 47 mm | 16 |
| 152 mm | 117 mm | 2 |
| 179 mm | 144 mm | 2 |
| 187 mm | 152 mm | 2 |
| 198 mm | 163 mm | 2 |

Those lengths are from the settled shape, as the app's connector view commits
them. The CSV is written at the slack moment and differs: 12 of the longer rods
rather than 8. Which one rods are cut from is the
same open question as for the cables (below).

**A rigid rod replaces what was a swivel.** The swivel was there because the two
pins swing as the figure moves. With the animation at its resonance (see "Motion"
below) a hanger's pins swing up to 4.6° against each other. The rods from a plate weren't
measured at that frequency. What takes that up is play: a 12 mm plate in a
14 mm gap, the pins in their holes, and the ring turning on its bolt. By hand
that's roughly the same size as the swing, so it needs trying on a real part, at
the animation's 1 Hz. A jaw gap over about 15.5 mm would let the nut turn in the
U, so the fork could turn on the rod, with spacers on the pin.

**M10** is an option if M8 isn't enough. Its nut needs a gap of about 17.5 mm, and
the shortest would be about 88 mm, or 82 with a thin lock nut.

**One nut per fork, with a separator.** For a shorter join, drop the lock nuts:
two forks back to back on a short rod, a load nut inside each U, the webs
clamped against a sleeve between them.

- Pin to pin is 68 mm plus the separator, and the rod is still pin to pin − 35 mm.
- It's a clamped joint, and clamping is what holds the nuts. That also ties the
  forks' relative angle to how tight they are, unless the nut can turn in a wider
  gap, in which case it can be tightened with a spanner from the U's open side
  before the pin goes in.
- Trying hanger lengths in the model (forks as above): 76, 82, 90, 100 and 110 mm
  build cleanly; 68 and 72 mm leave 2 cable ends colliding. So 76 mm, an 8 mm
  separator and a 41 mm rod, is the shortest that works today.
- At 82 mm it's a 14 mm sleeve and two nuts instead of four nuts. Nothing in the
  fabric needs under 82 mm yet, so this is a choice of hardware for now.

### Hinge (8)

At L6/R6, L8/R8, L10/R10, Z5 and Z6. Two fittings screw onto the two struts'
bolts above their stacks, acting as the stacks' nuts, joined by a pin exactly
where the strut axes meet, square to both. Struts spin freely on their own
axes, so the hinge only opens and closes. No cable ends on a hinge. The model
uses a clevis on one strut and a rod end on the other, 60 mm from pin to
fitting base, 16 mm jaw gap, 12 mm pin.

The intended design is the M12 rod end used on Open Claw's bottom connectors,
with a supported bolt through its ball on a custom fork; see below. The model
doesn't have the real part's dimensions yet.

## Mass

The simulation counts Headless Hug's mass from these parts (`Hardware::OPEN_CLAW`
in `src/fabric/dimensions.rs`), not a flat mass per joint: about 334 kg in all at scale 0.75.
The parts view (P) shows each listed part's mass, worked out from its dimensions
(`Part::mass`), and the total for the list.

- **Struts:** Open Claw's telescoping tubes, one outer 40×3 mm and two inner
  33×3 mm, EN AW-6060 T66 (247TailorSteel order NL26140673). Open Claw's tubes
  over its mean strut length come to 1.24 kg per metre, kept for every length.
- **Each strut end:** 254 g. The steel cap (Ø36 × 5 mm tapped plate on a
  40×3 × 40 mm tube, about 145 g), the M12×55 bolt, nyloc, DIN 9021 washer,
  spring washer and nut cap.
- **Each cable end:** 230 g. Terminal (160 g), ring with cross-tube (61 g), POM
  washer and more bolt. The same figure stands in for a rod fork.
- **Node plate:** 469 g, less what its cable ends already counted. **Hinge:**
  about 430 g.

The weighed total of Open Claw's 247TailorSteel order, 251.6 kg, matches its
tubes, caps, tabs and base plates computed this way, so a cap is about 145 g, not
the 1.2 kg once estimated by subtraction.

## Findings that need a decision

These came out of comparing the model with real catalogues.

### 1. Hinge: Open Claw's rod end, and a fork to design

Open Claw's order list (`ENS/Bestellijst Open Claw v20260527.xlsx`) has it as
"Stangkop M12", 4 for €15.06 if that column is the line total, as it appears
to be. The listing's name was *stangkop SIK12C / GIKR12PW / SIKB12F / PKS12ES,
gat 12 mm × M12 inwendig schroefdraad*, from NexaParts. **That page now
redirects away**, so it needs another source.

It screws directly onto a strut's M12 bolt, and the supported bolt goes through
its 12 mm bore. Figures for the INA part with those names, GIKR12-PW-A (also
sold as SKF SIKB12F):

| | |
|---|---|
| Bore, thread | 12 mm, M12 internal, right hand |
| Ball ring width | 16 mm |
| Housing width | 12 mm (from its PB sibling, same series) |
| Head diameter | 34 mm listed; unconfirmed |
| Load rating | 20.8 kN static, 24 kN dynamic |
| Sliding surface | Steel on PTFE bronze film, no maintenance |
| Standard | DIN ISO 12240-4, series K, type F |
| Weight | 119 g |

To settle before ordering:

- **What was actually bought.** At about €3.77 each it may be an unbranded
  equivalent, so INA's ratings may not apply. The name also lists SIK12C, which
  from general knowledge is a different sliding surface. Check any marking on
  the head.
- **Measurements.** Overall length is listed as both 66 and 70 mm, and I found
  no tilt angle for this part (13° is listed for GAKSR12-PS). Measure a real
  one: overall length, centre to end, head diameter, thread depth, tilt.
- **The fork's gap.** With the ball ring clamped in a 16 mm gap, the housing
  hits the jaws at only about 7° of tilt. About 2 mm of shoulder or spacer each
  side, a 20 mm gap, gives about 13°. The animation swings Z6's pin axis by up
  to 7.6°, so don't rely on the 16 mm gap unless the struts' spin takes that up.
- **The fork itself.** No stock clevis fits: an M12 DIN 71752 clevis has a
  12 mm gap, and one with 16 mm has an M16 thread. So it's custom, laser-cut
  plates welded to a threaded boss, sized by the engineer. I found no load
  ratings for DIN 71752 clevises anyway.
- **Load direction.** A rod end is rated for radial load, across its shank. How
  much the hinges see along the pin belongs with Peter's question about
  sideways forces.

### 2. The forces are indicative, not design loads

Headless Hug is modelled at scale 0.75: 8.4 m tall, struts 1.33–2.40 m, all within
the length range Open Claw's struts have proven (1.83–2.42 m) or shorter. At 0.8 it
was 9.1 m, with eight struts longer than any of Open Claw's, up to 2.54 m.

The simulation uses the real parts' stiffness (`Hardware::OPEN_CLAW`): 6 mm 7×19
cable at EA about 1.4 MN (an estimate until a datasheet gives it) and the
telescoping 40×3 / 33×3 tube at about 21 MN, without the locking pins' play. It is
pretensioned by lengthening every strut 0.5%. At scale 0.75 that keeps all
connectors clear and, settled under gravity, gives struts a median of 2.9 kN (max
5.4) and cables at most 4.5 kN, with 2 slack. Less collides (0.2–0.4%); more
raises the forces by half (0.6%) and tips the stance.

Through the animation at its resonance no cable passes 6.3 kN, a quarter of a 6 mm
cable's 25.5 kN breaking load. The most loaded at their peak:

| Cable | Settled | Peak |
|---|---:|---:|
| L45–L30, R45–R30 | 4.5, 4.1 kN | 6.2, 6.3 kN |
| L43–L32, R43–R32 | 3.6, 3.8 kN | 5.9, 5.9 kN |
| L5–L5b, R5–R5b (hangers, M8 rod) | 4.0, 3.9 kN | 5.7, 5.6 kN |
| L32–L44, R32–R44 | 3.1, 3.1 kN | 5.1, 5.1 kN |

Wind isn't modelled; Open Claw's engineer had cables reaching 4.7 kN under wind
alone. 8 mm cable for any of these is the engineer's call with real load cases.

### 3. Standing upright, and motion at the animation's resonance

The guy lines are set so the centre of gravity stands over the feet: anchors 5.2 m
in front of and behind the pelvis, the front pair at 90.5% and the rear at 109% of
their distance on landing puts it 13 mm in front of them, with both pairs lightly
taut (about 0.5 kN each). It is very sensitive: 1% on either pair moves it 10–40
cm, so real guy lines need fine adjustment. Tightening both pairs doesn't stand it
up; they pull the pelvis down instead.

Standing upright it sways freely. The animation peaks at about 0.24 Hz, a period of
4.2 s, with the guy lines driven at 0.75%: 237 mm mean joint motion, up to 493 mm.
Damping is the model's own, so the real height will differ, and the real
frequency needs finding by experiment. Measured in steady motion at 0.24 Hz:

| | At 0.24 Hz |
|---|---:|
| Cable leaving its boss's plane, disc 1 (Z1) | 1.3° |
| Disc 2 (Z4) | 0.8° |
| Disc 3 (Z2) | 1.4° |
| Disc 4 (Z3) | **2.3°** |
| Hanger pins swinging, worst (L5, R5) | 4.6° |
| Hinge pin axis swinging, worst (Z5) | **11.7°** |
| Hinge opening and closing, worst | 3.2° |
| Cables going slack each cycle, of 272 | 6 |

By hand the play between a jaw and a cross-tube is about 1.6°, so disc 4's bosses
to L15b and R15b are the ones most likely to bind. A hinge's pin axis can only
swing that far if both struts turn on their bolts, by up to 12° each cycle, so
those bolts and stacks need to turn freely under load.

### 4. Things in the model to check

- **Pin play.** The cross-tube's bore is 13 mm, but the terminal's pin in
  `fork.rs` is 9.7 mm (datasheet), while the note at the top of
  [connectors.md](connectors.md) says a 12 mm pin. That's 1 or 3.3 mm of play;
  settle which pin it is.
- **Boss flat.** With the pin axis at 30 mm and an 18 × 2.5 mm tube, the bore
  starts 23.5 mm from the bolt axis. The flat is now at 22 mm, a 1 mm seat
  that stays clear of the bore. Have the welder confirm that seat.
- **Strut end.** Without caps, what the bolt anchors into isn't designed. The
  stack starts right at the tube end.
- **Slack versus settled lengths.** Hardware lengths in the CSV are for the
  slack shape and differ from the settled figure (see
  [csv-handoff.md](csv-handoff.md)). Decide which one cuts are made from.

## Off-the-shelf sourcing

| Part | Where to start | Status |
|---|---|---|
| Jaw terminal, 6 mm AISI 316 | Louis Ruys B.V. (Zwijndrecht) 37260006, the part in the code. Open Claw paid €10.74 each (360 pcs, 40% off €17.90 list) | Dimensions in code are from its datasheet |
| Cables with terminals | Louis Ruys again: Open Claw used 36001106, galvanized 6 mm 7×19, MBL 25.5 kN. Or Mennens (NL) for AISI 316 | Quote needed for 244 made-up cables |
| M12 rod end, internal thread | INA GIKR12-PW-A or SKF SIKB12F, e.g. from ABF (Dutch number, price on request); NexaParts supplied Open Claw's | Details above |
| Precision tube 18 × 2.5 | Any steel stockist, or cut by the laser service (below) | |
| M12 bolts, nuts | Schroevengroothandel, as for Open Claw: M12x55 DIN 933, DIN 985 nyloc, thermally galvanized | Lengths to be worked out per stack |

## Ordering the custom steel

The custom steel is about 464 rings, 4 discs, 20 node plates, plus hinge plates. It's a simple flat-part job, with
one catch: a lot of welding.

### Who cuts it

| Service | What their pages say |
|---|---|
| **247TailorSteel** (Varsseveld, NL), Open Claw's supplier | DXF, DWG and STEP upload, quote in about a minute. S235 and S355 up to 25 mm. Minimum part 15 × 15 mm. Delivery from 48 hours. Tube lasers too. Engraving available. **Business customers only.** |
| **Xometry Europe** | DXF upload, instant quote, a network of partner workshops. S235 listed in 5 and 12 mm among others. Finishing "on request". Limits differ between its own pages, so check in the quoting tool. |

A local laser shop is a third route, and may do cutting, welding and arranging
galvanizing as one job, which suits this work well.

### Steps

1. **Settle the dimensions.** Finish the findings above first; the bore, boss
   and hole sizes all go into the files.
2. **Make the files** with the export above: true size in millimetres, lines,
   arcs and circles only. It covers rings, discs and node plates; the cross-tubes
   are welded on afterwards. Custom hinge plates aren't drawn yet.
3. **Choose the steel with the galvanizer.** S235 is the natural choice, with
   S355 if the engineer wants it. Ask the galvanizer which silicon content they
   want, since it affects the coating, and specify that when ordering.
4. **Get quotes** for the full quantities: 464 rings in 5 mm, the 4 discs in
   5 mm, and 20 plates in 12 mm. Ask about holes under plate thickness, tolerances
   (not published on the pages I read), and material certificates.
5. **Prototype first.** Order about ten rings, one disc and two plates, then
   weld, galvanize and fit them to a real terminal and rod fork before the run.
   This also tests the tube weld and the pin fit.
6. **Weld the tubes.** 480 tube welds (464 + 16) in a jig is likely the largest
   single cost, and it's what the [single-piece formed
   eye](connectors.md) alternative avoids. Get a quote for both. Open Claw's
   welding went to Laswerk Sil.
7. **Galvanize.** Coatinc spin-galvanized Open Claw's parts. It has several
   Dutch plants and does small parts by centrifuge, which leaves a thinner coat. Rings have open-ended tubes, so
   venting shouldn't be a problem, but ask. Zinc builds up in bores and holes,
   so check the fit on the prototypes.
8. **Engrave.** Engraving names at the cutter is cheapest for the custom discs
   and plates, if per-part names are wanted. A ring's place is only decided
   when the hardware is committed, so rings stay generic.

### Certification

247TailorSteel states it is not EN 1090-1 certified itself, but has external
assessments against EN 1090-2 and -3 that customers can request. For a
structure touring in public, ask the engineer what paperwork the welded parts
need before choosing a workshop.

## Sources

- Open Claw vault, ENS folder: order list, Louis Ruys quote, "Gaffelstang" drawing
- [ABF: INA GIKR12-PW-A rod end](https://www.abf.store/s/en/bearings/GIKR12-PW-A-INA/484136)
- [Schaeffler GAKSR12-PS rod end](https://medias-at.schaeffler.com/en/product/rotary/waelz--und-gleitlager/gleitlager/gelenkkoepfe/gaksr12-ps/p/346082)
- [INA GAKR12-PB-A rod end](https://www.acorn-ind.co.uk/p/ina/rod-ends-with-male-thread/gakr12-pb-a-ina/)
- [Norelem: stainless DIN 71752 clevis](https://norelem.at/en/Product-overview/Systems-and-components-for-machine-and-plant-construction/27000/Clevis-joints/Clevis-joints-stainless-steel-DIN-71752/Clevis-Joint-stainless-steel-bright/p/27622-1224)
- [Elesa+Ganter: DIN 71752 datasheet](https://elesa-ganter.be/technicaldatasheet/DIN%2071752.pdf/nl)
- [JW Winco: stainless DIN 71752](https://www.jwwinco.com/en-us/products/product-family/Stainless-Steel/DIN-71752-Metric-Size-Stainless-Steel-Clevis-Fork-Heads)
- [Mennens: AISI 316 cable](https://www.mennens.nl/producten/staalkabel-touw/rvs-staalkabel/7x7-6x7-wsc-rvs-p4220587)
- [Accu: POM washer 13 × 22 × 2](https://accu-components.com/pl/podkladki-metryczno-plaskie/505164-HPW-13-22-2-POM)
- [Big Dutchman: POM washer 40 × 19.5 × 3](https://shop.bigdutchman.com/bd350/83-17-2466)
- [247TailorSteel: thick plate](https://247tailorsteel.com/en/types-of-metal/custom-made-steel),
  [laser cutting](https://247tailorsteel.com/en/metalworking/laser-cutting),
  [FAQ](https://247tailorsteel.com/en/service/frequently-asked-questions)
- [Xometry Europe: laser cutting](https://xometry.eu/en/laser-cutting/)
- [Coatinc: centrifugal galvanizing](https://coatinc.com/?p=1784)
- [Galvanizers Association: spin galvanizing](https://galvanizing.org.uk/?p=32428)
- [Galvanizers Association: design detailing](https://galvanizing.org.uk/design-detailing/)
