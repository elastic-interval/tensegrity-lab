//! Propeller → self-contained Blender script export.
//!
//! Drives the Propeller fabric plan to completion, walks its 6 columns and
//! seed brick, and writes a one-file Python script (`scripts/propeller_blender.py`)
//! that — when opened in Blender — builds the geometry from hardcoded
//! constants and animates a camera flying through the structure as a
//! roller-coaster: 3 blade-shaped loops, each returning through the
//! central hub, with banking that tilts in turns and straightens up on
//! the straightaways. One full lap = 60 seconds.
//!
//! Convention: run with
//!     cargo test --release --lib test_propeller_blender_export -- --nocapture
//! mirroring the engineering-CSV pattern in `open_claw_symmetry.rs`.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use glam::Vec3;
use slotmap::Key;

use crate::build::dsl::fabric_library::{get_fabric_plan, FabricName};
use crate::build::dsl::fabric_plan_executor::{FabricPlanExecutor, IterateResult};
use crate::fabric::interval::Role;
use crate::fabric::joint_path::COLUMN_MARKER;
use crate::fabric::{Fabric, JointKey};
use crate::units::Unit;

/// Face-declaration order in `fabric_library.rs::Propeller`:
///   0 = OmniBotX, 1 = OmniBotY, 2 = OmniBotZ,
///   3 = OmniTopX, 4 = OmniTopY, 5 = OmniTopZ
///
/// Joins (3 blades) — see the `tips([(BotX, TopZ), (BotY, TopX), (BotZ, TopY)])`
/// call in the DSL:
///   BotX (0) ↔ TopZ (5)
///   BotY (1) ↔ TopX (3)
///   BotZ (2) ↔ TopY (4)
///
/// The camera path walks each blade outward along the Bot column and
/// returns along the Top column. The order below is chosen so that each
/// blade's inbound axis matches the next blade's outbound axis — i.e.
/// the column entering the seed is collinear (across the centre) with
/// the column leaving it. That straightens the centre crossings and
/// avoids a corner at the origin:
///   blade 1: BotX out, TopZ back  →  centre crossing on Z axis
///   blade 2: BotZ out, TopY back  →  centre crossing on Y axis
///   blade 3: BotY out, TopX back  →  centre crossing on X axis (closes loop)
const BLADES: [(u8, u8); 3] = [(0, 5), (2, 4), (1, 3)];

/// Drive Propeller to `IterateResult::Complete`. Generous cap; the smoke
/// test confirms it completes in well under this.
fn run_propeller_to_complete() -> Fabric {
    let plan = get_fabric_plan(FabricName::Propeller);
    let mut executor = FabricPlanExecutor::new(plan);
    for _ in 0..5_000_000u64 {
        if matches!(executor.iterate(), IterateResult::Complete) {
            return executor.fabric.clone();
        }
    }
    panic!("Propeller did not reach Complete within 5M iterations");
}

/// Extract `(face_index, column_depth)` from a joint's path branches —
/// or `None` for seed-brick joints (empty branches).
fn face_and_depth(branches: &[u8]) -> Option<(u8, u8)> {
    let face = *branches.first()?;
    let depth = branches.iter().skip(1).filter(|&&b| b == COLUMN_MARKER).count() as u8;
    Some((face, depth))
}

