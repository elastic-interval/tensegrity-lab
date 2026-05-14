#!/usr/bin/env python3
"""Build the per-strut assembly pages (46 pages, one per strut) as a single HTML
file ready for weasyprint.

Usage:
    python3 build_strut_pages.py source.csv
    weasyprint OpenClaw-slack-2026-05-13-strut-pages.html OpenClaw-slack-2026-05-13-strut-pages.pdf

Reuses build_manual.parse_csv for CSV ingestion. Adds:
  - Twist-number derivation from joint naming (e.g. AX3Z4 → leg A, twist 3)
  - Categorisation: twist / vertical bottom / hub / apex
  - "Up" / "down" end labels (compare alpha_z vs omega_z)
  - One A4 page per strut, page-break-after each
"""

from __future__ import annotations

import argparse
import math
import re
import shutil
import subprocess
import sys
from pathlib import Path

# Reuse the existing parser/data model.
sys.path.insert(0, str(Path(__file__).resolve().parent))
from build_manual import parse_csv, ManualData, Strut, Cable, Disc  # noqa: E402


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


def rotational_angle(strut: Strut, end_letter: str, disc: Disc,
                     cables: list, joint_xyz: dict) -> int:
    """Rotational angle of this disc around the strut axis, in degrees [0, 360).

    Convention: 0° points 'up relative to the strut' (world +Z projected onto
    the plane perpendicular to the strut axis). 90° is to the right when looking
    from outside the strut toward the joint. Increasing angle goes counter-
    clockwise about the outward axis.
    """
    if end_letter == "A":
        here = strut.alpha_xyz
        axis = v_norm(v_sub(strut.alpha_xyz, strut.omega_xyz))  # outward from this end
    else:
        here = strut.omega_xyz
        axis = v_norm(v_sub(strut.omega_xyz, strut.alpha_xyz))

    cable = next((c for c in cables if c.label == disc.cable_label), None)
    if cable is None:
        return 0
    this_end = f"{strut.label}{end_letter}"
    other_xyz = joint_xyz[cable.omega_joint if cable.alpha_strut_end == this_end else cable.alpha_joint]

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
    right_ref = v_cross(axis, up_ref)

    angle = math.degrees(math.atan2(v_dot(perp, right_ref), v_dot(perp, up_ref)))
    if angle < 0:
        angle += 360.0
    return int(round(angle)) % 360


# Joint-name patterns
_LEG_JOINT  = re.compile(r"^([ABC])X(\d+)(Y?)Z\d+$")  # e.g. AX3Z4 or AX4YZ0
_APEX_JOINT = re.compile(r"^YZ\d+$")                  # e.g. YZ0
_HUB_JOINT  = re.compile(r"^Z\d+$")                   # e.g. Z4


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
    """Find the other 2 struts in the same twist of the same leg."""
    if cat["category"] != "twist":
        return []
    mates = []
    for s in all_struts:
        if s.label == target.label:
            continue
        other_cat = categorise_strut(s)
        if (other_cat["category"] == "twist" and
            other_cat["leg"] == cat["leg"] and
            other_cat["twist"] == cat["twist"]):
            mates.append(s.label)
    return sorted(mates)


def cable_target_for_disc(disc: Disc, this_strut_end: str, cables: list) -> str:
    """Given a disc and the strut-end it's on, find the cable and its other endpoint."""
    cable = next((c for c in cables if c.label == disc.cable_label), None)
    if cable is None:
        return "?"
    if cable.alpha_strut_end == this_strut_end:
        other = cable.omega_strut_end
        other_slot = cable.omega_slot
    else:
        other = cable.alpha_strut_end
        other_slot = cable.alpha_slot
    return f"{other} s{other_slot}"


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
  font-size: 32pt;
  font-family: 'Menlo', 'Consolas', 'Courier New', monospace;
  letter-spacing: 0.05em;
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


