---
name: csv-diff
description: >
  Structured comparison of two OpenClaw engineering CSVs. Reports row counts by role,
  per-cable length diff (sorted-pair mean/max |Δ|), per-push length diff, height
  delta, and header changes. Invoke when the user says "diff these csvs",
  "compare CSV", "is the new CSV identical", or invokes /csv-diff. The arguments are
  the two CSV paths (old, new).
---

# csv-diff

Compare two OpenClaw CSVs (the schema produced by `write_csv` in `src/open_claw_symmetry.rs`)
and surface what changed structurally and numerically.

## Inputs

Two CSV paths as arguments. By convention, the first is the **stored reference** (older,
typically in `docs/`) and the second is the **candidate** (newer, possibly in repo root).

## Steps

Run all probes in a single bash block (they're independent and cheap).

1. **Row count by role.** Both CSVs should have the same counts. Tabular output:
   ```bash
   awk -F, 'NR>41 {print $2}' <csv> | sort | uniq -c
   ```
   Roles expected: `push`, `push-fea`, `pull`, `pull-fea`, `radial`, `tab`, `axial`.

2. **Header changes (skipping line 1, which has the timestamp).**
   ```bash
   diff <(tail -n +2 <old> | head -40) <(tail -n +2 <new> | head -40)
   ```

3. **Per-cable length diff (sorted-pair pairing).**
   ```bash
   paste <(awk -F, 'NR>41 && $2=="pull" {print $3}' <old> | sort -n) \
         <(awk -F, 'NR>41 && $2=="pull" {print $3}' <new> | sort -n) \
     | awk '{d=($2-$1)*1000; a=(d<0?-d:d); s+=a; if(a>m)m=a; n++} END {printf "n=%d  mean |Δ|=%.4f mm  max |Δ|=%.4f mm\n", n, s/n, m}'
   ```

4. **Per-push length diff.** Same as above but `$2=="push"`.

5. **Height comparison.** From line 1's `Height: <X>mm`:
   ```bash
   head -1 <old> ; head -1 <new>
   ```

## Output

Render a single markdown table summarizing the key numbers:

| Aspect | OLD | NEW | Δ |
|---|---|---|---|
| Push count | … | … | … |
| Pull count | … | … | … |
| Push mean / max length | … / … m | … / … m | … / … |
| Pull mean / max length | … / … m | … / … m | … / … |
| Height | … mm | … mm | … |
| Per-cable sorted-pair mean / max | — | — | … / … mm |

Then a short verdict line:
- **"Bit-identical (modulo timestamp)"** if header diff is empty and per-cable mean+max are 0.0 mm
- **"Naming-only differences"** if length stats are 0.0 but joint names differ (compare a few `$8/$14` samples)
- **"Geometry shifted"** with the magnitude if mean |Δ| > 0.1 mm

## Notes

- Sorted-pair pairing is approximate: it assumes the multiset of lengths is the same shape.
  If counts differ between the two CSVs, flag that loudly — the pairing is meaningless.
- The first 41 lines are header (`#` comments + the column header). Adjust if the header
  format changes.
- This is read-only — never modify the CSVs.
