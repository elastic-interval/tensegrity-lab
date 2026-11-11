# Open Claw — Software & Engineering Summary

Summary of how the Obsidian vault project relates to the tensegrity-lab software. Written for the vault as a bridge document.

## The Software

The structure is designed and simulated in [tensegrity-lab](https://github.com/elastic-interval/tensegrity-lab), a Rust application using Elastic Interval Geometry (EIG) physics. The software:

1. **Builds** the structure from a DSL definition (`fabric_library.rs` → `OpenClaw`)
2. **Pretensions** cables in zero gravity, then drops with real gravity
3. **Settles** to final shape with frozen surface (joints that touch ground lock in place)
4. **Exported a CSV** with all interval coordinates, lengths, strains, and
   connector geometry, captured at the slack moment at the end of building.
   That export, and the tools that read it, are archived on Open Claw's old
   branch; Headless Hug goes out as JSON instead ([fabric-json.md](fabric-json.md)).

`cargo test --release --lib test_open_claw_threefold_symmetry` still asserts
that every rotational triple is symmetric. Open Claw's connector design is
frozen on its own branch; `dev` uses the newer Headless Hug prototype
dimensions.

Joints are named `A12` / `B12` / `C12` (the three sectors, clockwise seen
from above) and `Z1` / `Z2` on the axis; see
[joint-naming.md](joint-naming.md).

## OpenClaw Definition Parameters

The structure is defined in `src/build/dsl/fabric_library.rs` as:

- **Scale:** 0.80, with connectors
- **Seed:** OmniSymmetrical brick (12-strut hub with 8 faces)
- **3 legs:** columns of 4 bricks each on OmniBotX/Y/Z faces, ending in
  200% prisms; 200% prism on top
- **Open bottom:** OmniBot face removed
- **Omits:** six cables round the hub (`C2`–`B3`, `C2`–`A1`, …)
- **Spacing:** the three leg tips spread to 46%
- **Vulcanize:** 50% linear pre-vulcanize + 1 s vulcanize
- **Pretense:** 1% over 3 s, then fall and settle on a frozen surface

### Current Dimensions (scale 0.80)

| Metric | Value |
|--------|-------|
| Ground contacts | 3 (equilateral triangle) |
| Base triangle edges | ~6486mm |
| Height | ~7621mm |
| Struts | 46 (longest ~2477mm) |
| Cables | 180 (range ~267–1663mm) |
| Joints | 92 |

### Target Dimensions

| Metric | Target | Constraint |
|--------|--------|------------|
| Base triangle edges | **6000mm** | Tower spacing confirmed 6m (project lead, 25 Mar) |
| Height | ~8000–10000mm | ITW Amersfoort: 8m total incl truss; other festivals: no limit |
| Longest strut | ≤3600mm | Must fit in bestelbus cargo (3.6m long) |

## Achieving 6m Base

At scale 0.80 the base comes out 8% wide. Scale multiplies everything
linearly; the base-triangle test finds the scale that lands the base on
6 m: **0.74**, giving 5996mm edges, a 7050mm height and a longest strut of
2291mm, well inside the 3.6m van limit (but below the height target).

Other levers in the DSL (all affect base width):

| Parameter | Current | Effect |
|-----------|---------|--------|
| `scale` | 0.80 | Multiplies everything linearly |
| `space(Sec, tips, Pct)` | 46% | Pushes leg tips apart — directly widens base |
| `prism(Pct)` | 200% | Larger prisms at ends = wider footprint |
| `column(n)` | 4 | More bricks = longer legs |

## Test

`test_open_claw_base_triangle` in `src/open_claw_test.rs` builds Open Claw,
checks for exactly 3 ground contacts on an equilateral triangle (edges
within 2%), and rescales until the average edge is 6000mm ±2%, reporting
the scale, height and longest strut for each round.
`test_open_claw_foot_positions` prints the feet for Blender.

## Build Pipeline

```
DSL definition (fabric_library.rs)
  → FabricPlanExecutor runs: Build → Shape → Pretense → Fall → Settle
  → CSV export for ENS (slack moment, end of Build; now archived)
  → ENS structural analysis
  → Iterate: adjust DSL → re-export → re-analyze
```

## Key Files

| File | Purpose |
|------|---------|
| `src/build/dsl/fabric_library.rs` | OpenClaw definition |
| `src/open_claw_test.rs` | Base triangle and foot position tests |
| `src/build/dsl/fabric_plan_executor.rs` | Execution engine |
| `src/open_claw_symmetry.rs` | Threefold-symmetry enforcement |

## ENS Iteration Loop

The software is the starting point for the engineering iteration described in Construction.md:

1. Adjust OpenClaw parameters in `fabric_library.rs`
2. Run `test_open_claw_base_triangle` to verify 6m base + strut lengths
3. Export the CSV (on the archived branch)
4. Send CSV to the structural engineer
5. ENS feeds back on forces/dimensioning
6. Repeat until green-lit
