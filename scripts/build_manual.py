#!/usr/bin/env python3
"""Build the OpenClaw construction manual from a CSV export.

Usage:
    python3 scripts/build_manual.py path/to/OpenClaw-slack.csv

Reads the CSV, pivots its per-interval data into a per-strut-end assembly
view, and writes a Markdown manual next to the input.

CSV is the sole input. Static prose (materials, sign convention, assembly
tips) lives in this script. See `docs/csv-handoff.md` for the CSV format.
"""

from __future__ import annotations

import argparse
import csv
import io
import re
import sys
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional


# ---------------------------------------------------------------- Data model

@dataclass
class HingeDimensions:
    """Hinge geometry parsed from the CSV header (all values in millimetres)."""

    push_radius: float           # A
    push_radius_margin: float    # B
    offset_through_radius: float # C (= t1/2)
    hinge_extension: float       # D
    hinge_hole_diameter: float   # E
    disc_thickness: float        # t1
    disc_separator: float        # t2
    cap_thickness: float
    pull_radius: float

    def disc_center_offset_mm(self, slot: int) -> float:
        """Axial offset from strut end to the centre of disc at `slot` (1-indexed)."""
        # cap + sep + t1/2 + (slot-1) * (t1 + sep)
        step = self.disc_thickness + self.disc_separator
        return self.cap_thickness + self.disc_separator + self.disc_thickness / 2.0 + step * (slot - 1)


@dataclass
class Strut:
    leg: str                # 'A', 'B', 'C', or 'H' (hub)
    pos: int                # 1-indexed position within its leg
    alpha_joint: str
    omega_joint: str
    length_mm: float
    alpha_xyz: tuple
    omega_xyz: tuple

    @property
    def label(self) -> str:
        return f"S{self.leg}{self.pos:02d}"

    @property
    def axis_outward_from_alpha(self) -> tuple:
        dx = self.omega_xyz[0] - self.alpha_xyz[0]
        dy = self.omega_xyz[1] - self.alpha_xyz[1]
        dz = self.omega_xyz[2] - self.alpha_xyz[2]
        length = (dx * dx + dy * dy + dz * dz) ** 0.5 or 1.0
        # Outward from alpha end means *from* alpha *toward* omega, then reversed
        # because "outward" means away from the strut body. At the alpha end,
        # outward = -push_dir; at the omega end, outward = +push_dir.
        return (-dx / length, -dy / length, -dz / length)


@dataclass
class Disc:
    """One disc in a strut-end stack: slot, signed bend angle, cable assignment."""

    slot: int                  # 1-indexed
    angle: int                 # signed, snapped to the locked magnitudes
    cable_label: str           # e.g. "CA01", "CHA03"


@dataclass
class Cable:
    category: str               # 'CA','CB','CC' (intra-leg); 'CHA','CHB','CHC' (hub-leg); 'CH' (intra-hub)
    pos: int                    # 1-indexed within category
    length_mm: float
    alpha_strut_end: str        # e.g. "SA01A"
    omega_strut_end: str        # e.g. "SC04B"
    alpha_joint: str
    omega_joint: str
    alpha_slot: int
    omega_slot: int
    alpha_angle: int
    omega_angle: int

    @property
    def label(self) -> str:
        return f"{self.category}{self.pos:02d}"


@dataclass
class ManualData:
    fabric_name: str
    phase: str
    height_mm: float
    created: str
    csv_path: Path
    hinge: HingeDimensions
    magnitudes: list           # locked set, ascending non-negative
    magnitudes_locked: bool
    bend_counts_signed: str    # raw text echoing the CSV header line
    bend_counts_per_magnitude: dict   # {magnitude: count}
    struts: list               # list[Strut]
    cables: list               # list[Cable]
    strut_ends: dict           # {(strut_label, "A"|"B"): list[Disc]} (sorted by slot)


# --------------------------------------------------------------- CSV parsing

