#!/usr/bin/env python3
"""Build the per-strut assembly pages (46 pages, one per strut) as a single HTML
file ready for weasyprint.

Usage:
    python3 build_strut_pages.py source.csv
    weasyprint OpenClaw-2026-05-16-strut-pages.html OpenClaw-2026-05-16-strut-pages.pdf

Adds, on top of CSV ingestion:
  - Twist-number derivation from joint naming (e.g. AX3Z4 → leg A, twist 3)
  - Categorisation: twist / vertical bottom / hub / apex
  - "Up" / "down" end labels (compare alpha_z vs omega_z)
  - One A4 page per strut, page-break-after each

For joint-naming details see docs/joint-naming.md.
"""

from __future__ import annotations

import argparse
import csv
import io
import math
import re
import shutil
import subprocess
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path


# ── Data model (lifted from the retired build_manual.py) ─────────────────────


@dataclass
class ConnectorDimensions:
    """Connector geometry parsed from the CSV header (all values in millimetres)."""

    push_radius: float           # A
    push_radius_margin: float    # B
    offset_through_radius: float # C (= t1/2)
    tab_extension: float       # D
    tab_hole_diameter: float   # E
    disc_thickness: float        # t1
    disc_separator: float        # t2
    cap_thickness: float
    pull_radius: float

    def disc_center_offset_mm(self, slot: int) -> float:
        """Axial offset from strut end to centre of disc at `slot` (1-indexed)."""
        step = self.disc_thickness + self.disc_separator
        return (
            self.cap_thickness
            + self.disc_separator
            + self.disc_thickness / 2.0
            + step * (slot - 1)
        )


@dataclass
class Strut:
    """One push interval. Identified externally by its (alpha_joint, omega_joint)
    pair — there is no separate strut label. `leg` and `pos` exist only to drive
    page ordering and twist-mate grouping."""

    leg: str                # 'A', 'B', 'C', or 'H' (hub)
    pos: int                # 1-indexed position within its leg
    alpha_joint: str
    omega_joint: str
    length_mm: float
    alpha_xyz: tuple
    omega_xyz: tuple

    @property
    def axis_outward_from_alpha(self) -> tuple:
        dx = self.omega_xyz[0] - self.alpha_xyz[0]
        dy = self.omega_xyz[1] - self.alpha_xyz[1]
        dz = self.omega_xyz[2] - self.alpha_xyz[2]
        length = (dx * dx + dy * dy + dz * dz) ** 0.5 or 1.0
        return (-dx / length, -dy / length, -dz / length)


@dataclass
class Disc:
    """One disc in a strut-end stack: slot, signed bend angle, and the (joint,
    slot) of the cable's other end. The cable itself has no separate identifier
    — it's just the pair of discs."""

    slot: int                  # 1-indexed
    angle: int                 # signed, snapped to the locked magnitudes
    other_joint: str           # cable's other-end joint
    other_slot: int            # cable's other-end slot (1-indexed)
    other_angle: int           # cable's other-end bend angle (signed)


@dataclass
class Cable:
    length_mm: float
    alpha_joint: str
    omega_joint: str
    alpha_slot: int
    omega_slot: int
    alpha_angle: int
    omega_angle: int


@dataclass
class ManualData:
    fabric_name: str
    phase: str
    height_mm: float
    created: str
    csv_path: Path
    connector: ConnectorDimensions
    magnitudes: list
    magnitudes_locked: bool
    bend_counts_signed: str
    bend_counts_per_magnitude: dict
    struts: list
    cables: list
    strut_ends: dict


# ── CSV parsing ──────────────────────────────────────────────────────────────


