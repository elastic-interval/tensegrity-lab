# Fabric JSON (Headless Hug)

Headless Hug as built, for the engineer and anything else that reads it. It
replaces the engineering CSVs (Open Claw's is archived on an old branch).
Written by `src/fabric_json.rs`, it says what is made and where, never how to
analyse it. The RFEM model is made from it by `scripts/rfem/build_rfem.py`,
which is where every structural modelling choice is made and explained. Both
go out in the handoff package:

```bash
cargo test --release --lib export_headless_hug_package -- --ignored
```

which writes `HeadlessHug-<date>/` and `HeadlessHug-<date>.zip`: the JSON,
`build_rfem.py`, the DXFs and the welding document, all from one build. The
JSON alone:

```bash
cargo test --release --lib export_headless_hug_json -- --ignored
```

Nothing in the JSON names RFEM or any other package. If a reader needs
something it doesn't have, add the physical fact here, not the reader's
idea of it.

## Moment and frame

The figure standing: pretensioned, settled under gravity on its feet and guy
lines, with the connectors committed on it (the moment the app's connector
view, the DXFs and the welding document use). Metres, kilograms, newtons; Z
up, the ground on Z = 0. Sim (x, y, z) becomes (x, −z, y). Positions are to
the micrometre.

Every member has its **length** here and its **rest length**, unloaded: what
it is made to. Their difference is the pretension, and the member's
**force** (newtons, tension positive) is that difference at the real parts'
stiffness. Tubes, cables and rods are made to rest lengths.

The slack shape the CSVs used is gone: its lengths were rest lengths before
the struts were lengthened 0.5 % to pretension, so the pin settings worked out
there were 7 to 12 mm short.

## Versioning

`schema_version` goes up whenever a field changes meaning or goes away;
adding a field doesn't change it. `build_rfem.py` refuses a version it
wasn't written for. Version 2 is the standing figure; 1 was the slack shape.

## Contents

| Key | What |
|---|---|
| `total_mass` | everything the simulation weighs |
| `materials` | name, description, `youngs_modulus` (Pa), `density` (kg/m³) |
| `tubes` | the strut tubes: `outer` 40×3 and `inner` 33×3, `outside_diameter`, `wall`, material |
| `tension_stock` | `cable` (6 mm 7×19) and `rod` (M8): material, `area` (m²), `mass_per_metre`, `breaking_load` (N) if known |
| `joints` | every simulation joint by name; reference only, as the hardware moves the real ends off them |
| `struts` | see below |
| `tension` | every cable and fork rod, see below |
| `plates` | node plates: `holes` (three positions), `normal`, `thickness`, `mass` of the plate alone |
| `hinges` | `pin` joint, `position`, `pin_axis`, `clevis_strut`, `eye_strut`, `mass` |
| `feet` | `joint`, `strut`, `position` (the rod end's ball), `pin_axis`, `part` |
| `guy_lines` | `strut` and `joint` it leaves, `from` (its stack), `anchor` and `anchor_position` on the ground, `length`, `rest_length`, `force` |

### Struts

`name` is `<alpha>-<omega>`. `length` and `rest_length` run stack to stack;
`force` is negative, a push; `tube_mass` is its three tubes. Each of its two
`ends` has

- `joint`, the strut end's name;
- `stack`, where the tubes end and the stack of rings starts;
- `axis`, outward along the strut, the way the stack builds;
- `hardware_mass`, its cap, bolt, nut and washers;
- `closed_by`: `washer and lock nut`, `hinge clevis`, `hinge eye` or
  `foot rod end`. For a fitting, the stack is set back from the joint, which
  is the fitting's pin.

`tube_set` gives the outer and inner tube lengths, the `pins` at each end as
the crew counts them (`12:3-6`, see `docs/parts.md`), and `length_error`,
how far the set tubes miss `rest_length`. `segments` are the three tubes in
place, inner / outer / inner, from the first end.

### Tension

`kind` is `cable`, `hanger` (a fork rod hanging a plate) or `fork rod` (a
plate cable too short to make). `stock` names its `tension_stock`.
`pin_to_pin` and `rest_pin_to_pin` are its lengths standing and unloaded;
`force` is nil if it hangs slack. `rod_cut_length` is the threaded rod to cut
for a rod.

Each of its two `ends` is `on` either

- `stack`: on the `strut` at `joint`, in `slot` (1 nearest the tubes),
  raised by `extra_washers`, the ring's `ring_center` on the strut axis and
  the cross-tube's `pivot` pin; `hardware_mass` is the terminal or fork, ring
  and washer;
- `plate`: in `hole` (1–3) of `plate`, at `position`; `hardware_mass` is the
  terminal or fork.

`name` is its two ends, `<pin>@<far joint>` for an end on one of a hinge's
two stacks (`L10@Z6-L10b`).

## From Open Claw's CSV

Open Claw's CSV gave the connectors as extra rows, which the engineer built
into RFEM by hand. Each is in the JSON, recorded per cable end instead:

| CSV rows | In the JSON |
|---|---|
| `pull`, pivot to pivot, with slot and elevation | `tension`, its `ends`' `pivot` and `slot`; elevation follows from `pivot`, `ring_center` and the strut end's `axis` |
| `axial`, joint → ring centre → next ring centre | the strut end's `stack`, then each end's `ring_center` there in `slot` order |
| `radial`, ring centre → pivot | each end's `ring_center` → `pivot` |
| `push-fea`, `pull-fea` (to the highest ring centre) | the highest `slot`'s `ring_center` at each strut end |

The CSV had no pretension: the engineer applied it by hand. The JSON's rest
lengths and forces carry it.

## The RFEM script

`scripts/rfem/build_rfem.py <json> --dry-run` makes the model in plain Python
and prints its counts, its weight beside the simulation's and the forces it
starts from. Without `--dry-run` it builds it in the running RFEM 6 through
Dlubal's `dlubal.api`. It opens with what it does and how, in prose, then its
settings: `PRESTRESS` (1 is the simulation's pretension, 0 none) and the
library names. The pretension goes in as an imposed strain on each member,
`rest length / length − 1`, the weight from the JSON with RFEM's self-weight
off, in one load case analysed with large deformations.

The calls into `dlubal.api` are a first guess, marked `# CHECK`, not yet run
against RFEM.

Open questions:

- Which RFEM version and Python client the engineer runs.
- The library names in `RFEM_NAMES`, and how to give the cable and rod their
  areas.
- The ring turns freely on its bolt and the struts spin; rigid ring links hold
  torsion the real parts don't.
- The inner tubes' overlap inside the outer, and the pins' play.
- Whether the engineer wants to build the pretension up himself, as a check:
  `PRESTRESS` can start at 0.
