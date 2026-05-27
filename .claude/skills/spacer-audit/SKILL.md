---
name: spacer-audit
description: >
  Audit a fabric's `space()` and `space_parallel()` calls in src/build/dsl/fabric_library.rs.
  Enumerates every pair that will be created, the role each will take (push/pull from
  the Pct vs 100% dispatch), counts intervals per spec, and flags conflicts (same pair
  appearing with both push and pull). Invoke when the user says "audit the spacers",
  "check my parallel block", "what spacers does HeadlessHug create", or invokes /spacer-audit.
  Optional argument: a fabric name (defaults to HeadlessHug, the most likely target).
---

# spacer-audit

Static analysis of a fabric's spacer specification — without building or running anything.

## Inputs

Optional fabric name as argument (`HeadlessHug` if none given). The fabric must be defined
in `src/build/dsl/fabric_library.rs`.

## Steps

1. **Locate the fabric's match arm** in `src/build/dsl/fabric_library.rs`.

2. **Find all `.space(...)` and `.space_parallel(...)` calls** in that arm. For
   `space_parallel`, walk each `spacer([…labels…], Pct(…))` line.

3. **For each spec, enumerate all pairs** (C(n,2) for an n-label list). Compute the
   role from the Pct value:
   - `Pct < 100.0` → **Pulling** (pull-shorten)
   - `Pct > 100.0` → **Pushing** (push-lengthen)
   - `Pct == 100.0` → no force (effectively inert)

4. **Build a per-pair table.** Key is the unordered pair `{A, B}` (canonicalise so order
   doesn't matter). Track all (Pct, role, source spec) entries.

5. **Render the result** as a markdown table:

   | Pair | Pct | Role | Source |
   |---|---|---|---|
   | LeftFoot ↔ LeftHand | 110% | push | spec #1 |
   | LeftFoot ↔ RightFoot | 30% | pull | spec #5 |
   | … | | | |

6. **Flag conflicts.** Any pair that appears with both push and pull from different specs
   is a conflict (the shorter target wins; the other interval goes slack). Output a
   distinct **⚠ Conflicts** section listing each.

7. **Summary line.** "N specs → M pairs total → K Pulling, L Pushing, X conflicts."

## Notes

- The "all pairs" semantics is the same primitive `space()` uses internally — every
  unordered pair gets one interval. For `[A, B, C]` that's 3 intervals; for `[A, B, C, D]`
  that's 6.
- An n-label spec with n ≥ 4 deserves a small callout in the summary — usually those are
  intentional (e.g. preserving a quadrilateral shape), but the C(n,2) explosion is
  surprising on first read.
- This is a static reader. It does **not** simulate or compute actual interval lengths.
  Distances are determined at runtime by `create_spacer` using the current geometry.
- Read-only; do not modify any source.