_HEADER_TITLE = re.compile(r"^# (.*?), Phase:\s*(\S+),\s*Height:\s*([\d.]+)mm,\s*Created:\s*(.+)$")
_CONN_PARAM = re.compile(r"^#\s+(?:[A-E]\s+)?\S.*?\(([a-z_]+)\):\s*([\d.]+)mm")
_CONN_PARAM_PLAIN = re.compile(r"^#\s+([a-z_]+):\s*([\d.]+)mm")
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

    title = next((m for m in (_HEADER_TITLE.match(l) for l in header_lines) if m), None)
    if title is None:
        raise SystemExit("CSV missing title line")
    fabric_name, phase, height_mm_str, created = title.groups()

    connector_values: dict[str, float] = {}
    for line in header_lines:
        m = _CONN_PARAM.match(line) or _CONN_PARAM_PLAIN.match(line)
        if m:
            connector_values[m.group(1)] = float(m.group(2))

    needed = [
        "push_radius", "push_radius_margin", "tab_extension",
        "tab_hole_diameter", "disc_thickness", "disc_separator",
        "cap_thickness", "pull_radius",
    ]
    missing = [k for k in needed if k not in connector_values]
    if missing:
        raise SystemExit(f"CSV header missing connector parameter(s): {missing}")

    connector = ConnectorDimensions(
        push_radius=connector_values["push_radius"],
        push_radius_margin=connector_values["push_radius_margin"],
        offset_through_radius=connector_values["disc_thickness"] / 2.0,
        tab_extension=connector_values["tab_extension"],
        tab_hole_diameter=connector_values["tab_hole_diameter"],
        disc_thickness=connector_values["disc_thickness"],
        disc_separator=connector_values["disc_separator"],
        cap_thickness=connector_values["cap_thickness"],
        pull_radius=connector_values["pull_radius"],
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

    # Classify a joint by its label's first character (see docs/joint-naming.md):
    #   path joints AX..., BX..., CX...  → leg 'A', 'B', 'C'
    #   seed joints BAA, BOB, TAC, ...   → hub 'H' (start with B or T but only
    #                                             two letters long before leg)
    #   apex prism YZ0, YZ1              → hub 'H'
    def classify(joint: str) -> str:
        if len(joint) >= 2 and joint[0] in ("A", "B", "C") and joint[1] == "X":
            return joint[0]
        return "H"

    leg_counters: dict[str, int] = {"A": 0, "B": 0, "C": 0, "H": 0}
    struts: list[Strut] = []
    joint_to_strut: dict[str, "Strut"] = {}
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
                f"alpha {alpha_joint!r} ({leg_alpha}) vs omega {omega_joint!r} ({leg_omega})."
            )
        leg = leg_alpha
        leg_counters[leg] += 1
        pos = leg_counters[leg]
        length_mm = float(row[2]) * 1000.0
        s = Strut(
            leg=leg, pos=pos,
            alpha_joint=alpha_joint, omega_joint=omega_joint,
            length_mm=length_mm,
            alpha_xyz=(ax, ay, az), omega_xyz=(ox, oy, oz),
        )
        struts.append(s)
        if alpha_joint in joint_to_strut or omega_joint in joint_to_strut:
            raise SystemExit(f"Joint sharing detected at CSV push row {row_idx}.")
        joint_to_strut[alpha_joint] = s
        joint_to_strut[omega_joint] = s

    cables: list[Cable] = []
    strut_ends: dict[str, list[Disc]] = defaultdict(list)
    for i, row in enumerate(pulls_raw, start=1):
        length_mm = float(row[2]) * 1000.0
        alpha_joint = row[7]
        alpha_slot = int(row[8])
        alpha_angle = int(row[9])
        omega_joint = row[13]
        omega_slot = int(row[14])
        omega_angle = int(row[15])
        if alpha_joint not in joint_to_strut:
            raise SystemExit(f"Pull row {i}: alpha {alpha_joint!r} not found in any strut")
        if omega_joint not in joint_to_strut:
            raise SystemExit(f"Pull row {i}: omega {omega_joint!r} not found in any strut")
        cables.append(Cable(
            length_mm=length_mm,
            alpha_joint=alpha_joint, omega_joint=omega_joint,
            alpha_slot=alpha_slot, omega_slot=omega_slot,
            alpha_angle=alpha_angle, omega_angle=omega_angle,
        ))
        # Each disc points directly at the cable's other end.
        strut_ends[alpha_joint].append(Disc(
            slot=alpha_slot, angle=alpha_angle,
            other_joint=omega_joint, other_slot=omega_slot, other_angle=omega_angle,
        ))
        strut_ends[omega_joint].append(Disc(
            slot=omega_slot, angle=omega_angle,
            other_joint=alpha_joint, other_slot=alpha_slot, other_angle=alpha_angle,
        ))

    for joint in strut_ends:
        strut_ends[joint].sort(key=lambda d: d.slot)

    struts.sort(key=lambda s: (s.leg, s.pos))

    return ManualData(
        fabric_name=fabric_name, phase=phase,
        height_mm=float(height_mm_str), created=created,
        csv_path=path, connector=connector,
        magnitudes=magnitudes, magnitudes_locked=locked,
        bend_counts_signed=bend_signed,
        bend_counts_per_magnitude=bend_counts_per_magnitude,
        struts=struts, cables=cables, strut_ends=dict(strut_ends),
    )