/// Walk the fabric and emit the Python script.
fn export(fabric: &Fabric, out_path: &PathBuf) -> std::io::Result<()> {
    // Stable joint index map.
    let mut joint_keys: Vec<JointKey> = fabric.joints.keys().collect();
    joint_keys.sort_by_key(|k| k.data().as_ffi());
    let joint_idx: std::collections::HashMap<JointKey, usize> = joint_keys
        .iter()
        .enumerate()
        .map(|(i, k)| (*k, i))
        .collect();
    let joints: Vec<Vec3> = joint_keys.iter().map(|k| fabric.joints[*k].location).collect();

    // Push/pull pairs as joint indices.
    let mut pushes: Vec<(usize, usize)> = Vec::new();
    let mut pulls: Vec<(usize, usize)> = Vec::new();
    // Accumulate push midpoints per (face, depth) — i.e. per brick. The
    // brick centroid (mean of its 3 push midpoints) is the camera-path
    // control point. Taking one waypoint per push would zig-zag inside
    // each brick and put a visible wobble on the spline.
    let mut brick_sums: BTreeMap<(u8, u8), (Vec3, usize)> = BTreeMap::new();

    for interval in fabric.intervals.values() {
        let a = joint_idx[&interval.alpha_key];
        let o = joint_idx[&interval.omega_key];
        if interval.has_role(Role::Pushing) {
            pushes.push((a, o));
            let alpha_branches = &fabric.joints[interval.alpha_key].path.branches;
            if let Some((face, depth)) = face_and_depth(alpha_branches) {
                let midpoint = (joints[a] + joints[o]) * 0.5;
                let entry = brick_sums.entry((face, depth)).or_insert((Vec3::ZERO, 0));
                entry.0 += midpoint;
                entry.1 += 1;
            }
            // seed-brick pushes (empty branches) are part of the geometry but
            // not the camera path — the bezier between blades handles the centre.
        } else if !interval.has_role(Role::Support) {
            // Every cable-like role: Pulling, BowTie (vulcanize),
            // Circumference (face-triangle perimeters), FaceRadial,
            // Springy, PrismPull. Support is anchors to the ground —
            // floating fabrics like Propeller have none, but skip them
            // either way.
            pulls.push((a, o));
        }
    }

    // Collapse brick sums into one centroid per (face, depth).
    let mut centroids_by_face: BTreeMap<u8, Vec<(u8, Vec3)>> = BTreeMap::new();
    for ((face, depth), (sum, n)) in &brick_sums {
        let c = *sum / *n as f32;
        centroids_by_face.entry(*face).or_default().push((*depth, c));
    }
    for v in centroids_by_face.values_mut() {
        v.sort_by_key(|(d, _)| *d);
    }

    let centroid = fabric.centroid();

    // Build the camera path: for each blade, walk Bot face seed→tip,
    // then Top face tip→seed, and pass through the fabric centroid
    // between blades so the spline drifts straight through the centre
    // rather than veering around it. The closed loop wraps from blade
    // 3's return back to blade 1's outbound through one final centre
    // crossing.
    let mut camera_path: Vec<Vec3> = Vec::new();
    for &(bot_face, top_face) in &BLADES {
        let bot = centroids_by_face
            .get(&bot_face)
            .unwrap_or_else(|| panic!("missing Bot face {bot_face}"));
        let top = centroids_by_face
            .get(&top_face)
            .unwrap_or_else(|| panic!("missing Top face {top_face}"));
        for (_, p) in bot {
            camera_path.push(*p);
        }
        for (_, p) in top.iter().rev() {
            camera_path.push(*p);
        }
        camera_path.push(centroid);
    }

    // Bounding radius and real connector radii (in metres — sim positions
    // and dimensions both use metres at scale 1.0). `centroid` was already
    // captured above for use in the camera path.
    let bounding_radius = fabric.bounding_radius().max(fabric.scale());
    let push_radius = fabric.dimensions.connector.push_radius.f32();
    let pull_radius = fabric.dimensions.pull_radius.f32();

    // Emit Python.
    let mut f = fs::File::create(out_path)?;
    writeln!(f, "{}", PY_HEADER)?;
    writeln!(f, "CENTROID = ({:.5}, {:.5}, {:.5})", centroid.x, centroid.y, centroid.z)?;
    writeln!(f, "BOUNDING_RADIUS = {:.5}", bounding_radius)?;
    writeln!(f, "PUSH_RADIUS = {:.5}", push_radius)?;
    writeln!(f, "PULL_RADIUS = {:.5}", pull_radius)?;
    writeln!(f)?;
    writeln!(f, "JOINTS = [")?;
    for v in &joints {
        writeln!(f, "    ({:.5}, {:.5}, {:.5}),", v.x, v.y, v.z)?;
    }
    writeln!(f, "]")?;
    writeln!(f)?;
    writeln!(f, "PUSHES = [")?;
    for (a, o) in &pushes {
        writeln!(f, "    ({}, {}),", a, o)?;
    }
    writeln!(f, "]")?;
    writeln!(f)?;
    writeln!(f, "PULLS = [")?;
    for (a, o) in &pulls {
        writeln!(f, "    ({}, {}),", a, o)?;
    }
    writeln!(f, "]")?;
    writeln!(f)?;
    writeln!(f, "CAMERA_PATH = [")?;
    for v in &camera_path {
        writeln!(f, "    ({:.5}, {:.5}, {:.5}),", v.x, v.y, v.z)?;
    }
    writeln!(f, "]")?;
    writeln!(f)?;
    writeln!(f, "{}", PY_BUILD)?;
    Ok(())
}

