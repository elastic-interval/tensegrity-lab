# CSV Handoff to Engineer

How the CSV export is structured and what the engineer's workflow with it
looks like. Source of truth for the format and emission is
`src/open_claw_symmetry.rs` — the entire CSV mechanism lives there, alongside
the threefold-symmetry enforcement that produces a manufacturable file. The
slack moment itself is the end of the Building stage in
`src/build/dsl/fabric_plan_executor.rs`.

## One CSV: `slack`

The simulation exports exactly one CSV, captured right after slackening (end
of Building, before zero-G pretensing begins). The export is driven by the
test `test_open_claw_threefold_symmetry`:

```
cargo test --release --lib test_open_claw_threefold_symmetry
```

It writes `OpenClaw-<date>.csv` (e.g. `OpenClaw-2026-05-17.csv`) into the
working directory and asserts that every rotational triple has identical
length and slot assignment at each cable end — i.e. the engineer's CSV is
guaranteed symmetric before it ships.

Physical state at this moment: pulls have been given extra rest length
(slack); pushes have been snapped to discrete lengths. Joint positions are
end-of-build geometry. No pretension yet.

Earlier iterations of the codebase also emitted `pretenst`, `settled`, and
`grav_pretenst` snapshots, but the engineer's FEA workflow never consumed
them, so they were removed.

## The connector

The connector (full geometric spec: [connectors.md](connectors.md)) is a flat
steel ring turning on an axial bolt at the strut end, with a radial boss
ending in a cross-tube at `pivot_radius` (R_tube = 32 mm) from the strut
axis. The cable's fork terminal is pinned through the tube and **pivots
freely**; the ring turns freely on the bolt. Azimuth and elevation are
therefore both free, and **every connector is geometrically identical** —
there is no bend-angle inventory and no per-position parts. The angle
columns in the CSV are informational (the elevation each fork will naturally
take), not manufacturing instructions.

## What the engineer uses, and how

- Only the **geometry** is imported from the CSV — joint coordinates and
  element connectivity. Strains and forces from the simulation are not
  consumed.
- The engineer applies the pretension themselves in their FEA tool, then
  layers on the operational loads (gravity, wind, etc.) once the pretensioned
  structure is stable.

Implications for our exports:

- The slack joint coordinates are the end-of-build geometry. At this
  moment every non-Support interval has been frozen at its current
  geometric length (zero strain everywhere); the subsequent pretensing
  grows each push's rest length by a small percentage to build tension.
  Because no part-length snapping happens, the push's rest length in the
  CSV matches the Euclidean distance between its two joint coordinates.

## Column meanings — read carefully before sending anything to the factory

The data row format is:

```
Index, Role, Length(m), Strain,
AlphaX, AlphaY, AlphaZ, AlphaJoint, AlphaSlot, AlphaAngle,
OmegaX, OmegaY, OmegaZ, OmegaJoint, OmegaSlot, OmegaAngle
```

`AlphaJoint` and `OmegaJoint` follow the naming scheme described in
[joint-naming.md](joint-naming.md): path-shaped names like `AX4YZ1` for
joints created during construction, and short 3-letter names like `BAA` /
`TOC` for the seed joints. The seed-joint scheme is what makes the 3-fold
symmetry of cable triples visible at a glance.

These columns mean *different physical things* depending on `Role`. Mixing
them up is the most likely path to a mis-fabricated part. The conventions
are unfortunately not symmetric between push and pull rows:

### Push rows (`Role = push`)

- `Length(m)` — **rest length** of the strut tube — what the strut should be
  manufactured to, including end-cap allowances.
- `AlphaXYZ`, `OmegaXYZ` — the **joint locations** in CSV coordinates (mm,
  Z-up). The Euclidean distance between them equals `Length(m)` × 1000.
- `AlphaSlot`, `OmegaSlot` — always `0` for push rows (push intervals carry
  no connector arm).
- `AlphaAngle`, `OmegaAngle` — always `90` (axial).

### Pull rows (`Role = pull`)

- `Length(m)` — the **distance between the two pivot pins**, *not* the
  joint-to-joint distance. It is the length the cable's tensioned segment
  would have if it spanned exactly between the pin at each end's connector.
  **This is the closest thing in the CSV to "what the cable should be
  manufactured to"** (the fork terminal hardware at each end is extra).