# ── Vector helpers (3-tuples) ────────────────────────────────────────────────
def v_sub(a, b):  return (a[0]-b[0], a[1]-b[1], a[2]-b[2])
def v_dot(a, b):  return a[0]*b[0] + a[1]*b[1] + a[2]*b[2]
def v_cross(a, b): return (a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0])
def v_len(v):     return math.sqrt(v[0]**2 + v[1]**2 + v[2]**2)
def v_scale(v, s): return (v[0]*s, v[1]*s, v[2]*s)
def v_norm(v):
    l = v_len(v)
    return (v[0]/l, v[1]/l, v[2]/l) if l > 1e-9 else v


def build_joint_xyz_map(struts: list) -> dict:
    m = {}
    for s in struts:
        m[s.alpha_joint] = s.alpha_xyz
        m[s.omega_joint] = s.omega_xyz
    return m


def rotational_angle(strut: Strut, is_alpha: bool, disc: Disc,
                     joint_xyz: dict) -> int:
    """Rotational angle of this disc around the strut axis, in degrees [0, 360).

    Viewing convention: the clock is drawn as the disc is seen by an observer
    **outside the joint, looking back at it with the strut receding away into
    the distance**. This is the natural inspection pose (you're staring at the
    disc face, with the rest of the strut behind it). With world +Z as 'up':
      - 0° = top of clock (world +Z projected perpendicular to the strut axis)
      - 90° = right of clock (the observer's right when looking *back* along
              −axis at the joint — i.e. `up × axis`)
      - Increasing angle goes clockwise on screen.
    """
    if is_alpha:
        here = strut.alpha_xyz
        axis = v_norm(v_sub(strut.alpha_xyz, strut.omega_xyz))  # outward from this end
    else:
        here = strut.omega_xyz
        axis = v_norm(v_sub(strut.omega_xyz, strut.alpha_xyz))

    other_xyz = joint_xyz.get(disc.other_joint)
    if other_xyz is None:
        return 0
    cable_dir = v_norm(v_sub(other_xyz, here))
    # Project cable direction onto plane perpendicular to strut axis
    perp = v_sub(cable_dir, v_scale(axis, v_dot(cable_dir, axis)))
    perp_l = v_len(perp)
    if perp_l < 1e-6:
        return 0
    perp = v_scale(perp, 1.0 / perp_l)

    # Reference direction: world +Z projected onto same plane (fallback: world +X)
    up = (0.0, 0.0, 1.0)
    up_ref = v_sub(up, v_scale(axis, v_dot(up, axis)))
    if v_len(up_ref) < 1e-3:
        x = (1.0, 0.0, 0.0)
        up_ref = v_sub(x, v_scale(axis, v_dot(x, axis)))
    up_ref = v_norm(up_ref)
    # `right_ref` is the observer's right hand when looking back at the joint
    # along −axis. (Forward × up = right, with forward = −axis →
    # right = −axis × up = up × axis.)
    right_ref = v_cross(up_ref, axis)

    angle = math.degrees(math.atan2(v_dot(perp, right_ref), v_dot(perp, up_ref)))
    if angle < 0:
        angle += 360.0
    return int(round(angle)) % 360


