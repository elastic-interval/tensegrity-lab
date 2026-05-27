---
name: regen-openclaw
description: >
  Regenerate the full OpenClaw artifact set (engineering CSV, assembly PDF, cable-order CSV)
  from the current branch. Auto-detects whether the CSV-writing test is the gated
  `export_open_claw_csv` (dev-style branches) or the always-on `test_open_claw_threefold_symmetry`
  (open-claw-style branches). Invoke when the user says "regen openclaw", "regenerate the
  CSV and PDF", "rebuild the assembly", or invokes /regen-openclaw.
---

# regen-openclaw

Run the OpenClaw artifact pipeline end-to-end and report what was produced.

## Steps

1. **Pick the CSV-writing test.** Look at `src/open_claw_symmetry.rs`:
   - If a test `export_open_claw_csv` exists and is marked `#[ignore]`, run it via:
     `cargo test --release --lib export_open_claw_csv -- --ignored`
   - Otherwise (older branches), `test_open_claw_threefold_symmetry` writes the CSV as a
     side-effect of its assertion. Run: `cargo test --release --lib test_open_claw_threefold_symmetry`
   The CSV lands at `OpenClaw-<today>.csv` in the repo root.

2. **Build the assembly PDF.**
   `python3 scripts/build_assembly.py OpenClaw-<today>.csv`
   Produces `OpenClaw-<today>-assembly.pdf` next to the CSV.

3. **Build the cable-order CSV.**
   `python3 scripts/build_cable_order.py OpenClaw-<today>.csv`
   Produces `OpenClaw-<today>-cable-order.csv` next to the CSV.

4. **Report.** Print:
   - Current branch (`git branch --show-current`)
   - The three artifact paths and their sizes
   - The summary lines that `build_assembly.py` prints (cables + length groups + strut pages)
   - The summary lines that `build_cable_order.py` prints (Cables ordered / Rows / Merged)
   - The CSV's header line 1 (height + creation timestamp) — useful at a glance

## Notes

- Do **not** move the artifacts into `docs/` by default. The repo root is the working area;
  the user decides what gets archived. If the user asks for `docs/` placement, move them.
- If the CSV-writing test fails, surface the failure clearly and stop — do not attempt the
  PDF or cable-order steps with a stale or missing CSV.
- `weasyprint` is required for the PDF step. If it's missing, the script self-exits with a
  brew-install hint — just relay that.
- This is purely a *regenerate* operation. It does not compare to any stored reference —
  use `/csv-diff` after if you want to verify against a baseline.
