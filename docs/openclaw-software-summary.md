# Open Claw — Software & Engineering Summary

Summary of how the Obsidian vault project relates to the tensegrity-lab software. Written for the vault as a bridge document.

## The Software

The structure is designed and simulated in [tensegrity-lab](https://github.com/elastic-interval/tensegrity-lab), a Rust application using Elastic Interval Geometry (EIG) physics. The software:

1. **Builds** the structure from a DSL definition (`fabric_library.rs` → `OpenClaw`)
2. **Pretensions** cables in zero gravity, then drops with real gravity
3. **Settles** to final shape with frozen surface (joints that touch ground lock in place)
4. **Exports CSV** with all interval coordinates, lengths, strains, and hinge geometry

A run with `--snapshot` produces a single `OpenClaw-slack.csv` — the input to
the engineer's FEA workflow. See [docs/csv-handoff.md](csv-handoff.md) for the
full handoff format.

## OpenClaw Definition Parameters

The structure is defined in `src/build/dsl/fabric_library.rs` as:

- **Seed:** OmniSymmetrical brick (12-strut hub with 8 faces)
- **3 legs:** columns of 4 bricks each on OmniBotX/Y/Z faces
- **Shrink:** 20% per brick step (each brick is 80% of previous)
- **Prisms:** 250% on leg ends, 200% on top
- **Open bottom:** OmniBot face removed
- **Spacing:** 35% at End marks
- **Vulcanize:** 50% linear pre-vulcanize + 1s vulcanize

### Current Dimensions (scale 1.0)

| Metric | Value |
|--------|-------|
| Ground contacts | 3 (equilateral triangle) |
| Base triangle edges | ~4780mm |
| Height | ~7739mm |
| Struts | 46 (longest ~3070mm) |
| Cables | 180 (range ~260–1950mm) |
| Joints | 92 |

### Target Dimensions

| Metric | Target | Constraint |
|--------|--------|------------|
| Base triangle edges | **6000mm** | Tower spacing confirmed 6m (project lead, 25 Mar) |
| Height | ~8000–10000mm | ITW Amersfoort: 8m total incl truss; other festivals: no limit |
| Longest strut | ≤3600mm | Must fit in bestelbus cargo (3.6m long) |

## Achieving 6m Base

Naive approach: scale from 1.0 to ~1.255 → 6m edges, but longest strut would become ~3850mm (exceeds van cargo length).

Alternative approaches available in the DSL (all affect base width):

| Parameter | Current | Effect |
|-----------|---------|--------|
| `scale` | 1.0 | Multiplies everything linearly |
| `space(Sec, End, Pct)` | 35% | Pushes leg ends apart — directly widens base |
| `shrink_by(Pct)` | 20% | Less shrink = longer legs = wider base |
| `prism(Pct)` | 250% | Larger prisms at ends = wider footprint |
| `column(n)` | 4 | More bricks = longer legs |

The `space` percentage is the most direct lever for base width without proportionally increasing strut lengths. Combinations can achieve 6m base while keeping max strut under 3.6m.

## Test

`test_open_claw_base_triangle` in `plan_runner_test.rs` asserts:
1. Exactly 3 ground contacts
2. Base triangle is equilateral (edges within 2% of each other)
3. Average edge length is 6000mm ±2%

Also reports height and max strut length for monitoring transport constraints.

## Build Pipeline

```
DSL definition (fabric_library.rs)
  → FabricPlanExecutor runs: Build → Shape → Pretense → Fall → Settle → GravPretense
  → Settled Fabric with final coordinates
  → CSV export for ENS
  → ENS structural analysis (Phase 3)
  → Iterate: adjust DSL → re-export → re-analyze
```

## Key Files

| File | Purpose |
|------|---------|
| `src/build/dsl/fabric_library.rs:34–72` | OpenClaw definition |
| `src/build/dsl/plan_runner_test.rs` | Base triangle test |
| `src/build/dsl/fabric_plan_executor.rs` | Execution engine |
| `src/fabric/csv_export.rs` | CSV export for ENS |
| `docs/csv-handoff.md` | CSV format + engineer's FEA workflow |

## ENS Iteration Loop

The software is the starting point for the engineering iteration described in Construction.md:

1. Adjust OpenClaw parameters in `fabric_library.rs`
2. Run `test_open_claw_base_triangle` to verify 6m base + strut lengths
3. Run application, export CSV
4. Send CSV to the structural engineer
5. ENS feeds back on forces/dimensioning
6. Repeat until green-lit