# Joint-name patterns
_LEG_JOINT  = re.compile(r"^([ABC])X(\d+)(Y?)Z\d+$")    # e.g. AX3Z4 or AX4YZ0
_APEX_JOINT = re.compile(r"^YZ\d+$")                    # e.g. YZ0
_HUB_JOINT  = re.compile(r"^[BT][AO][ABC]$")            # e.g. BAA, BOC, TAA, TOC


def categorise_strut(s: Strut) -> dict:
    """Classify a strut by parsing its joint name.

    Returns a dict with:
      - category: 'twist' | 'vertical_bottom' | 'hub' | 'apex' | 'unknown'
      - twist:    int (1..4) for 'twist' category, else None
      - leg:      'A' | 'B' | 'C' | 'H'
    """
    m = _LEG_JOINT.match(s.alpha_joint)
    if m:
        leg, twist, has_y = m.group(1), int(m.group(2)), bool(m.group(3))
        if has_y:
            return {"category": "vertical_bottom", "twist": None, "leg": leg}
        return {"category": "twist", "twist": twist, "leg": leg}
    if _APEX_JOINT.match(s.alpha_joint):
        return {"category": "apex", "twist": None, "leg": "H"}
    if _HUB_JOINT.match(s.alpha_joint):
        return {"category": "hub", "twist": None, "leg": "H"}
    return {"category": "unknown", "twist": None, "leg": "?"}


def up_down_ends(s: Strut) -> tuple[str, str]:
    """Return (up_end, down_end) as 'A' or 'B' based on Z coordinate."""
    if s.alpha_xyz[2] >= s.omega_xyz[2]:
        return "A", "B"
    return "B", "A"


def fmt_signed(n: int) -> str:
    return f"{n:+d}°" if n != 0 else "0°"


def twist_mates(target: Strut, all_struts: list, cat: dict) -> list[str]:
    """The other two struts in the same (leg, twist) group.
    Returns a list of `"<alpha>↔<omega>"` descriptors (sorted)."""
    if cat["category"] != "twist":
        return []
    mates = []
    for s in all_struts:
        if s.alpha_joint == target.alpha_joint:
            continue
        other_cat = categorise_strut(s)
        if (other_cat["category"] == "twist"
            and other_cat["leg"] == cat["leg"]
            and other_cat["twist"] == cat["twist"]):
            mates.append(f"{s.alpha_joint}↔{s.omega_joint}")
    return sorted(mates)


def disc_target_label(disc: Disc) -> str:
    """Cable target shown next to a disc: '<other_joint> sN'."""
    return f"{disc.other_joint} s{disc.other_slot}"


CSS = """
@page {
  size: A4 portrait;
  margin: 15mm;
}
body {
  font-family: 'Helvetica Neue', 'Helvetica', 'Arial', sans-serif;
  font-size: 10pt;
  color: #1a1a1a;
  line-height: 1.4;
  margin: 0;
}
.strut-page {
  page-break-after: always;
}
.strut-page:last-child {
  page-break-after: auto;
}
header {
  border-bottom: 2px solid #1a1a1a;
  padding-bottom: 3mm;
  margin-bottom: 3mm;
}
h1 {
  margin: 0;
  font-size: 24pt;
  font-family: 'Menlo', 'Consolas', 'Courier New', monospace;
  letter-spacing: 0.03em;
}
.subtitle {
  margin-top: 3mm;
  margin-bottom: 8mm;
  font-size: 12pt;
  color: #333;
  font-family: 'Menlo', 'Consolas', 'Courier New', monospace;
}
.subtitle .twist {
  color: #1a1a1a;
  font-weight: 600;
}
.diagram {
  margin: 4mm 0 8mm 0;
}
.diagram svg {
  width: 100%;
  height: auto;
  display: block;
}
.endpoint-tables {
  display: flex;
  gap: 6mm;
}
.endpoint-table {
  flex: 1;
  border-collapse: collapse;
  font-size: 10pt;
}
.endpoint-table caption {
  font-weight: bold;
  font-size: 13pt;
  text-align: left;
  padding-bottom: 2mm;
  font-family: 'Menlo', 'Consolas', 'Courier New', monospace;
  color: #1a1a1a;
}
.endpoint-table th,
.endpoint-table td {
  border-bottom: 1px solid #ddd;
  padding: 2mm 2.5mm;
  text-align: left;
}
.endpoint-table th {
  background: #f4f4f4;
  font-weight: 600;
  font-size: 9pt;
  color: #555;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  border-bottom: 2px solid #1a1a1a;
}
.rotation-clocks {
  display: flex;
  gap: 6mm;
  margin-top: 8mm;
}
.rotation-clocks .clock {
  flex: 1;
  text-align: center;
}
.rotation-clocks .clock svg {
  width: 100%;
  max-width: 70mm;
  height: auto;
  display: block;
  margin: 0 auto;
}
.id { font-family: 'Menlo', 'Consolas', 'Courier New', monospace; }
.pos { color: #1d7a3e; font-weight: 600; }
.neg { color: #b32626; font-weight: 600; }
.ud-top    { color: #1d7a3e; font-weight: 700; font-size: 11pt; margin-left: 4mm; }
.ud-bottom { color: #b32626; font-weight: 700; font-size: 11pt; margin-left: 4mm; }
"""