_HEADER_TITLE = re.compile(r"^# (.*?), Phase:\s*(\S+),\s*Height:\s*([\d.]+)mm,\s*Created:\s*(.+)$")
_HINGE_PARAM = re.compile(r"^#\s+(?:[A-E]\s+)?\S.*?\(([a-z_]+)\):\s*([\d.]+)mm")
_HINGE_PARAM_PLAIN = re.compile(r"^#\s+([a-z_]+):\s*([\d.]+)mm")  # for cap_thickness, pull_radius
_OPTIMAL_MAGS = re.compile(r"^#\s+Optimal magnitudes:\s*\[(.+)\]")
_LOCKED = re.compile(r"^#\s+Magnitude source:.*LOCKED")
_BEND_COUNTS_SIGNED = re.compile(r"^#\s+Bend counts \(signed\):\s+(.+)$")
_BEND_COUNTS_MAG = re.compile(r"^#\s+Bend counts \(per magnitude\):\s+(.+)$")
_MAG_COUNT_PAIR = re.compile(r"(\d+)°×(\d+)")


def parse_csv(path: Path) -> ManualData:
    text = path.read_text(encoding="utf-8")
    header_lines: list[str] = []
    data_lines: list[str] = []
    seen_data_header = False
    for line in text.splitlines():
        if line.startswith("#"):
            header_lines.append(line)
        elif not seen_data_header and line.startswith("Index,"):
            seen_data_header = True
        elif seen_data_header:
            data_lines.append(line)

    # --- Header parsing ---
    title = next((m for m in (_HEADER_TITLE.match(l) for l in header_lines) if m), None)
    if title is None:
        raise SystemExit("CSV missing title line")
    fabric_name, phase, height_mm_str, created = title.groups()

    hinge_values = {}
    for line in header_lines:
        m = _HINGE_PARAM.match(line) or _HINGE_PARAM_PLAIN.match(line)
        if m:
            hinge_values[m.group(1)] = float(m.group(2))

    needed = [
        "push_radius", "push_radius_margin", "hinge_extension",
        "hinge_hole_diameter", "disc_thickness", "disc_separator",
        "cap_thickness", "pull_radius",
    ]
    missing = [k for k in needed if k not in hinge_values]
    if missing:
        raise SystemExit(f"CSV header missing hinge parameter(s): {missing}")

    hinge = HingeDimensions(
        push_radius=hinge_values["push_radius"],
        push_radius_margin=hinge_values["push_radius_margin"],
        offset_through_radius=hinge_values["disc_thickness"] / 2.0,
        hinge_extension=hinge_values["hinge_extension"],
        hinge_hole_diameter=hinge_values["hinge_hole_diameter"],
        disc_thickness=hinge_values["disc_thickness"],
        disc_separator=hinge_values["disc_separator"],
        cap_thickness=hinge_values["cap_thickness"],
        pull_radius=hinge_values["pull_radius"],
    )

    mags_match = next((m for m in (_OPTIMAL_MAGS.match(l) for l in header_lines) if m), None)
    if mags_match is None:
        raise SystemExit("CSV header missing '# Optimal magnitudes' line")
    magnitudes = [int(s.strip().rstrip("°")) for s in mags_match.group(1).split(",")]

    locked = any(_LOCKED.match(l) for l in header_lines)

    bend_signed = ""
    for line in header_lines:
        m = _BEND_COUNTS_SIGNED.match(line)
        if m:
            bend_signed = m.group(1).strip()

    bend_counts_per_magnitude: dict[int, int] = {}
    for line in header_lines:
        m = _BEND_COUNTS_MAG.match(line)
        if m:
            for mag_str, count_str in _MAG_COUNT_PAIR.findall(m.group(1)):
                bend_counts_per_magnitude[int(mag_str)] = int(count_str)

    # --- Data rows ---
    reader = csv.reader(io.StringIO("\n".join(data_lines)))
    pushes_raw = []
    pulls_raw = []
    for row in reader:
        if len(row) < 16:
            continue
        role = row[1]
        if role == "push":
            pushes_raw.append(row)
        elif role == "pull":
            pulls_raw.append(row)
        # `push-fea`, `pull-fea`, `axial`, `radial`, `hinge` are intentionally skipped.

    # --- Build struts: classify by leg from joint-path prefix, then number within each leg.
    # Joint paths start with 'A', 'B', or 'C' for the three legs; anything else is hub ('H').
    # Both ends of a strut must agree; cross-leg struts would indicate a parser bug.
    def classify(joint: str) -> str:
        prefix = joint[:1]
        return prefix if prefix in ("A", "B", "C") else "H"

    leg_counters: dict[str, int] = {"A": 0, "B": 0, "C": 0, "H": 0}
    struts: list[Strut] = []
    joint_to_strut: dict[str, tuple[str, str]] = {}   # joint -> (label, end_letter)
    for row_idx, row in enumerate(pushes_raw, start=1):
        ax, ay, az = float(row[4]), float(row[5]), float(row[6])
        alpha_joint = row[7]
        ox, oy, oz = float(row[10]), float(row[11]), float(row[12])
        omega_joint = row[13]
        leg_alpha = classify(alpha_joint)
        leg_omega = classify(omega_joint)
        if leg_alpha != leg_omega:
            raise SystemExit(
                f"Cross-leg strut at CSV push row {row_idx}: "
                f"alpha {alpha_joint!r} ({leg_alpha}) vs omega {omega_joint!r} ({leg_omega}). "
                f"Classification rule needs review."
            )
        leg = leg_alpha
        leg_counters[leg] += 1
        pos = leg_counters[leg]
        length_mm = float(row[2]) * 1000.0
        s = Strut(
            leg=leg,
            pos=pos,
            alpha_joint=alpha_joint,
            omega_joint=omega_joint,
            length_mm=length_mm,
            alpha_xyz=(ax, ay, az),
            omega_xyz=(ox, oy, oz),
        )
        struts.append(s)
        if alpha_joint in joint_to_strut or omega_joint in joint_to_strut:
            raise SystemExit(
                f"Joint sharing detected at CSV push row {row_idx}; manual cannot "
                f"unambiguously map joints to strut ends."
            )
        joint_to_strut[alpha_joint] = (s.label, "A")
        joint_to_strut[omega_joint] = (s.label, "B")

    # --- Build cables grouped by topological category (intra-leg / hub-to-leg / intra-hub) ---
    # No cross-leg cables exist in OpenClaw; we fail loudly if one ever shows up.
    cable_category_map = {
        ("A", "A"): "CA", ("B", "B"): "CB", ("C", "C"): "CC",
        ("H", "H"): "CH",
        ("A", "H"): "CHA", ("B", "H"): "CHB", ("C", "H"): "CHC",
    }

    def cable_category(leg_a: str, leg_b: str) -> str:
        key = tuple(sorted([leg_a, leg_b]))
        if key not in cable_category_map:
            raise SystemExit(f"Unexpected cable category for legs {leg_a}/{leg_b}: "
                             f"cross-leg cables aren't expected in OpenClaw.")
        return cable_category_map[key]

    cable_counters: dict[str, int] = defaultdict(int)
    cables: list[Cable] = []
    strut_ends: dict[tuple[str, str], list[Disc]] = defaultdict(list)
    for i, row in enumerate(pulls_raw, start=1):
        length_mm = float(row[2]) * 1000.0
        alpha_joint = row[7]
        alpha_slot = int(row[8])
        alpha_angle = int(row[9])
        omega_joint = row[13]
        omega_slot = int(row[14])
        omega_angle = int(row[15])
        if alpha_joint not in joint_to_strut:
            raise SystemExit(f"Pull row {i}: alpha joint {alpha_joint!r} not found in any strut")
        if omega_joint not in joint_to_strut:
            raise SystemExit(f"Pull row {i}: omega joint {omega_joint!r} not found in any strut")
        alpha_strut_label, alpha_end_letter = joint_to_strut[alpha_joint]
        omega_strut_label, omega_end_letter = joint_to_strut[omega_joint]
        leg_alpha = alpha_strut_label[1]  # 'S{leg}{pos}' → leg is index 1
        leg_omega = omega_strut_label[1]
        category = cable_category(leg_alpha, leg_omega)
        cable_counters[category] += 1
        pos = cable_counters[category]
        cable = Cable(
            category=category,
            pos=pos,
            length_mm=length_mm,
            alpha_strut_end=f"{alpha_strut_label}{alpha_end_letter}",
            omega_strut_end=f"{omega_strut_label}{omega_end_letter}",
            alpha_joint=alpha_joint,
            omega_joint=omega_joint,
            alpha_slot=alpha_slot,
            omega_slot=omega_slot,
            alpha_angle=alpha_angle,
            omega_angle=omega_angle,
        )
        cables.append(cable)
        strut_ends[(alpha_strut_label, alpha_end_letter)].append(
            Disc(slot=alpha_slot, angle=alpha_angle, cable_label=cable.label))
        strut_ends[(omega_strut_label, omega_end_letter)].append(
            Disc(slot=omega_slot, angle=omega_angle, cable_label=cable.label))

    for key in strut_ends:
        strut_ends[key].sort(key=lambda d: d.slot)

    # Sort struts by (leg, pos) so SA01..SA13, SB01..SB13, SC01..SC13, SH01..SH07
    # appear contiguous in §5 strut-end pages and §6 cross-reference.
    struts.sort(key=lambda s: (s.leg, s.pos))

    return ManualData(
        fabric_name=fabric_name,
        phase=phase,
        height_mm=float(height_mm_str),
        created=created,
        csv_path=path,
        hinge=hinge,
        magnitudes=magnitudes,
        magnitudes_locked=locked,
        bend_counts_signed=bend_signed,
        bend_counts_per_magnitude=bend_counts_per_magnitude,
        struts=struts,
        cables=cables,
        strut_ends=dict(strut_ends),
    )


