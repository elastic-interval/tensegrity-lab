# CSV Handoff to Engineer

How the CSV export is structured and what the engineer's workflow with it
looks like. Source of truth for the format is `src/fabric/csv_export.rs`; the
single `Slack` broadcast lives in `src/build/dsl/fabric_plan_executor.rs`.

## One CSV: `slack`

The simulation exports exactly one CSV, captured right after slackening (end
of Building, before zero-G pretensing begins). Run with `--snapshot` on the
CLI and the file is written as `<FabricName>-slack.csv` (e.g.
`OpenClaw-slack.csv`).

Physical state at this moment: pulls have been given extra rest length
(slack); pushes have been snapped to discrete lengths. Joint positions are
end-of-build geometry. No pretension yet.

Earlier iterations of the codebase also emitted `pretenst`, `settled`, and
`grav_pretenst` snapshots, but the engineer's FEA workflow never consumed
them, so they were removed.

## What the engineer uses, and how

- Only the **geometry** is imported from the CSV — joint coordinates and
  element connectivity. Strains and forces from the simulation are not
  consumed.
- The engineer applies the pretension themselves in their FEA tool, then
  layers on the operational loads (gravity, wind, etc.) once the pretensioned
  structure is stable.

Implications for our exports:

- The slack joint coordinates are the end-of-build geometry. Pushes have
  already been snapped to discrete lengths (`snap_push_length`) at this point,
  so a push's rest length in the CSV does not exactly equal the geometric
  distance between its two joint coordinates. If both were imported, the small
  residual discrepancy would show up as initial strain in the FEA. With only
  coordinates imported, this does not bite.
- The pulls in the slack CSV have rest lengths that are
  `(1 + pull_lengthening)` times the geometric distance between their joints —
  i.e. they are intentionally slack. Again, irrelevant if only coordinates are
  imported.

## Column meanings — read carefully before sending anything to the factory

The data row format is:

```
Index, Role, Length(m), Strain,
AlphaX, AlphaY, AlphaZ, AlphaJoint, AlphaSlot, AlphaAngle,
OmegaX, OmegaY, OmegaZ, OmegaJoint, OmegaSlot, OmegaAngle
```

These columns mean *different physical things* depending on `Role`. Mixing
them up is the most likely path to a mis-fabricated part. The conventions
are unfortunately not symmetric between push and pull rows:

### Push rows (`Role = push`)

- `Length(m)` — **snapped rest length** of the strut tube. Already rounded
  to a discrete length via `HingeDimensions::snap_push_length`. This is what
  the strut should be **manufactured to**, including end-cap allowances.
- `AlphaXYZ`, `OmegaXYZ` — the **joint locations** in CSV coordinates (mm,
  Z-up). The Euclidean distance between them will be very close to
  `Length(m)` × 1000 but **not exactly equal**, because of the snap.
- `AlphaSlot`, `OmegaSlot` — always `0` for push rows (push intervals are
  not on a hinge).
- `AlphaAngle`, `OmegaAngle` — always `90` (axial; no hinge bend).

### Pull rows (`Role = pull`)

- `Length(m)` — the **distance between the two hinge endpoints**
  (`pull_end_pos`), *not* the slack rest length and *not* the joint-to-joint
  distance. It is the length the cable's tensioned segment would have if it
  spanned exactly between the bolt-and-disc terminations on each side, with
  the hinge mechanism's `length()` (≈ `t1/2 + D + E` ≈ 28.5 mm) accounted
  for at each end. **This is the closest thing in the CSV to "what the
  cable should be manufactured to" — but it is *not* the slack rest
  length.** The slack rest length used internally by the simulation is
  longer by the `pull_lengthening` factor; that number is not exported.
- `AlphaXYZ`, `OmegaXYZ` — the **hinge endpoint** at each end (i.e. the
  point on the bolt-and-disc termination where the cable attaches), *not*
  the joint location. Distance between them equals `Length(m)`. The actual
  joints are inset toward the strut centre by `length()` along the bend
  direction; the joint coordinates can be recovered from the corresponding
  push row at the same `AlphaJoint` / `OmegaJoint`.
- `AlphaSlot`, `OmegaSlot` — `1`-indexed slot at each end's hinge stack.
  The axial position of slot `k` along the strut, measured from the strut
  end, is `disc_center_offset(k - 1)` in `HingeDimensions`. **This depends
  on `cap_thickness`, `disc_thickness`, `disc_separator_thickness`, all of
  which have changed in recent iterations** — see Ongoing changes below.