def render_subtitle(s: Strut, cat: dict, mates: list[str]) -> str:
    """Build the subtitle line that appears under the title."""
    cat_name = cat["category"]
    if cat_name == "twist":
        twist_str = f"<span class='twist'>twist {cat['twist']}</span>"
        mate_str = f" (with {', '.join(mates)})" if mates else ""
        return f"Leg {cat['leg']} · {twist_str}{mate_str} · {s.length_mm:.0f} mm"
    if cat_name == "vertical_bottom":
        return f"Vertical bottom strut · Leg {cat['leg']} · {s.length_mm:.0f} mm"
    if cat_name == "apex":
        return f"Apex strut · {s.length_mm:.0f} mm"
    if cat_name == "hub":
        return f"Hub strut · {s.length_mm:.0f} mm"
    return f"Strut · {s.length_mm:.0f} mm"


def render_disc_label(disc: Disc, side: str, rot: int) -> str:
    """Build the disc fan-line label.

    Format: 'slot 1 ↻ 045° · +30° → AX1Z3 s2'
    where ↻ NNN° is the rotational angle around the strut axis.
    """
    arrow = "→" if side == "omega" else "←"
    return (
        f"slot {disc.slot} ↻ {rot:03d}° · "
        f"{fmt_signed(disc.angle)} {arrow} {disc.other_joint} s{disc.other_slot}"
    )


def fan_y_positions(n: int, top: int = 60, step: int = 90) -> list[int]:
    """Vertical positions for n stacked fan labels at one strut end."""
    return [top + i * step for i in range(n)]