/// Python preamble: imports, docstring, knobs.
const PY_HEADER: &str = r#""""Self-contained Blender script: builds the Propeller tensegrity and
animates a camera flying through its three loops as a roller-coaster.

Generated by `cargo test --release --lib test_propeller_blender_export`.
Edit the source (src/propeller_blender_export.rs) and re-run that test
to regenerate — don't hand-edit the constants below.

Usage:
  - Open Blender with an empty scene.
  - Open this file in the scripting workspace and Run Script (Alt+P), or
    `blender --python scripts/propeller_blender.py`.

Conventions:
  - Coordinates are in metres, Y-up (simulation convention). Blender's
    default Z-up world is rotated 90° around X on import so the
    structure stands the right way.
  - One lap of the camera = 60 seconds at 24 fps.
"""

import bpy
import math
import os
import mathutils
from mathutils import Vector

# ── Knobs ────────────────────────────────────────────────────────────────────
FPS = 24
LAP_SECONDS = 60.0
TOTAL_FRAMES = int(FPS * LAP_SECONDS)

# PUSH_RADIUS / PULL_RADIUS come from the fabric's actual connector
# dimensions (in metres). Tweak the multipliers if you want fatter
# struts for stylised renders.
PUSH_RADIUS_SCALE = 1.0
PULL_RADIUS_SCALE = 0.25  # quarter of the physical cable thickness — these
                          # are tension lines and read best thin.
BANK_GAIN = 0.6               # how much to tilt into corners (radians per
                              # unit curvature); 0 = no banking

# Look-ahead for the camera target, as a fraction of the loop.
LOOKAHEAD_FRACTION = 0.04

# Evening-scene knobs. Coloured spots are placed at this distance from
# the centroid (as a multiple of the bounding radius), and the world
# HDRI is dimmed by HDRI_STRENGTH.
LIGHT_DISTANCE_FACTOR = 1.6
HDRI_STRENGTH = 0.08
"#;

/// Python body: builders for geometry, the spline, banking, and camera.
const PY_BUILD: &str = r#"
# ── Helpers ──────────────────────────────────────────────────────────────────

def clear_scene():
    """Remove every object from the active scene — keep the slate clean."""
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    for coll in (bpy.data.meshes, bpy.data.curves, bpy.data.cameras,
                 bpy.data.lights, bpy.data.materials):
        for item in list(coll):
            if item.users == 0:
                coll.remove(item)


def make_material(name, rgba, metallic=0.0, roughness=0.5):
    """Principled BSDF — same shader the rest of the project's Blender
    scripts use, so the propeller responds properly to the evening lights."""
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    if bsdf:
        bsdf.inputs["Base Color"].default_value = rgba
        bsdf.inputs["Metallic"].default_value = metallic
        bsdf.inputs["Roughness"].default_value = roughness
    # Viewport solid-mode colour falls back to this:
    mat.diffuse_color = rgba
    return mat


# ── Scripts directory + HDRI ────────────────────────────────────────────────

def _find_scripts_dir():
    """Same logic as scripts/create-environment.py — `__file__` is
    unreliable in Blender's text editor, so try a few likely homes."""
    candidates = []
    try:
        candidates.append(os.path.dirname(os.path.abspath(__file__)))
    except (NameError, OSError):
        pass
    if bpy.data.filepath:
        blend_dir = os.path.dirname(bpy.data.filepath)
        candidates.append(os.path.join(blend_dir, "scripts"))
        candidates.append(os.path.join(blend_dir, "..", "scripts"))
    candidates.append(os.path.expanduser("~/RustroverProjects/tensegrity-lab/scripts"))
    for d in candidates:
        if os.path.isdir(d):
            return os.path.abspath(d)
    return "."