# --------------------------------------------------------------- Output helpers

def fmt_signed(n: int) -> str:
    return f"{n:+d}" if n != 0 else "0"


def fmt_mm(value_mm: float, decimals: int = 1) -> str:
    return f"{value_mm:.{decimals}f} mm"


def fmt_xyz_mm(xyz: tuple) -> str:
    return f"({xyz[0]:.1f}, {xyz[1]:.1f}, {xyz[2]:.1f}) mm"


def length_bucket(length_mm: float) -> int:
    """Round to nearest 10 mm for the strut-tube and cable-length histograms."""
    return int(round(length_mm / 10.0) * 10)


def disc_count_for_end(data: ManualData, strut_label: str, end_letter: str) -> int:
    return len(data.strut_ends.get((strut_label, end_letter), []))


# --------------------------------------------------------------- Section writers

def write_cover(data: ManualData) -> str:
    lines = [
        f"# {data.fabric_name} — Construction Manual",
        "",
        f"- Snapshot phase: `{data.phase}`",
        f"- CSV created: {data.created}",
        f"- Source CSV: `{data.csv_path.name}`",
        "",
        f"**Top-line counts** (derived from the CSV):",
        "",
        f"- Struts: {len(data.struts)}",
        f"- Strut ends: {2 * len(data.struts)}",
        f"- Cables: {len(data.cables)}",
        f"- Disc positions: {2 * len(data.cables)}",
        "",
        "Each cable end occupies one disc; each strut end carries 3 or 4 discs "
        "depending on how many cables terminate there.",
    ]
    return "\n".join(lines)