def render_strut_diagram(s: Strut, data: ManualData, up_is_alpha: bool, joint_xyz: dict) -> str:
    """Render the SVG strut diagram for one page."""
    alpha_discs = data.strut_ends.get(s.alpha_joint, [])
    omega_discs = data.strut_ends.get(s.omega_joint, [])

    alpha_rots = [rotational_angle(s, True,  d, joint_xyz) for d in alpha_discs]
    omega_rots = [rotational_angle(s, False, d, joint_xyz) for d in omega_discs]

    alpha_ys = fan_y_positions(len(alpha_discs))
    omega_ys = fan_y_positions(len(omega_discs))

    alpha_up = up_is_alpha
    omega_up = not up_is_alpha
    alpha_ud = "↑ TOP" if alpha_up else "↓ BOTTOM"
    omega_ud = "↑ TOP" if omega_up else "↓ BOTTOM"
    alpha_color = "#1d7a3e" if alpha_up else "#b32626"
    omega_color = "#1d7a3e" if omega_up else "#b32626"

    # Wider viewBox so the long fan labels fit on both sides without clipping.
    parts = []
    parts.append(
        '<svg viewBox="-260 0 1520 480" xmlns="http://www.w3.org/2000/svg">'
    )
    # Strut body + caps
    parts.append('<rect x="260" y="220" width="480" height="40" fill="#e8e8e8" stroke="#1a1a1a" stroke-width="1.5"/>')
    parts.append('<rect x="252" y="216" width="12" height="48" fill="#888" stroke="#1a1a1a" stroke-width="1.5"/>')
    parts.append('<rect x="736" y="216" width="12" height="48" fill="#888" stroke="#1a1a1a" stroke-width="1.5"/>')
    # Joint labels (alpha on left, omega on right) — actual joint names.
    parts.append(f'<text x="258" y="295" font-family="Menlo,monospace" font-size="18" font-weight="bold" text-anchor="middle">{s.alpha_joint}</text>')
    parts.append(f'<text x="258" y="318" font-family="Menlo,monospace" font-size="14" font-weight="bold" text-anchor="middle" fill="{alpha_color}">{alpha_ud}</text>')
    parts.append(f'<text x="742" y="295" font-family="Menlo,monospace" font-size="18" font-weight="bold" text-anchor="middle">{s.omega_joint}</text>')
    parts.append(f'<text x="742" y="318" font-family="Menlo,monospace" font-size="14" font-weight="bold" text-anchor="middle" fill="{omega_color}">{omega_ud}</text>')
    # Length label
    parts.append(f'<text x="500" y="246" font-family="Menlo,monospace" font-size="13" text-anchor="middle" fill="#555">{s.length_mm:.0f} mm</text>')

    # Alpha-end fan lines (left)
    for disc, y, rot in zip(alpha_discs, alpha_ys, alpha_rots):
        color = "#1d7a3e" if disc.angle > 0 else ("#b32626" if disc.angle < 0 else "#555")
        label = render_disc_label(disc, "alpha", rot)
        parts.append(f'<line x1="258" y1="240" x2="60" y2="{y}" stroke="{color}" stroke-width="1.5"/>')
        parts.append(f'<text x="55" y="{y - 5}" font-size="10" text-anchor="end" fill="{color}" font-family="Menlo,monospace">{label}</text>')

    # Omega-end fan lines (right)
    for disc, y, rot in zip(omega_discs, omega_ys, omega_rots):
        color = "#1d7a3e" if disc.angle > 0 else ("#b32626" if disc.angle < 0 else "#555")
        label = render_disc_label(disc, "omega", rot)
        parts.append(f'<line x1="742" y1="240" x2="940" y2="{y}" stroke="{color}" stroke-width="1.5"/>')
        parts.append(f'<text x="945" y="{y - 5}" font-size="10" fill="{color}" font-family="Menlo,monospace">{label}</text>')

    parts.append('</svg>')
    return "\n".join(parts)


def render_endpoint_table(s: Strut, is_alpha: bool, data: ManualData,
                          is_top: bool, joint_xyz: dict) -> str:
    joint = s.alpha_joint if is_alpha else s.omega_joint
    discs = data.strut_ends.get(joint, [])
    rows_data = []
    for d in discs:
        rot = rotational_angle(s, is_alpha, d, joint_xyz)
        rows_data.append((d, rot))
    rows_data.sort(key=lambda x: x[0].slot)

    rows = []
    for d, rot in rows_data:
        cls = "pos" if d.angle > 0 else ("neg" if d.angle < 0 else "")
        rows.append(
            f'<tr><td>{d.slot}</td>'
            f'<td class="{cls}">{fmt_signed(d.angle)}</td>'
            f'<td class="id">{rot:03d}°</td>'
            f'<td class="id">{disc_target_label(d)}</td></tr>'
        )
    rows_html = "\n".join(rows)
    ud_text = "↑ TOP" if is_top else "↓ BOTTOM"
    ud_class = "ud-top" if is_top else "ud-bottom"
    return (
        f'<table class="endpoint-table">'
        f'<caption>Joint {joint} <span class="{ud_class}">{ud_text}</span></caption>'
        f'<thead><tr><th>Slot</th><th>Bend</th><th>Rot</th><th>Connects to</th></tr></thead>'
        f'<tbody>{rows_html}</tbody>'
        f'</table>'
    )