SCRIPT_DIR = _find_scripts_dir()
HDRI_TEXTURE = os.path.join(SCRIPT_DIR, "moonless_golf_4k.exr")


def sim_to_world(p):
    """Sim is Y-up; PropellerRoot rotates by +90° around X to make it
    Z-up. The world-coords version of a sim point (x, y, z) is (x, -z, y)."""
    x, y, z = p
    return (x, -z, y)


def cylinder_between(p1, p2, radius, material, name, end_fill="NGON"):
    """Create a cylinder primitive between two points. `end_fill` can be
    "NGON" (default — flat cap) or "NOTHING" (no end geometry; only safe
    when something else is going to cover the ends)."""
    a = Vector(p1)
    b = Vector(p2)
    vec = b - a
    length = vec.length
    if length < 1e-9:
        return None
    midpoint = (a + b) * 0.5
    bpy.ops.mesh.primitive_cylinder_add(
        radius=radius, depth=length, location=midpoint, end_fill_type=end_fill,
    )
    obj = bpy.context.active_object
    obj.name = name
    # Align the cylinder's local Z axis with `vec`.
    z = Vector((0.0, 0.0, 1.0))
    if vec.length > 0:
        axis = z.cross(vec)
        if axis.length < 1e-9:
            # parallel or anti-parallel
            if vec.dot(z) < 0:
                obj.rotation_euler = (math.pi, 0.0, 0.0)
        else:
            angle = z.angle(vec)
            obj.rotation_mode = "AXIS_ANGLE"
            obj.rotation_axis_angle = (angle, *axis.normalized())
    obj.data.materials.append(material)
    return obj


def build_geometry():
    # Brushed-aluminium struts, warm dielectric cables, joint-balls
    # using the same metal as the struts so they read as natural caps.
    push_mat = make_material("Push", (0.78, 0.78, 0.82, 1.0),
                              metallic=1.0, roughness=0.22)
    pull_mat = make_material("Pull", (0.95, 0.95, 0.95, 1.0),
                              metallic=0.0, roughness=0.4)
    joint_mat = make_material("Joint", (0.72, 0.72, 0.76, 1.0),
                               metallic=1.0, roughness=0.28)
    push_r = PUSH_RADIUS * PUSH_RADIUS_SCALE
    pull_r = PULL_RADIUS * PULL_RADIUS_SCALE
    for i, (a, o) in enumerate(PUSHES):
        cylinder_between(JOINTS[a], JOINTS[o], push_r, push_mat, f"Push.{i:03d}")
    # Pull ends sit inside the joint spheres, so they don't need caps.
    for i, (a, o) in enumerate(PULLS):
        cylinder_between(JOINTS[a], JOINTS[o], pull_r, pull_mat,
                         f"Pull.{i:03d}", end_fill="NOTHING")
    # One sphere per joint, sized to the push radius — covers every
    # cylinder end (both push and pull) where it meets the joint, so
    # the flat cylinder caps and the pull-cable end-disc artefacts
    # disappear behind a clean hemispherical cap.
    for i, p in enumerate(JOINTS):
        bpy.ops.mesh.primitive_uv_sphere_add(
            radius=push_r, segments=16, ring_count=10, location=p,
        )
        obj = bpy.context.active_object
        obj.name = f"Joint.{i:03d}"
        # Smooth shading so the spheres don't show facets.
        for poly in obj.data.polygons:
            poly.use_smooth = True
        obj.data.materials.append(joint_mat)