def write_hinge_reference(data: ManualData) -> str:
    h = data.hinge
    lines = [
        "## §2 Hinge mechanism reference",
        "",
        "Per-disc geometry, all in millimetres. Dutch labels reflect the engineer's drawing.",
        "",
        "| Symbol | Name (NL) | Value | Description |",
        "|---|---|---:|---|",
        f"| A  | radius van de buis | {h.push_radius:g} | strut tube outer radius |",
        f"| B  | marge              | {h.push_radius_margin:g} | gap between tube surface and disc centre |",
        f"| C  | offset door radius | {h.offset_through_radius:g} | = t1 / 2 |",
        f"| D  | randafstand        | {h.hinge_extension:g} | hinge-arm length past the disc edge |",
        f"| E  | diameter gat       | {h.hinge_hole_diameter:g} | cable-attachment hole diameter |",
        f"| t1 | dikte staal        | {h.disc_thickness:g} | disc thickness |",
        f"| t2 | dikte POM          | {h.disc_separator:g} | separator thickness |",
        f"|    | cap_thickness      | {h.cap_thickness:g} | strut end cap thickness |",
        f"|    | pull_radius        | {h.pull_radius:g} | cable nominal radius |",
        "",
        "**Derived values:**",
        "",
        f"- A + B + C = {h.push_radius + h.push_radius_margin + h.offset_through_radius:g} mm  (halve breedte schijf)",
        f"- C + D + E = {h.offset_through_radius + h.hinge_extension + h.hinge_hole_diameter:g} mm  (scharnier lengte)",
        f"- t1 + t2   = {h.disc_thickness + h.disc_separator:g} mm  (disc + separator step along strut axis)",
        f"- disc_center_offset(1) = {h.disc_center_offset_mm(1):g} mm  (axial offset to centre of slot-1 disc)",
    ]
    return "\n".join(lines)