- `AlphaAngle`, `OmegaAngle` — the **snapped hinge bend** at each end, in
  whole degrees. `+30` and `-30` are the same physical part installed in
  opposite orientations.

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
- For each cable: a length and (separately) the hinge bend angles at each
  end and the slot assignments. The "length" is **not** in the CSV in any
  directly usable form. The closest column is the pull row's `Length(m)`
  (hinge-endpoint to hinge-endpoint), but that omits the
  hinge-mechanism portion at each end, which is part of the physical cable
  routing. The slack rest length used by the simulation is `Length(m) ×
  (1 + pull_lengthening)`, where `pull_lengthening` lives in the fabric's
  `zero_g_pretense_phase` configuration (not in the CSV). If a single
  authoritative cable length is needed for fabrication, derive it in the
  FEA from the deployed equilibrium geometry rather than the CSV.
- For each hinge mechanism: `cap_thickness`, `disc_thickness`,
  `disc_separator_thickness`, the bend magnitudes set, and the slot
  inventory. All of this lives in the header parameters block and the
  bend-quality summary; it does **not** appear in the data rows.

Common pitfalls:

- **Computing element lengths from joint-to-joint Euclidean distance.** This
  is wrong for both pushes (snap drift) and especially pulls (the AlphaXYZ /
  OmegaXYZ are *hinge endpoints*, not joints, on pull rows).
- **Reading the AlphaSlot column as if it were a node index.** It's a slot
  number on the hinge stack at that joint.
- **Reusing factory drawings across simulation iterations.** If
  `cap_thickness` or any other hinge dimension changed between runs, slot
  axial positions shifted, and the manufactured strut endcaps no longer
  match the cable attachment positions. The CSV's `Created:` timestamp is
  the version marker.
- **Manufacturing a hinge inventory at iteration N's optimised
  magnitudes, then running iteration N+1.** The optimiser is currently
  free to pick fresh magnitudes per fabric. Once parts are made, the
  intended set should be locked in (a "freeze magnitudes" mode is
  outstanding work; until it lands, freezing happens by editing the
  source).

## CSV header layout

```
# OpenClaw, Phase: <moment>, Height: ...mm, Created: <timestamp>
#
# === Hinge parameters (see diagram) ===
# A  push_radius
# B  push_radius_margin
# C  offset (= t1/2)
# D  hinge_extension
# E  hinge_hole_diameter
# t1 disc_thickness
# t2 disc_separator_thickness
#    cap_thickness
#    pull_radius
#
# === Afgeleide waarden ===
#    A + B + C  (halve breedte schijf)
#    C + D + E  (scharnier lengte)
#    t1 + t2    (schijf + separator)
#    disc_center_offset(0)
#
# Orientation check (CSV coords, mm, Z-up): ground plane at Z=0, apex at Z=...
# Lowest[1..3]: joint=...
# Highest:      joint=...
#
# === Hinge bend snap quality ===
# Bend count (K):       4
# Optimal magnitudes:   [...]
# Effective signed set: [...]
# Bend counts (signed): ...
# Bend counts (per magnitude): ...
# Cable ends measured:  ...
# Snap error:           mean / max / RMS
#
Index,Role,Length(m),Strain,AlphaX,...,AlphaAngle,OmegaX,...,OmegaAngle
<rows>
```

The hinge parameters block sits early on purpose: a downstream Grasshopper
"Overview of design parameters" panel shows roughly the first 12 lines of the
CSV. With the parameters block in lines 2–11, all of A–E plus t1 and t2 are
visible without resizing the panel.

## Coordinate system

The CSV is written in **Z-up** (Rhino/RFEM convention) while the simulation
runs in Y-up. The transform is `Mat3::from_rotation_x(π/2)` applied to every
exported position: sim (x, y, z) → csv (x, z, −y). See
`csv_export.rs::sim_to_csv`.

## Hinge bend angles (per cable end)

The `AlphaAngle` and `OmegaAngle` columns hold the snapped hinge bend angle
at each cable end. Snapping uses the optimised magnitudes computed by
`Fabric::recompute_bend_magnitudes` (1D k-center on the ideal-angle
distribution), called automatically when the fabric reaches Viewing and at
the start of CSV export. Number of distinct magnitudes is configured by
`HingeDimensions::bend_count` (currently 4).

For push rows, both columns are `90` (no hinge — the push tube is
axial). For pull rows, the values are the snapped angles in degrees, e.g.
`+31`, `-49`, `0`. The `# Bend counts` lines in the header give a histogram
so the engineer can see how many of each manufactured angle to produce.

## Ongoing changes worth noting

- **Cap thickness** was 6mm until 2026-05-08; corrected to 5mm (matches `t1`)
  per the engineer's note that `disc_center_offset(0)` should equal `t1 + t2 + t1/2 = 8.5 mm`.
- **Push radius** changed 25 → 20 mm earlier in the same week.
- **Disc thickness (`t1`)** changed 6 → 5 mm.
- **Bend angles** moved from a fixed set `{-60, -30, 0, +30, +60}` to per-fabric
  optimised magnitudes (k-center, K configurable, default 4, rounded to whole
  degrees).

The engineer should always be working from the most recent zip. If something
looks off, check that the CSV's `Created:` timestamp is fresh.