# ── Camera path ──────────────────────────────────────────────────────────────

def _compute_banking_tilts(points):
    """For each control point i, the tilt that rolls the camera into
    the turn at that point. Signed magnitude of the cross product
    between adjacent tangents, projected onto world-up."""
    pts = [Vector(p) for p in points]
    n = len(pts)
    up = Vector((0.0, 1.0, 0.0))  # sim is Y-up; PropellerRoot rotates it to Z-up
    tilts = [0.0] * n
    for i in range(n):
        prev_t = pts[i] - pts[(i - 1) % n]
        next_t = pts[(i + 1) % n] - pts[i]
        if prev_t.length < 1e-9 or next_t.length < 1e-9:
            continue
        prev_t.normalize()
        next_t.normalize()
        curvature_vec = next_t - prev_t
        # signed lean: positive = roll into right turn
        signed = curvature_vec.dot(prev_t.cross(up))
        tilts[i] = BANK_GAIN * signed
    return tilts


def build_camera_curve():
    """Closed bezier with one control point per *original* path waypoint.
    Blender's AUTO handles need enough spacing between control points to
    produce real curvature — packing in one point per frame (the prior
    bug) collapsed the spline into a polyline. Tilts bank into corners."""
    tilts = _compute_banking_tilts(CAMERA_PATH)

    curve_data = bpy.data.curves.new("CamPath", type="CURVE")
    curve_data.dimensions = "3D"
    curve_data.use_path = True
    curve_data.path_duration = TOTAL_FRAMES
    spline = curve_data.splines.new(type="BEZIER")
    spline.use_cyclic_u = True
    spline.bezier_points.add(len(CAMERA_PATH) - 1)
    for bp, pt, tilt in zip(spline.bezier_points, CAMERA_PATH, tilts):
        bp.co = Vector(pt)
        bp.handle_left_type = "AUTO"
        bp.handle_right_type = "AUTO"
        bp.tilt = tilt

    curve_obj = bpy.data.objects.new("CamPath", curve_data)
    bpy.context.collection.objects.link(curve_obj)
    return curve_obj


