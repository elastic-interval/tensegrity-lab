---
name: fabric-build-check
description: >
  Minimum sanity check after touching DSL, brick or connector code. Runs the all-fabrics
  smoke test, the OpenClaw threefold-symmetry test and the Headless Hug buildability
  test, reports pass/fail tersely. If anything fails,
  surfaces the first error line per failure so you can see what broke without scrolling.
  Invoke when the user says "smoke test", "fabric check", "did I break anything",
  "build check", or invokes /fabric-build-check.
---

# fabric-build-check

Quick post-edit sanity check covering: (a) every named fabric still builds to completion
and its plan applies with no failed steps, (b) OpenClaw's geometry still satisfies
threefold symmetry, (c) Headless Hug's connectors have no collisions or short cables.

## Steps

1. **Build first.** `cargo build --release --lib 2>&1 | tail -20`. If this fails, stop —
   no point running tests against a non-compiling crate.

2. **Run the three tests in one cargo invocation:**
   ```bash
   cargo test --release --lib -- \
     test_all_fabrics_build test_open_claw_threefold_symmetry headless_hug_is_buildable 2>&1 | tail -30
   ```

3. **Report.** Build a tight summary:
   - ✅ / ❌ test_all_fabrics_build
   - ✅ / ❌ test_open_claw_threefold_symmetry
   - ✅ / ❌ headless_hug_is_buildable

   If any fail, also print:
   - The first assertion-failure line per failed test
   - The relevant fabric name (extract from the panic message — `"{fabric_name}: ..."`)

4. If all pass, output a single confirmation line: "Build clean, all three tests pass."

## Notes

- This skill is intentionally narrow: only the three highest-signal tests, not the full
  suite. For the whole suite, run `cargo test --release` directly.
- The smoke test (`test_all_fabrics_build`) iterates every `FabricName` variant and runs
  the build phase to completion. Panics during any fabric's build show up here.
- `test_open_claw_threefold_symmetry` re-asserts that every rotational triple of cables
  is symmetric in length/slot/bend after threefold symmetry is enforced. A failure here
  usually means the brick library or threefold-pasting logic shifted.
- Read-only — does not regenerate any artifacts. Use `/regen-openclaw` for that.
