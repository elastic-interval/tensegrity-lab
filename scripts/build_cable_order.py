#!/usr/bin/env python3
"""Group OpenClaw's 180 pull intervals by 3-fold symmetry into a cable-order
CSV the manufacturer can read directly.

Usage:
    python3 scripts/build_cable_order.py OpenClaw-2026-05-17.csv

Most cables fall into rotational triples (one per leg) with identical length
— those become one row per triple, Quantity=3. The two triples whose cables
all attach to the apex strut `YZ0↔YZ1` are an exception: they share that
single central-axis push, so their three rotational copies have to occupy
three different slots and therefore have three different lengths. Each
member of these apex triples gets its own row, Quantity=1.

Output columns (next to the input, suffix `-cable-order.csv`):

    GroupID, Length(mm), Quantity, Members

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

def _touches_apex(key: tuple[str, str]) -> bool:
    """Cables touching the central-axis apex push `YZ0↔YZ1` can't have
    rotationally-equal lengths — their three copies share that single push
    at three different slots."""
    return "YZ0" in key or "YZ1" in key


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path, help="Input slack CSV")
    args = parser.parse_args()

    cables = parse_pulls(args.source)
    if not cables:
        raise SystemExit(f"{args.source}: no pull rows found")

    groups: dict[tuple[str, str], list[Cable]] = defaultdict(list)
    for c in cables:
        groups[canonical_key(c.alpha, c.omega)].append(c)

    # One row per orderable cable spec. Non-apex triples collapse to a single
    # row (Quantity=3). Apex triples expand to one row per cable (Quantity=1).
    rows = []
    for key, members in groups.items():
        if _touches_apex(key):
            for c in members:
                rows.append({"length": c.length_mm, "qty": 1, "members": [c]})
        else:
            mean = sum(c.length_mm for c in members) / len(members)
            rows.append({"length": mean, "qty": len(members), "members": members})
    rows.sort(key=lambda r: r["length"])

    out_path = args.source.with_name(f"{args.source.stem}-cable-order.csv")
    with out_path.open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["GroupID", "Length(mm)", "Quantity", "Members"])
        for i, r in enumerate(rows, start=1):
            members_str = " | ".join(f"{c.alpha}↔{c.omega}" for c in r["members"])
            w.writerow([i, round(r["length"]), r["qty"], members_str])

    total_cables = sum(r["qty"] for r in rows)
    print(f"Wrote {out_path}", file=sys.stderr)
    print(f"  Cables ordered:  {total_cables}", file=sys.stderr)
    print(f"  Rows:            {len(rows)}", file=sys.stderr)


if __name__ == "__main__":
    main()
