#!/usr/bin/env python3
"""Group OpenClaw's 180 pull intervals by 3-fold symmetry into a cable-order
CSV the manufacturer can read directly.

Usage:
    python3 scripts/build_cable_order.py OpenClaw-2026-05-17.csv

Most cables fall into rotational triples (one per leg) with identical length
— those become one row per triple, Quantity=3. The two triples whose cables
all attach to the apex strut `Z1:Z2` are an exception: they share that
single central-axis push, so their three rotational copies have to occupy
three different slots and therefore have three different lengths. Each
member of these apex triples gets its own row, Quantity=1.

Output columns (next to the input, suffix `-cable-order.csv`):

    GroupID, Length(mm), Quantity, Members

Member endpoints are formatted as `<joint>.<slot>` — the cable-end label that
gets engraved on the physical part. For the symbolic naming scheme this
depends on, see docs/joint-naming.md.
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
# Under the orbit-naming scheme (see docs/joint-naming.md):
#   - Off-axis joints look like `<leg><n>` (e.g. A12, B30, C5). Rotation
#     cycles the leading leg letter only; the index is preserved.
#   - Axis singletons look like `Z<n>` (e.g. Z1, Z2). They sit on the
#     rotation axis and map to themselves under 120°.

_LEG_JOINT = re.compile(r"^([ABC])(\d+)$")
_AXIS_JOINT = re.compile(r"^Z(\d+)$")


def _rotate_letter(c: str) -> str:
    return {"A": "B", "B": "C", "C": "A"}.get(c, c)


def rotate_label(label: str) -> str:
    m = _LEG_JOINT.match(label)
    if m:
        return _rotate_letter(m.group(1)) + m.group(2)
    return label  # axis singleton (Z<n>) or unrecognised shape — unchanged


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
    alpha_slot: int
    omega_slot: int
    length_mm: float

    @property
    def alpha_end(self) -> str:
        """Engraved label for the alpha end of this cable: `<joint>.<slot>`."""
        return f"{self.alpha}.{self.alpha_slot}"

    @property
    def omega_end(self) -> str:
        """Engraved label for the omega end of this cable: `<joint>.<slot>`."""
        return f"{self.omega}.{self.omega_slot}"


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
                alpha_slot=int(row[8]),
                omega_slot=int(row[14]),
                length_mm=float(row[2]) * 1000.0,
            ))
    return cables


# ── Grouping + summary ──────────────────────────────────────────────────────

def _touches_apex(key: tuple[str, str]) -> bool:
    """Cables touching an axis singleton (label shape `Z<n>`) can't have
    rotationally-equal lengths — their three copies share that single
    central-axis push end at three different slots."""
    return any(_AXIS_JOINT.match(label) for label in key)


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

    # Fold rows that round to the same integer mm: physically distinct
    # triples (or apex singletons) that share a length get combined into a
    # single order line with summed quantity. The engineer manufactures
    # cables by length, so merging two 500mm rows into one 500mm × 6 row
    # matches how the order is fulfilled.
    merged = []
    for r in rows:
        rounded = round(r["length"])
        if merged and merged[-1]["rounded"] == rounded:
            merged[-1]["qty"] += r["qty"]
            merged[-1]["members"].extend(r["members"])
        else:
            merged.append({"rounded": rounded, "qty": r["qty"], "members": list(r["members"])})

    out_path = args.source.with_name(f"{args.source.stem}-cable-order.csv")
    with out_path.open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["GroupID", "Length(mm)", "Quantity", "Members"])
        for i, r in enumerate(merged, start=1):
            members_str = " | ".join(
                f"{c.alpha_end}:{c.omega_end}" for c in r["members"]
            )
            w.writerow([i, r["rounded"], r["qty"], members_str])

    total_cables = sum(r["qty"] for r in merged)
    print(f"Wrote {out_path}", file=sys.stderr)
    print(f"  Cables ordered:  {total_cables}", file=sys.stderr)
    print(f"  Rows:            {len(merged)}", file=sys.stderr)
    if len(merged) < len(rows):
        print(f"  Merged {len(rows) - len(merged)} duplicate-length rows", file=sys.stderr)


if __name__ == "__main__":
    main()