def render_rotation_clock(s: Strut, is_alpha: bool, data: ManualData,
                          joint_xyz: dict, is_top: bool) -> str:
    """Clock-face SVG showing rotational positions of each disc around the strut axis.

    Convention:
      - 0° at top ('up relative to strut' = world +Z projected onto the joint plane)
      - 90° at right, 180° at bottom, 270° at left (clockwise as viewed from outside the strut)
    """
    joint = s.alpha_joint if is_alpha else s.omega_joint
    discs = data.strut_ends.get(joint, [])
    rotations = [
        (d, rotational_angle(s, is_alpha, d, joint_xyz))
        for d in discs
    ]

    cx, cy, r = 110, 130, 75
    title_color = "#1d7a3e" if is_top else "#b32626"
    title_label = "↑ TOP" if is_top else "↓ BOTTOM"

    parts = ['<svg viewBox="0 0 220 270" xmlns="http://www.w3.org/2000/svg">']
    parts.append(
        f'<text x="110" y="16" font-family="Menlo,monospace" font-size="12" '
        f'font-weight="bold" text-anchor="middle">Joint {joint}</text>'
    )
    parts.append(
        f'<text x="110" y="32" font-family="Menlo,monospace" font-size="10" '
        f'font-weight="bold" text-anchor="middle" fill="{title_color}">{title_label}</text>'
    )

    # Outer circle
    parts.append(f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="none" stroke="#bbb" stroke-width="1"/>')

    # Four cardinal tick marks at 0/90/180/270
    for ang in (0, 90, 180, 270):
        a = math.radians(ang)
        x_outer = cx + (r + 4) * math.sin(a)
        y_outer = cy - (r + 4) * math.cos(a)
        x_inner = cx + (r - 4) * math.sin(a)
        y_inner = cy - (r - 4) * math.cos(a)
        parts.append(
            f'<line x1="{x_inner:.1f}" y1="{y_inner:.1f}" '
            f'x2="{x_outer:.1f}" y2="{y_outer:.1f}" stroke="#999" stroke-width="1"/>'
        )
    # Cardinal labels
    parts.append(f'<text x="{cx}" y="{cy - r - 10}" font-size="9" text-anchor="middle" fill="#888" font-family="Menlo,monospace">↑ 0°</text>')
    parts.append(f'<text x="{cx + r + 12}" y="{cy + 3}" font-size="9" text-anchor="start" fill="#888" font-family="Menlo,monospace">90°</text>')
    parts.append(f'<text x="{cx}" y="{cy + r + 14}" font-size="9" text-anchor="middle" fill="#888" font-family="Menlo,monospace">180°</text>')
    parts.append(f'<text x="{cx - r - 12}" y="{cy + 3}" font-size="9" text-anchor="end" fill="#888" font-family="Menlo,monospace">270°</text>')

    # Center dot (the strut axis emerging from the joint)
    parts.append(f'<circle cx="{cx}" cy="{cy}" r="3" fill="#333"/>')

    # Disc markers
    for d, rot in sorted(rotations, key=lambda x: x[1]):
        a = math.radians(rot)
        x = cx + r * math.sin(a)
        y = cy - r * math.cos(a)
        color = "#1d7a3e" if d.angle > 0 else ("#b32626" if d.angle < 0 else "#555")
        # Line from centre to marker
        parts.append(f'<line x1="{cx}" y1="{cy}" x2="{x:.1f}" y2="{y:.1f}" stroke="{color}" stroke-width="1.5" opacity="0.7"/>')
        # Marker circle with slot number
        parts.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="11" fill="white" stroke="{color}" stroke-width="2"/>')
        parts.append(
            f'<text x="{x:.1f}" y="{y + 4:.1f}" font-size="12" font-weight="bold" '
            f'text-anchor="middle" fill="{color}" font-family="Menlo,monospace">{d.slot}</text>'
        )
        # Small rotation-angle label outside the marker
        x_lbl = cx + (r + 14) * math.sin(a)
        y_lbl = cy - (r + 14) * math.cos(a)
        # Skip if it would collide with cardinal labels (near 0/90/180/270)
        near_cardinal = any(abs(((rot - c) % 360 + 180) % 360 - 180) < 12 for c in (0, 90, 180, 270))
        if not near_cardinal:
            parts.append(
                f'<text x="{x_lbl:.1f}" y="{y_lbl + 3:.1f}" font-size="8" '
                f'text-anchor="middle" fill="#666" font-family="Menlo,monospace">{rot:03d}°</text>'
            )
        else:
            # Place rotation label inside the marker as a tiny suffix; here just omit to avoid clutter.
            pass

    parts.append('</svg>')
    return "\n".join(parts)


def render_strut_page(s: Strut, data: ManualData, all_struts: list, joint_xyz: dict) -> str:
    cat = categorise_strut(s)
    up_end, _ = up_down_ends(s)         # 'A' or 'B' — alpha-end up or omega-end up
    up_is_alpha = (up_end == "A")
    mates = twist_mates(s, all_struts, cat)

    subtitle = render_subtitle(s, cat, mates)
    diagram = render_strut_diagram(s, data, up_is_alpha, joint_xyz)
    table_a = render_endpoint_table(s, True,  data, is_top=up_is_alpha, joint_xyz=joint_xyz)
    table_b = render_endpoint_table(s, False, data, is_top=not up_is_alpha, joint_xyz=joint_xyz)
    clock_a = render_rotation_clock(s, True,  data, joint_xyz, is_top=up_is_alpha)
    clock_b = render_rotation_clock(s, False, data, joint_xyz, is_top=not up_is_alpha)

    title = f"{s.alpha_joint} ↔ {s.omega_joint}"
    return (
        f'<div class="strut-page">'
        f'<header>'
        f'<h1>{title}</h1>'
        f'</header>'
        f'<div class="subtitle">{subtitle}</div>'
        f'<div class="diagram">{diagram}</div>'
        f'<div class="endpoint-tables">{table_a}{table_b}</div>'
        f'<div class="rotation-clocks">'
        f'<div class="clock">{clock_a}</div>'
        f'<div class="clock">{clock_b}</div>'
        f'</div>'
        f'</div>'
    )


def render_html(data: ManualData) -> str:
    joint_xyz = build_joint_xyz_map(data.struts)
    pages = [render_strut_page(s, data, data.struts, joint_xyz) for s in data.struts]
    return (
        '<!DOCTYPE html>\n'
        '<html lang="en">\n'
        '<head>\n'
        '<meta charset="utf-8">\n'
        f'<title>{data.fabric_name} — Strut Pages</title>\n'
        f'<style>{CSS}</style>\n'
        '</head>\n'
        '<body>\n' + "\n".join(pages) + '\n</body>\n</html>\n'
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("csv", type=Path, help="Path to an OpenClaw-*.csv export")
    parser.add_argument("--out", type=Path, default=None,
                        help="Output PDF path (default: <csv-basename>-strut-pages.pdf alongside the input)")
    parser.add_argument("--keep-html", action="store_true",
                        help="Keep the intermediate HTML alongside the PDF (default: delete it)")
    args = parser.parse_args()

    if not args.csv.is_file():
        sys.exit(f"CSV not found: {args.csv}")

    data = parse_csv(args.csv)
    html = render_html(data)

    pdf_path = args.out or args.csv.with_name(args.csv.stem + "-strut-pages.pdf")
    html_path = pdf_path.with_suffix(".html")
    html_path.write_text(html, encoding="utf-8")

    weasyprint_bin = shutil.which("weasyprint")
    if not weasyprint_bin:
        sys.exit("weasyprint CLI not found. Install with: brew install weasyprint")

    result = subprocess.run(
        [weasyprint_bin, str(html_path), str(pdf_path)],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        sys.stderr.write(result.stderr)
        sys.exit(f"weasyprint failed (exit {result.returncode})")

    if not args.keep_html:
        html_path.unlink()

    print(f"Wrote {pdf_path}")
    print(f"  {len(data.struts)} struts → {len(data.struts)} pages")


if __name__ == "__main__":
    main()