- `AlphaXYZ`, `OmegaXYZ` — the **pivot pin position** at each end (the point
  where the cable's fork is pinned to the connector), *not* the joint
  location. Distance between them equals `Length(m)`. The joint coordinates
  can be recovered from the corresponding push row at the same
  `AlphaJoint` / `OmegaJoint`.
- `AlphaSlot`, `OmegaSlot` — `1`-indexed slot at each end's connector stack.
  The axial position of slot `k` along the strut, measured from the strut
  end, is `ring_center_offset(k - 1)` in `ConnectorDimensions`
  (= `cap + washer + t_ring/2 + (k-1) × (t_ring + washer)` — a divider
  washer sits between cap and first ring and between adjacent rings).
- `AlphaAngle`, `OmegaAngle` — the **free pivot elevation** at each end, in
  degrees (`0` = radial, positive tilts outward along the strut axis).
  Informational only: the fork pivots to this angle by itself. Useful for
  checking that no fork needs to articulate beyond its physical range.

### Strain

For both roles, `Strain` is the simulation's internal strain at the moment
the snapshot was taken. For `slack` it will be near zero across the board —
the structure has not yet been pretensioned. **The engineer's workflow
ignores this column**, computing strains in the FEA tool instead.

### What the factory needs to receive (and what gets it wrong)

Send to the factory:

- For each strut: a length, a tube diameter, and end-cap details. The length
  must come from the **push row's `Length(m)`** column, not from the
  Euclidean distance between joint coordinates.
- For each cable: a length (the pull row's `Length(m)`, pivot pin to pivot
  pin, adjusted for the fork terminal hardware at each end) and the slot
  assignments. If a single authoritative cable length is needed for
  fabrication, derive it in the FEA from the deployed equilibrium geometry
  rather than the CSV.
- For the connector: one identical part per cable end, per
  [connectors.md](connectors.md). Quantity = the `Cable ends measured` line in
  the header. `cap_thickness`, `ring_thickness`, and `pivot_radius` live in
  the header parameters block.

Common pitfalls:

- **Computing element lengths from joint-to-joint Euclidean distance.** This
  is wrong for both pushes (snap drift) and especially pulls (the AlphaXYZ /
  OmegaXYZ are *pivot pins*, not joints, on pull rows).
- **Reading the AlphaSlot column as if it were a node index.** It's a slot
  number on the connector stack at that joint.
- **Reusing factory drawings across simulation iterations.** If
  `cap_thickness` or any other connector dimension changed between runs, slot
  axial positions shifted, and the manufactured strut endcaps no longer
  match the cable attachment positions. The CSV's `Created:` timestamp is
  the version marker.

## CSV header layout

```
# OpenClaw, Phase: <moment>, Height: ...mm, Created: <timestamp>
#
# === Connector parameters (see docs/connectors.md) ===
# t_ring  ring_thickness
# t_w     washer_thickness (sluitring)
#         cap_thickness
# R_tube  pivot_radius (momentarm kabel)
#         push_radius
#         pull_radius
#
# === Afgeleide waarden ===
#    ring_center_offset(0)
#    ring stap (= t_ring + sluitring)
#
# Orientation check (CSV coords, mm, Z-up): ground plane at Z=0, apex at Z=...
# Lowest[1..3]: joint=...
# Highest:      joint=...
#
# === Pivot elevation angles ===
# Cable ends measured:  ...
# Elevation:            min / mean |a| / max
#
# === Connector arm clearance ===
# Joint-ends measured / Pairs measured / Clearance min-mean-max
#
Index,Role,Length(m),Strain,AlphaX,...,AlphaAngle,OmegaX,...,OmegaAngle
<rows>
```

The connector parameters block sits early on purpose: a downstream Grasshopper
"Overview of design parameters" panel shows roughly the first 12 lines of the
CSV.

## Coordinate system

The CSV is written in **Z-up** (Rhino/RFEM convention) while the simulation
runs in Y-up. The transform is `Mat3::from_rotation_x(π/2)` applied to every
exported position: sim (x, y, z) → csv (x, z, −y). See
`open_claw_symmetry::sim_to_csv`.

## Ongoing changes worth noting

- **2026-07: new connector.** The angled-disc design (rotating discs with
  fixed-bend clevis tabs, per-fabric optimised bend-magnitude inventory) was
  replaced by the pivoting connector of [connectors.md](connectors.md). All
  bend-angle machinery is gone from the CSV: the `Bend snap quality` header
  section became `Pivot elevation angles`, pull-row angle columns became
  free elevations, and the `tab` link rows were dropped (only `axial` and
  `radial` links remain). Slot step stays 6 mm (t_ring 5 + washer 1, as
  disc + separator was before) but the cable moment arm changed from
  ~24.5 mm (disc edge) to 32 mm (`R_tube`).
- **Cap thickness** was 6mm until 2026-05-08; corrected to 5mm.
- **Push radius** changed 25 → 20 mm earlier in the same week.

The engineer should always be working from the most recent zip. If something
looks off, check that the CSV's `Created:` timestamp is fresh.
