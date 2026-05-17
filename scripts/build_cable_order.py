#!/usr/bin/env python3
"""Group OpenClaw's 180 pull intervals into 60 triples by 3-fold symmetry
and write a cable-order CSV.

Usage:
    python3 scripts/build_cable_order.py OpenClaw-2026-05-17.csv

The output CSV (next to the input, suffix `-cable-order.csv`) lists 60 rows
— one per unique cable length — with columns:

    GroupID, Length(mm), Quantity, LengthMin(mm), LengthMax(mm), Spread(mm),
    Members

Each group's `Quantity` is always 3 (the three rotational copies). `Spread`
is `Max − Min` within the triple; ideally zero, in practice sub-mm.

A summary to stderr reports total cables, spread statistics, and lists any
outlier triples whose spread exceeds 1 mm — so you can spot anomalies before
sending the order.

For the symbolic naming scheme this depends on, see docs/joint-naming.md.
"""

from __future__ import annotations

import argparse
import csv
import re
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

# ── Joint-label rotation ────────────────────────────────────────────────────
# Under 120° rotation about the central axis, leg letters cycle A→B→C→A.
# The leg letter sits at:
#   - the FIRST character for path joints (AX4YZ1, BX1Z4, CX2YZ0, …)
#   - the LAST character for seed joints (BAA, TOC, …) — the leading 2 chars
#     are a category code (BA/BO/TA/TO)
#   - nowhere for apex prism joints (YZ0, YZ1) — those are on the central
#     axis and don't rotate.

_SEED_JOINT = re.compile(r"^[BT][AO][ABC]$")
_PATH_JOINT_LEAD = re.compile(r"^[ABC]")


def _rotate_letter(c: str) -> str:
    return {"A": "B", "B": "C", "C": "A"}.get(c, c)


def rotate_label(label: str) -> str:
    if _SEED_JOINT.match(label):
        return label[:2] + _rotate_letter(label[2])
    if _PATH_JOINT_LEAD.match(label):
        return _rotate_letter(label[0]) + label[1:]
    return label  # apex prism etc.


def canonical_key(a: str, b: str) -> tuple[str, str]:
    """Smallest (lex) of the six rotation × end-swap equivalents of (a, b)."""
    variants = []
    aa, bb = a, b
    for _ in range(3):
        variants.append((aa, bb))
        variants.append((bb, aa))
        aa, bb = rotate_label(aa), rotate_label(bb)
    return min(variants)


# ── CSV ingestion ───────────────────────────────────────────────────────────

@dataclass
class Cable:
    alpha: str
    omega: str
    length_mm: float


def parse_pulls(path: Path) -> list[Cable]:
    cables: list[Cable] = []
    with path.open() as f:
        header_done = False
        for row in csv.reader(f):
            if not row or row[0].startswith("#"):
                continue
            if not header_done:
                if row[0] == "Index":
                    header_done = True
                continue
            if len(row) < 16 or row[1] != "pull":
                continue
            cables.append(Cable(
                alpha=row[7],
                omega=row[13],
                length_mm=float(row[2]) * 1000.0,
            ))
    return cables


# ── Grouping + summary ──────────────────────────────────────────────────────

def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path, help="Input slack CSV")
    parser.add_argument(
        "--outlier-threshold-mm", type=float, default=1.0,
        help="Spread (mm) above which a triple is reported as an outlier (default 1.0)",
    )
    args = parser.parse_args()

    cables = parse_pulls(args.source)
    if not cables:
        raise SystemExit(f"{args.source}: no pull rows found")

    groups: dict[tuple[str, str], list[Cable]] = defaultdict(list)
    for c in cables:
        groups[canonical_key(c.alpha, c.omega)].append(c)

    # Sort triples by length (ascending) for the final order.
    rows = []
    for (ka, kb), members in groups.items():
        lengths = sorted(c.length_mm for c in members)
        mean = sum(lengths) / len(lengths)
        spread = lengths[-1] - lengths[0]
        rows.append({
            "key": (ka, kb),
            "members": members,
            "lengths": lengths,
            "mean": mean,
            "min": lengths[0],
            "max": lengths[-1],
            "spread": spread,
        })
    rows.sort(key=lambda r: r["mean"])

    # ── Write output CSV ────────────────────────────────────────────────────
    out_path = args.source.with_name(f"{args.source.stem}-cable-order.csv")
    with out_path.open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow([
            "GroupID", "Length(mm)", "Quantity",
            "LengthMin(mm)", "LengthMax(mm)", "Spread(mm)",
            "Members",
        ])
        for i, r in enumerate(rows, start=1):
            members_str = " | ".join(
                f"{c.alpha}↔{c.omega}" for c in r["members"]
            )
            w.writerow([
                i,
                f"{r['mean']:.2f}",
                len(r["members"]),
                f"{r['min']:.2f}",
                f"{r['max']:.2f}",
                f"{r['spread']:.2f}",
                members_str,
            ])

    # ── Diagnostics to stderr ───────────────────────────────────────────────
    total_cables = sum(len(r["members"]) for r in rows)
    triple_count = sum(1 for r in rows if len(r["members"]) == 3)
    wrong_size = [r for r in rows if len(r["members"]) != 3]
    all_spreads = [r["spread"] for r in rows]
    outliers = sorted(
        (r for r in rows if r["spread"] > args.outlier_threshold_mm),
        key=lambda r: r["spread"], reverse=True,
    )

    print(f"Wrote {out_path}", file=sys.stderr)
    print(f"  Cables read:       {total_cables}", file=sys.stderr)
    print(f"  Groups:            {len(rows)}", file=sys.stderr)
    print(f"  Triples (size 3):  {triple_count}", file=sys.stderr)
    if wrong_size:
        print(
            f"  ⚠ Groups not of size 3: {len(wrong_size)}",
            file=sys.stderr,
        )
        for r in wrong_size:
            ka, kb = r["key"]
            print(
                f"    {ka}↔{kb}  size={len(r['members'])}",
                file=sys.stderr,
            )
    print(
        f"  Spread within triple:  min={min(all_spreads):.2f}mm  "
        f"mean={sum(all_spreads)/len(all_spreads):.2f}mm  "
        f"max={max(all_spreads):.2f}mm",
        file=sys.stderr,
    )
    if outliers:
        print(
            f"  Outliers (spread > {args.outlier_threshold_mm}mm): "
            f"{len(outliers)}",
            file=sys.stderr,
        )
        for r in outliers:
            ka, kb = r["key"]
            ls = "/".join(f"{l:.2f}" for l in r["lengths"])
            print(
                f"    {ka}↔{kb}  lengths={ls}mm  spread={r['spread']:.2f}mm",
                file=sys.stderr,
            )
    else:
        print(
            f"  No outliers above {args.outlier_threshold_mm}mm threshold.",
            file=sys.stderr,
        )


if __name__ == "__main__":
    main()