def write_parts_list(data: ManualData) -> str:
    n_ends = 2 * len(data.struts)

    # Total separators: each strut end has (1 + disc_count) separators
    n_separators = sum(1 + disc_count_for_end(data, s.label, e)
                       for s in data.struts for e in ("A", "B"))

    # Strut tube length histogram (bucketed)
    strut_buckets = defaultdict(int)
    for s in data.struts:
        strut_buckets[length_bucket(s.length_mm)] += 1
    strut_hist_rows = sorted(strut_buckets.items())

    # Cable length histogram (bucketed)
    cable_buckets = defaultdict(int)
    for c in data.cables:
        cable_buckets[length_bucket(c.length_mm)] += 1
    cable_hist_rows = sorted(cable_buckets.items())

    lines = [
        "## §3 Parts list",
        "",
        "### Connector hardware (per strut end, × {n})".format(n=n_ends),
        "",
        "| Part | Material | Count | Notes |",
        "|---|---|---:|---|",
        f"| Cap | S235 steel, 5 mm | {n_ends} | end-cap closing strut tube |",
        f"| POM separator (t2) | POM plastic, 1 mm | {n_separators} | one before first disc, one between adjacent discs |",
        f"| Bolt M20 × 100 | grade 8.8 | {n_ends} | through-bolt holding the stack |",
        f"| Fender washer M20 | steel | {n_ends} | under the lock nut |",
        f"| M20 lock nut | steel | {n_ends} | top of stack |",
        "",
        "### Discs by manufactured bend magnitude",
        "",
        "Each magnitude is one distinct part. The signed `+`/`−` of any disc in the "
        "assembly pages indicates **plate orientation at install time** "
        "(flippable through 180° about a radial axis), not a different part.",
        "",
        f"Locked inventory: **{data.magnitudes}**" if data.magnitudes_locked else
        f"Magnitudes (k-center optimised this run): {data.magnitudes}",
        "",
        "| Magnitude | Material | Count |",
        "|---:|---|---:|",
    ]
    for m in data.magnitudes:
        count = data.bend_counts_per_magnitude.get(m, 0)
        lines.append(f"| {m}° | S235 steel, t1 mm | {count} |")
    total_discs = sum(data.bend_counts_per_magnitude.values())
    lines.append(f"| **Total** |  | **{total_discs}** |")

    lines += [
        "",
        f"Signed distribution (echoed from CSV header):",
        f"`{data.bend_counts_signed}`",
        "",
        "### Strut tubes (cut lengths)",
        "",
        "Lengths bucketed to nearest 10 mm. The CSV's `Length(m)` for push rows "
        "is the **snapped rest length** — what the tube should be manufactured to.",
        "",
        "| Length | Count |",
        "|---:|---:|",
    ]
    for length, count in strut_hist_rows:
        lines.append(f"| {length} mm | {count} |")

    lines += [
        "",
        "### Cables (6 mm steel/RVS, factory-supplied with fork terminations at both ends)",
        "",
        "Lengths bucketed to nearest 10 mm. The CSV's `Length(m)` for pull rows "
        "is the **hinge-endpoint to hinge-endpoint** distance — not the final "
        "cut length. The factory adds the termination allowances (see "
        "`docs/csv-handoff.md`).",
        "",
        "| Length (hinge-to-hinge) | Count |",
        "|---:|---:|",
    ]
    for length, count in cable_hist_rows:
        lines.append(f"| {length} mm | {count} |")
    return "\n".join(lines)