def render_disc_label(disc: Disc, target: str, side: str, rot: int) -> str:
    """Build the disc fan-line label.

    Format: 'slot 1 ↻ 045° · CA01 · +30° → SA06B s1'
    where ↻ NNN° is the rotational angle around the strut axis.
    """
    arrow = "→" if side == "B" else "←"
    return (
        f"slot {disc.slot} ↻ {rot:03d}° · {disc.cable_label} · "
        f"{fmt_signed(disc.angle)} {arrow} {target}"
    )


def fan_y_positions(n: int, top: int = 60, step: int = 90) -> list[int]:
    """Vertical positions for n stacked fan labels at one strut end."""
    return [top + i * step for i in range(n)]


def render_strut_diagram(s: Strut, data: ManualData, up_end: str, joint_xyz: dict) -> str:
    """Render the SVG strut diagram for one page."""
    a_discs = data.strut_ends.get((s.label, "A"), [])
    b_discs = data.strut_ends.get((s.label, "B"), [])

    a_targets = [cable_target_for_disc(d, f"{s.label}A", data.cables) for d in a_discs]
    b_targets = [cable_target_for_disc(d, f"{s.label}B", data.cables) for d in b_discs]
    a_rots = [rotational_angle(s, "A", d, data.cables, joint_xyz) for d in a_discs]
    b_rots = [rotational_angle(s, "B", d, data.cables, joint_xyz) for d in b_discs]

    a_ys = fan_y_positions(len(a_discs))
    b_ys = fan_y_positions(len(b_discs))

    a_up = (up_end == "A")
    b_up = (up_end == "B")
    a_ud = "↑ TOP" if a_up else "↓ BOTTOM"
    b_ud = "↑ TOP" if b_up else "↓ BOTTOM"
    a_color = "#1d7a3e" if a_up else "#b32626"
    b_color = "#1d7a3e" if b_up else "#b32626"

    # Wider viewBox so the long fan labels fit on both sides without clipping.
    parts = []
    parts.append(
        '<svg viewBox="-260 0 1520 480" xmlns="http://www.w3.org/2000/svg">'
    )
    # Strut body + caps
    parts.append('<rect x="260" y="220" width="480" height="40" fill="#e8e8e8" stroke="#1a1a1a" stroke-width="1.5"/>')
    parts.append('<rect x="252" y="216" width="12" height="48" fill="#888" stroke="#1a1a1a" stroke-width="1.5"/>')
    parts.append('<rect x="736" y="216" width="12" height="48" fill="#888" stroke="#1a1a1a" stroke-width="1.5"/>')
    # Joint labels (A on left, B on right)
    parts.append(f'<text x="258" y="295" font-family="Menlo,monospace" font-size="18" font-weight="bold" text-anchor="middle">{s.label}A</text>')
    parts.append(f'<text x="258" y="318" font-family="Menlo,monospace" font-size="14" font-weight="bold" text-anchor="middle" fill="{a_color}">{a_ud}</text>')
    parts.append(f'<text x="742" y="295" font-family="Menlo,monospace" font-size="18" font-weight="bold" text-anchor="middle">{s.label}B</text>')
    parts.append(f'<text x="742" y="318" font-family="Menlo,monospace" font-size="14" font-weight="bold" text-anchor="middle" fill="{b_color}">{b_ud}</text>')
    # Length label
    parts.append(f'<text x="500" y="246" font-family="Menlo,monospace" font-size="13" text-anchor="middle" fill="#555">{s.length_mm:.0f} mm</text>')

    # Joint A fan lines (left)
    for disc, target, y, rot in zip(a_discs, a_targets, a_ys, a_rots):
        color = "#1d7a3e" if disc.angle > 0 else ("#b32626" if disc.angle < 0 else "#555")
        label = render_disc_label(disc, target, "A", rot)
        parts.append(f'<line x1="258" y1="240" x2="60" y2="{y}" stroke="{color}" stroke-width="1.5"/>')
        parts.append(f'<text x="55" y="{y - 5}" font-size="10" text-anchor="end" fill="{color}" font-family="Menlo,monospace">{label}</text>')

    # Joint B fan lines (right)
    for disc, target, y, rot in zip(b_discs, b_targets, b_ys, b_rots):
        color = "#1d7a3e" if disc.angle > 0 else ("#b32626" if disc.angle < 0 else "#555")
        label = render_disc_label(disc, target, "B", rot)
        parts.append(f'<line x1="742" y1="240" x2="940" y2="{y}" stroke="{color}" stroke-width="1.5"/>')
        parts.append(f'<text x="945" y="{y - 5}" font-size="10" fill="{color}" font-family="Menlo,monospace">{label}</text>')

    parts.append('</svg>')
    return "\n".join(parts)