def setup_camera(curve_obj):
    """Camera follows the curve. A target Empty follows the same curve
    a small offset ahead, and the camera tracks it — so the camera is
    always looking down the next stretch of track."""
    # Camera
    cam_data = bpy.data.cameras.new("Camera")
    cam_data.lens = 28.0
    cam_obj = bpy.data.objects.new("Camera", cam_data)
    bpy.context.collection.objects.link(cam_obj)
    cam_obj.location = (0.0, 0.0, 0.0)

    follow = cam_obj.constraints.new(type="FOLLOW_PATH")
    follow.target = curve_obj
    follow.use_curve_follow = True
    follow.forward_axis = "FORWARD_Y"
    follow.up_axis = "UP_Z"
    # The follow constraint starts the camera at the path's start at frame 1.

    # Path animation: drive evaluation_time linearly over the lap.
    path_anim = curve_obj.data.animation_data_create()
    action = bpy.data.actions.new("CamPathDrive")
    path_anim.action = action
    fcurve = action.fcurves.new(data_path="eval_time", index=-1)
    fcurve.keyframe_points.add(2)
    fcurve.keyframe_points[0].co = (1, 0)
    fcurve.keyframe_points[0].interpolation = "LINEAR"
    fcurve.keyframe_points[1].co = (TOTAL_FRAMES, TOTAL_FRAMES)
    fcurve.keyframe_points[1].interpolation = "LINEAR"

    # Look-ahead target Empty: same path, offset ahead in evaluation time.
    target_curve = curve_obj.copy()
    target_curve.data = curve_obj.data.copy()
    target_curve.data.name = "CamPathTarget"
    target_curve.name = "CamPathTarget"
    bpy.context.collection.objects.link(target_curve)

    tgt = bpy.data.objects.new("CamTarget", None)
    tgt.empty_display_type = "PLAIN_AXES"
    tgt.empty_display_size = 0.05 * BOUNDING_RADIUS
    bpy.context.collection.objects.link(tgt)
    t_follow = tgt.constraints.new(type="FOLLOW_PATH")
    t_follow.target = target_curve
    t_follow.use_curve_follow = False

    lookahead = int(LOOKAHEAD_FRACTION * TOTAL_FRAMES)
    target_anim = target_curve.data.animation_data_create()
    target_action = bpy.data.actions.new("CamPathTargetDrive")
    target_anim.action = target_action
    t_fc = target_action.fcurves.new(data_path="eval_time", index=-1)
    t_fc.keyframe_points.add(2)
    t_fc.keyframe_points[0].co = (1, lookahead)
    t_fc.keyframe_points[0].interpolation = "LINEAR"
    t_fc.keyframe_points[1].co = (TOTAL_FRAMES, TOTAL_FRAMES + lookahead)
    t_fc.keyframe_points[1].interpolation = "LINEAR"

    track = cam_obj.constraints.new(type="TRACK_TO")
    track.target = tgt
    track.track_axis = "TRACK_NEGATIVE_Z"
    track.up_axis = "UP_Y"

    bpy.context.scene.camera = cam_obj


# ── Evening world + lighting ────────────────────────────────────────────────

def setup_evening_world():
    """World background: night-sky HDRI dimmed for an evening feel, with
    a flat dark-blue fallback if the HDRI file isn't found alongside the
    script."""
    world = bpy.context.scene.world
    if world is None:
        world = bpy.data.worlds.new("World")
        bpy.context.scene.world = world
    world.use_nodes = True
    nodes = world.node_tree.nodes
    links = world.node_tree.links
    nodes.clear()

    output = nodes.new("ShaderNodeOutputWorld")
    output.location = (300, 0)
    background = nodes.new("ShaderNodeBackground")
    background.location = (0, 0)
    background.inputs["Strength"].default_value = HDRI_STRENGTH
    links.new(background.outputs["Background"], output.inputs["Surface"])

    if os.path.exists(HDRI_TEXTURE):
        env_tex = nodes.new("ShaderNodeTexEnvironment")
        env_tex.location = (-300, 0)
        env_tex.image = bpy.data.images.load(HDRI_TEXTURE)
        links.new(env_tex.outputs["Color"], background.inputs["Color"])
        print(f"Loaded HDRI: {HDRI_TEXTURE}")
    else:
        background.inputs["Color"].default_value = (0.02, 0.03, 0.07, 1.0)
        print(f"NOTE: HDRI not found at {HDRI_TEXTURE} — using flat evening blue.")


