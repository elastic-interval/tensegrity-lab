# CSV Handoff to Engineer

How the CSV exports are structured, which one the engineer (Peter) uses, and what
his workflow with it looks like. Source of truth for the format is
`src/fabric/csv_export.rs`; for snapshot moments, `src/lib.rs::SnapshotMoment`
and the broadcasts in `src/build/dsl/fabric_plan_executor.rs`.

## Snapshot moments

The simulation broadcasts `SnapshotMoment` events at four distinct stages of the
build → pretension → drop pipeline. Each moment, when matched against
`--snapshot=<moment>` on the CLI (or `All`), triggers a CSV export with the
moment's suffix in the filename (e.g. `OpenClaw-slack.csv`).

| Moment | Suffix | When | Physical state |
|---|---|---|---|
| `Slack` | `slack` | After slackening, before zero-G pretensing begins | Pulls have been given extra rest length (slack); pushes have been snapped to discrete lengths. Joint positions are end-of-build geometry. No pretension yet. |
| `Pretenst` | `pretenst` | After zero-G pretension equilibrium | Pushes incrementally extended in symmetric groups until each reaches target compression strain. No gravity. |
| `Settled` | `settled` | After fall + settle, when no `grav_pretense` phase exists | Structure has dropped onto the surface and reached static equilibrium under gravity. |
| `GravPretenst` | `grav_pretenst` | After gravitational re-pretensioning | A second pretension pass with gravity active. Final deployed-state geometry. |

`Settled` and `GravPretenst` are **mutually exclusive**: a fabric plan with a
`.grav_pretense(...)` step (such as OpenClaw) emits `GravPretenst`; a plan
without it emits `Settled`. So a typical fabric produces three CSVs in one run:
`slack`, `pretenst`, plus one of `settled`/`grav_pretenst`.

## Pretension is iterative, not single-step

The `pretenst` and `grav_pretenst` moments are not simple instantaneous loads.
The simulation reaches them through a stepwise process:

1. **Symmetric groups** are formed from the pushes only (one group per
   `(depth, axis)` of the alpha joint's path) — see
   `Fabric::discover_symmetric_groups` in
   `src/build/dsl/fabric_plan_executor.rs`.
2. The group with the **highest (least compressed) strain** that has not yet
   reached `min_push_strain` is selected.
3. Every push in that group has its rest length increased by
   `dimensions.push_length_increment` (one tick per call to
   `extend_symmetric_group`). Longer rest length means the push pries its joints
   further apart; the slack pulls between those joints stretch and gain tension.
4. The system iterates physics until equilibrium re-settles.
5. Steps 2–4 repeat until every group meets the target strain. The final state
   is broadcast as the moment.

This ordering — pushes incrementally extending, pulls passively pulled into
tension — is the opposite of "tighten the cables." Cables are not actively
shortened anywhere in the pipeline.

## What Peter uses, and how

Peter (engineer) takes the **`slack`** CSV as input to his FEA workflow. He has
confirmed:

- He imports **only the geometry** from the CSV — joint coordinates and
  element connectivity. He does not consume strains or forces from the
  simulation.
- He applies the pretension himself in his FEA tool, then layers on the
  operational loads (gravity, wind, etc.) once the pretensioned structure is
  stable.

Implications for our exports:

- The `slack` joint coordinates are the end-of-build geometry. Pushes have
  already been snapped to discrete lengths (`snap_push_length`) at this point,
  so a push's rest length in the CSV does not exactly equal the geometric
  distance between its two joint coordinates. If Peter were importing both, the
  small residual discrepancy would show up as initial strain in his FEA. He
  imports only coordinates, so this does not bite him.
- The pulls in the `slack` CSV have rest lengths that are
  `(1 + pull_lengthening)` times the geometric distance between their joints —
  i.e. they are intentionally slack. Again, irrelevant if only coordinates are
  imported.
- If verification of our pretensioning solution is ever desired, the
  `pretenst` and `grav_pretenst` CSVs give equilibrium geometries that can be
  compared against the same in his FEA. Independent cross-check, not part of
  the active workflow.

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

The hinge parameters block sits early on purpose: Peter's Grasshopper "Overview
of design parameters" panel shows roughly the first 12 lines of the CSV. With
the parameters block in lines 2–11, all of A–E plus t1 and t2 are visible
without resizing the panel.

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
  per Peter's note that `disc_center_offset(0)` should equal `t1 + t2 + t1/2 = 8.5 mm`.
- **Push radius** changed 25 → 20 mm earlier in the same week.
- **Disc thickness (`t1`)** changed 6 → 5 mm.
- **Bend angles** moved from a fixed set `{-60, -30, 0, +30, +60}` to per-fabric
  optimised magnitudes (k-center, K configurable, default 4, rounded to whole
  degrees).

Peter should always be working from the most recent zip. If something looks off,
check that the CSV's `Created:` timestamp is fresh.