def render_endpoint_table(s: Strut, end_letter: str, data: ManualData,
                          is_top: bool, joint_xyz: dict) -> str:
    discs = data.strut_ends.get((s.label, end_letter), [])
    rows_data = []
    for d in discs:
        target = cable_target_for_disc(d, f"{s.label}{end_letter}", data.cables)
        rot = rotational_angle(s, end_letter, d, data.cables, joint_xyz)
        rows_data.append((d, target, rot))
    # Slot order (column 'Rot' shows the physical rotation around the strut).
    rows_data.sort(key=lambda x: x[0].slot)

    rows = []
    for d, target, rot in rows_data:
        cls = "pos" if d.angle > 0 else ("neg" if d.angle < 0 else "")
        rows.append(
            f'<tr><td>{d.slot}</td>'
            f'<td class="id">{d.cable_label}</td>'
            f'<td class="{cls}">{fmt_signed(d.angle)}</td>'
            f'<td class="id">{rot:03d}°</td>'
            f'<td class="id">{target}</td></tr>'
        )
    rows_html = "\n".join(rows)
    ud_text = "↑ TOP" if is_top else "↓ BOTTOM"
    ud_class = "ud-top" if is_top else "ud-bottom"
    return (
        f'<table class="endpoint-table">'
        f'<caption>Joint {s.label}{end_letter} <span class="{ud_class}">{ud_text}</span></caption>'
        f'<thead><tr><th>Slot</th><th>Cable</th><th>Bend</th><th>Rot</th><th>Connects to</th></tr></thead>'
        f'<tbody>{rows_html}</tbody>'
        f'</table>'
    )


def render_rotation_clock(s: Strut, end_letter: str, data: ManualData,
                          joint_xyz: dict, is_top: bool) -> str:
    """Clock-face SVG showing rotational positions of each disc around the strut axis.

    Convention:
      - 0° at top ('up relative to strut' = world +Z projected onto the joint plane)
      - 90° at right, 180° at bottom, 270° at left (clockwise as viewed from outside the strut)
    """
    discs = data.strut_ends.get((s.label, end_letter), [])
    rotations = [
        (d, rotational_angle(s, end_letter, d, data.cables, joint_xyz))
        for d in discs
    ]

    cx, cy, r = 110, 130, 75
    title_color = "#1d7a3e" if is_top else "#b32626"
    title_label = "↑ TOP" if is_top else "↓ BOTTOM"

    parts = ['<svg viewBox="0 0 220 270" xmlns="http://www.w3.org/2000/svg">']
    parts.append(
        f'<text x="110" y="16" font-family="Menlo,monospace" font-size="12" '
        f'font-weight="bold" text-anchor="middle">Joint {s.label}{end_letter}</text>'
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
    up_end, _ = up_down_ends(s)
    mates = twist_mates(s, all_struts, cat)

    subtitle = render_subtitle(s, cat, mates)
    diagram = render_strut_diagram(s, data, up_end, joint_xyz)
    table_a = render_endpoint_table(s, "A", data, is_top=(up_end == "A"), joint_xyz=joint_xyz)
    table_b = render_endpoint_table(s, "B", data, is_top=(up_end == "B"), joint_xyz=joint_xyz)
    clock_a = render_rotation_clock(s, "A", data, joint_xyz, is_top=(up_end == "A"))
    clock_b = render_rotation_clock(s, "B", data, joint_xyz, is_top=(up_end == "B"))

    return (
        f'<div class="strut-page">'
        f'<header>'
        f'<h1>{s.label}</h1>'
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