def build_lighting():
    """Three coloured spotlights at 120° intervals around the propeller's
    bounding sphere (in world coords — these are NOT parented to the
    rotating PropellerRoot, so they stay put while the structure spins
    relative to them), plus a soft moonlight fill from above."""
    cx, cy, cz = sim_to_world(CENTROID)
    distance = BOUNDING_RADIUS * LIGHT_DISTANCE_FACTOR

    colors = [
        ((1.0, 0.18, 0.12), "Red"),
        ((0.12, 1.0, 0.18), "Green"),
        ((0.18, 0.22, 1.0), "Blue"),
    ]
    for i, (color, name) in enumerate(colors):
        angle = i * 2 * math.pi / 3
        pos = Vector((
            cx + math.cos(angle) * distance,
            cy + math.sin(angle) * distance,
            cz,
        ))
        target = Vector((cx, cy, cz))
        direction = target - pos
        rot_quat = direction.to_track_quat("-Z", "Y")

        spot_data = bpy.data.lights.new(f"Spot_{name}", "SPOT")
        spot_data.energy = 8000.0
        spot_data.color = color
        spot_data.spot_size = math.radians(100)
        spot_data.spot_blend = 0.6
        spot_data.shadow_soft_size = 1.5
        spot_obj = bpy.data.objects.new(f"Spot_{name}", spot_data)
        spot_obj.location = pos
        spot_obj.rotation_euler = rot_quat.to_euler()
        bpy.context.collection.objects.link(spot_obj)

    # Soft moonlight fill from above (world Z+).
    sun_data = bpy.data.lights.new("Moonlight", "SUN")
    sun_data.energy = 1.0
    sun_data.color = (0.7, 0.78, 1.0)
    sun_data.angle = math.radians(12)
    sun_obj = bpy.data.objects.new("Moonlight", sun_data)
    sun_obj.location = (cx, cy, cz + distance)
    sun_obj.rotation_euler = (math.radians(15), math.radians(20), 0.0)
    bpy.context.collection.objects.link(sun_obj)


# ── World orientation ────────────────────────────────────────────────────────

def orient_world():
    """Sim is Y-up; Blender's world is Z-up. Wrap the propeller geometry
    in an Empty rotated 90° around X so it stands the right way.
    Lights live in world coords (placed by build_lighting using
    sim_to_world) and stay put when the structure spins around them."""
    parent = bpy.data.objects.new("PropellerRoot", None)
    parent.empty_display_type = "PLAIN_AXES"
    bpy.context.collection.objects.link(parent)
    parent.rotation_euler = (math.pi / 2.0, 0.0, 0.0)
    for obj in list(bpy.context.scene.objects):
        if obj is parent:
            continue
        if obj.type in {"LIGHT"}:
            continue
        if obj.parent is None:
            obj.parent = parent


# ── Entry point ──────────────────────────────────────────────────────────────

def main():
    clear_scene()
    bpy.context.scene.frame_start = 1
    bpy.context.scene.frame_end = TOTAL_FRAMES
    bpy.context.scene.render.fps = FPS

    build_geometry()
    curve = build_camera_curve()
    setup_camera(curve)
    build_lighting()
    setup_evening_world()
    orient_world()

    # Eevee bloom looks nice on the spot beams. (Cycles renders them too;
    # this is a no-op there.)
    eevee = getattr(bpy.context.scene, "eevee", None)
    if eevee is not None and hasattr(eevee, "use_bloom"):
        eevee.use_bloom = True

    print(f"Propeller built — {len(PUSHES)} pushes, {len(PULLS)} pulls, "
          f"{len(CAMERA_PATH)} path waypoints, {TOTAL_FRAMES} frames "
          f"({LAP_SECONDS}s at {FPS}fps).")


if __name__ == "__main__":
    main()
"#;

#[cfg(test)]
mod tests {
    use super::*;

    /// Build Propeller, write `scripts/propeller_blender.py` next to the
    /// other build scripts. Run with:
    ///     cargo test --release --lib test_propeller_blender_export -- --nocapture
    #[test]
    fn test_propeller_blender_export() {
        let fabric = run_propeller_to_complete();

        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let out_path = PathBuf::from(manifest_dir).join("scripts/propeller_blender.py");
        export(&fabric, &out_path).expect("write propeller_blender.py");

        let n_pushes = fabric.intervals.values().filter(|i| i.has_role(Role::Pushing)).count();
        // Cables = everything except pushes and ground anchors.
        let n_cables = fabric.intervals.values()
            .filter(|i| !i.has_role(Role::Pushing) && !i.has_role(Role::Support))
            .count();
        eprintln!(
            "wrote {} — {} joints, {} pushes, {} cables",
            out_path.display(),
            fabric.joints.len(),
            n_pushes,
            n_cables,
        );
    }
}