def write_cable_inventory(data: ManualData) -> str:
    by_category = sorted(data.cables, key=lambda c: (c.category, c.pos))
    lines = [
        "## §4 Cable inventory",
        "",
        "Cables are grouped by where they sit in the structure: intra-leg cables "
        "(`CA`, `CB`, `CC`), hub-to-leg cables (`CHA`, `CHB`, `CHC`), and intra-hub "
        "cables (`CH`). Equivalents across the three legs share a position number — "
        "e.g. `CA01`, `CB01`, `CC01` play the same structural role on their respective legs.",
        "",
        "Label each physical cable with its label during fabrication.",
        "",
        "| Cable | Length (mm) | End A (slot, angle) | End B (slot, angle) |",
        "|---|---:|---|---|",
    ]
    for c in by_category:
        lines.append(
            f"| {c.label} | {c.length_mm:.0f} "
            f"| {c.alpha_strut_end} slot {c.alpha_slot}, {fmt_signed(c.alpha_angle)}° "
            f"| {c.omega_strut_end} slot {c.omega_slot}, {fmt_signed(c.omega_angle)}° |"
        )
    return "\n".join(lines)


def write_strut_end_pages(data: ManualData) -> str:
    lines = [
        "## §5 Strut-end assembly pages",
        "",
        "One section per strut end. Stack the listed parts on the M20 bolt from the cap "
        "outward. Discs are identified by magnitude; `+m` / `−m` are the same physical "
        "part in opposite orientations (the plate is flippable about a radial axis). "
        "Discs rotate freely on the bolt during assembly — no radial alignment is required.",
        "",
        "Cable references like `C047` correspond to the cable inventory in §4.",
    ]

    h = data.hinge
    for s in data.struts:
        for end_label, end_joint, end_xyz in (
            ("A", s.alpha_joint, s.alpha_xyz),
            ("B", s.omega_joint, s.omega_xyz),
        ):
            discs = data.strut_ends.get((s.label, end_label), [])
            tag = f"{s.label}{end_label}"
            lines += [
                "",
                f"### {tag} — joint `{end_joint}`",
                "",
                f"- Strut tube length (cut to): **{s.length_mm:.0f} mm**",
                f"- End coordinate (CSV Z-up): {fmt_xyz_mm(end_xyz)}",
                f"- Discs on this end: {len(discs)}",
                "",
                "```",
                "Stack on the M20 bolt, cap end first:",
                "",
                "    [ cap ]                              axial offset from strut end:",
                "    [ separator ]",
            ]
            for d in discs:
                offset = h.disc_center_offset_mm(d.slot)
                lines.append(
                    f"    [ DISC {fmt_signed(d.angle)}° ]      "
                    f"slot {d.slot}  →  {d.cable_label:<6}                  {offset:.1f} mm"
                )
                lines.append("    [ separator ]")
            lines += [
                "    [ fender washer + M20 lock nut ]",
                "```",
            ]
    return "\n".join(lines)


def write_cross_reference(data: ManualData) -> str:
    lines = [
        "## §6 Cross-reference index",
        "",
        "### Strut numbers ↔ CSV joint paths",
        "",
        "| Strut | Alpha joint (end A) | Omega joint (end B) | Cut length |",
        "|---|---|---|---:|",
    ]
    for s in data.struts:
        lines.append(f"| {s.label} | `{s.alpha_joint}` | `{s.omega_joint}` | {s.length_mm:.0f} mm |")
    lines += [
        "",
        "### Cable labels ↔ CSV joint paths",
        "",
        "| Cable | Alpha joint | Omega joint |",
        "|---|---|---|",
    ]
    by_category = sorted(data.cables, key=lambda c: (c.category, c.pos))
    for c in by_category:
        lines.append(f"| {c.label} | `{c.alpha_joint}` | `{c.omega_joint}` |")
    return "\n".join(lines)


def write_static_intro() -> str:
    return "\n".join([
        "## §1 Conventions and notes for the assembly team",
        "",
        "**Materials.** Discs are S235 steel at thickness t1. Separators are POM "
        "(polyoxymethylene) at thickness t2. Caps are S235 steel at thickness "
        "`cap_thickness`. Bolts are M20 × 100 grade 8.8 with fender washers and "
        "M20 lock nuts. Strut tubes are aluminium (grade per fabrication spec). "
        "Cables are 6 mm steel (RVS) with two fork terminations supplied by the "
        "factory.",
        "",
        "**Disc angle sign convention.** A disc magnitude (e.g. 30°) is the bend "
        "from a flat plate. `0°` = flat, `90°` = right-angle. The sign in the "
        "assembly stack (`+30°` vs `−30°`) is plate orientation, not a different "
        "part: each disc is flippable through 180° about a radial axis through "
        "the bolt hole, which reverses the sign of its bend in the assembly.",
        "",
        "  - `+α`: bend tilts outward off the cap (cable continues away from "
        "the strut tube).",
        "  - `−α`: bend tilts back along the strut tube (cable continues parallel "
        "to the strut body).",
        "",
        "**Cable labels.** Each cable carries a label from §4 indicating its "
        "category and position — `CA`/`CB`/`CC` for intra-leg cables, "
        "`CHA`/`CHB`/`CHC` for hub-to-leg, `CH` for intra-hub. The §5 strut-end "
        "pages reference cables by these labels; verify the label matches "
        "before attaching each end.",
        "",
        "**Assembly order on each bolt.** Cap end first. After the cap a "
        "separator, then alternating disc–separator–disc–separator. Finally a "
        "fender washer and M20 lock nut at the open end of the bolt. Discs "
        "rotate freely on the bolt; no radial alignment is required at install — "
        "the cable's pull naturally rotates each disc to its preferred angle "
        "around the strut axis.",
    ])


# --------------------------------------------------------------- Main

def write_manual(data: ManualData, out_path: Path) -> None:
    sections = [
        write_cover(data),
        write_static_intro(),
        write_hinge_reference(data),
        write_parts_list(data),
        write_cable_inventory(data),
        write_strut_end_pages(data),
        write_cross_reference(data),
    ]
    out_path.write_text("\n\n".join(sections) + "\n", encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("csv", type=Path, help="Path to an OpenClaw-*.csv export")
    parser.add_argument("--out", type=Path, default=None,
                        help="Output path (default: <csv-basename>-manual.md alongside the input)")
    args = parser.parse_args()

    if not args.csv.is_file():
        sys.exit(f"CSV not found: {args.csv}")

    data = parse_csv(args.csv)

    # --- Consistency checks ---
    expected_disc_total = 2 * len(data.cables)
    actual_disc_total = sum(len(discs) for discs in data.strut_ends.values())
    if expected_disc_total != actual_disc_total:
        sys.exit(f"Disc count mismatch: 2×cables={expected_disc_total}, "
                 f"strut-end stacks total={actual_disc_total}")
    histogram_total = sum(data.bend_counts_per_magnitude.values())
    if histogram_total != expected_disc_total:
        # CSV header histogram should match the disc total; warn but don't fail.
        print(f"warning: CSV bend-count histogram total ({histogram_total}) "
              f"differs from disc total ({expected_disc_total})", file=sys.stderr)

    out = args.out or args.csv.with_name(args.csv.stem + "-manual.md")
    write_manual(data, out)
    print(f"Wrote {out}")
    print(f"  {len(data.struts)} struts, {len(data.cables)} cables, "
          f"{actual_disc_total} discs, "
          f"{len(data.strut_ends)} strut ends with cables")


if __name__ == "__main__":
    main()
