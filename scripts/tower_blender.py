"""
Open Claw Truss Tower — Blender Model Generator

Run in Blender: Scripting workspace > Open > Run Script
Creates 3 aluminum truss towers on concrete bases, arranged in an
equilateral triangle with 6m center-to-center spacing.

Spec: docs/tower-blender-spec.md
"""

import bpy
import bmesh
import math
from mathutils import Vector

# --- Configuration ---

TOWER_HEIGHT = 2.9          # meters (aluminum truss only)
CONCRETE_HEIGHT = 0.3       # meters
TOTAL_HEIGHT = TOWER_HEIGHT + CONCRETE_HEIGHT

# The imported structure stands on the XY plane (Z=0). Towers sit below
# the surface so the top platform triangles are exactly at Z=0.
TOWER_Z_OFFSET = -TOTAL_HEIGHT  # towers descend from Z=0 down to Z=-3.2

TOWER_BASE_RADIUS = 0.55    # meters (triangular footprint circumradius ~1.1m wide)
TOWER_TOP_RADIUS = 0.35     # meters (slight taper)
PLATFORM_RADIUS = 0.45      # meters (triangular platform at tower top)
TUBE_RADIUS = 0.025         # 50mm OD → 25mm radius
TUBE_SEGMENTS = 8           # polygon count per tube cross-section

CONCRETE_SIZE = 1.5         # meters (square side)

TOWER_SPACING = 6.0         # meters center-to-center

# Bracing levels (Z above concrete top)
LEVELS = [0.0, 0.7, 1.7, 2.6]  # relative to tower base (Z=0.3)

# Colors
COLOR_ALUMINUM = (0.75, 0.75, 0.78, 1.0)   # silver
COLOR_CONCRETE = (0.37, 0.37, 0.37, 1.0)   # dark gray


def clear_scene():
    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.object.delete(use_global=False)
    for mat in list(bpy.data.materials):
        bpy.data.materials.remove(mat)


def make_material(name, color):
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    if bsdf:
        bsdf.inputs["Base Color"].default_value = color
        if "aluminum" in name.lower():
            bsdf.inputs["Metallic"].default_value = 0.9
            bsdf.inputs["Roughness"].default_value = 0.3
        else:
            bsdf.inputs["Metallic"].default_value = 0.0
            bsdf.inputs["Roughness"].default_value = 0.8
    return mat


def add_tube(start, end, radius, collection, material):
    direction = end - start
    length = direction.length
    if length < 1e-6:
        return
    mid = (start + end) / 2

    bpy.ops.mesh.primitive_cylinder_add(
        radius=radius,
        depth=length,
        vertices=TUBE_SEGMENTS,
        location=mid,
    )
    obj = bpy.context.active_object
    obj.name = "tube"

    up = Vector((0, 0, 1))
    axis = up.cross(direction.normalized())
    angle = up.angle(direction.normalized())
    if axis.length > 1e-6:
        obj.rotation_mode = 'AXIS_ANGLE'
        obj.rotation_axis_angle = (angle, axis.x, axis.y, axis.z)

    obj.data.materials.append(material)

    # Move to tower collection
    for c in obj.users_collection:
        c.objects.unlink(obj)
    collection.objects.link(obj)
    return obj


def tower_posts(base_z, base_radius, top_radius, height, n=3):
    """Return list of (bottom, top) pairs for n vertical corner posts."""
    posts = []
    for i in range(n):
        angle = i * 2 * math.pi / n - math.pi / 2  # first post at -Y
        bx = math.cos(angle) * base_radius
        by = math.sin(angle) * base_radius
        tx = math.cos(angle) * top_radius
        ty = math.sin(angle) * top_radius
        posts.append((
            Vector((bx, by, base_z)),
            Vector((tx, ty, base_z + height)),
        ))
    return posts


def post_at_level(post_bottom, post_top, base_z, height, z_rel):
    """Interpolate a post position at relative height z_rel."""
    t = z_rel / height
    return post_bottom.lerp(post_top, t)


def add_platform(center, z, radius, collection, material, name_prefix):
    """Add a triangular platform (flat surface) at the top of a tower."""
    mesh = bpy.data.meshes.new(f"{name_prefix}_platform_mesh")
    obj = bpy.data.objects.new(f"{name_prefix}_platform", mesh)

    bm = bmesh.new()
    verts = []
    for i in range(3):
        angle = i * 2 * math.pi / 3 - math.pi / 2
        v = bm.verts.new((
            center.x + math.cos(angle) * radius,
            center.y + math.sin(angle) * radius,
            z,
        ))
        verts.append(v)
    bm.faces.new(verts)
    bm.to_mesh(mesh)
    bm.free()

    obj.data.materials.append(material)
    collection.objects.link(obj)
    return obj


def build_tower(center, mat_aluminum, mat_concrete):
    """Build one truss tower with concrete base at the given XY center.
    The tower top (platform) sits at Z=0; everything else descends below."""
    name = f"Tower_{center.x:.0f}_{center.y:.0f}"
    collection = bpy.data.collections.new(name)
    bpy.context.scene.collection.children.link(collection)

    # All Z coordinates are shifted so tower top = Z=0
    z_off = TOWER_Z_OFFSET

    # Concrete base (at the bottom of the tower)
    bpy.ops.mesh.primitive_cube_add(
        size=1,
        location=(center.x, center.y, z_off + CONCRETE_HEIGHT / 2),
        scale=(CONCRETE_SIZE, CONCRETE_SIZE, CONCRETE_HEIGHT),
    )
    base = bpy.context.active_object
    base.name = f"{name}_concrete"
    base.data.materials.append(mat_concrete)
    for c in base.users_collection:
        c.objects.unlink(base)
    collection.objects.link(base)

    # Triangular platform at tower top (exactly at Z=0)
    add_platform(center, 0.0, PLATFORM_RADIUS, collection, mat_aluminum, name)

    # Tower geometry
    base_z = z_off + CONCRETE_HEIGHT
    posts = tower_posts(base_z, TOWER_BASE_RADIUS, TOWER_TOP_RADIUS, TOWER_HEIGHT)

    # Offset posts by tower center
    posts = [
        (b + Vector((center.x, center.y, 0)),
         t + Vector((center.x, center.y, 0)))
        for b, t in posts
    ]

    # Vertical struts (main corner posts)
    for bottom, top in posts:
        add_tube(bottom, top, TUBE_RADIUS, collection, mat_aluminum)

    n = len(posts)

    # Horizontal bracing at each level
    for z_rel in LEVELS:
        points = []
        for bottom, top in posts:
            p = post_at_level(bottom, top, base_z, TOWER_HEIGHT, z_rel)
            points.append(p)
        for i in range(n):
            add_tube(points[i], points[(i + 1) % n], TUBE_RADIUS * 0.7, collection, mat_aluminum)

    # Diagonal cross-bracing between levels
    for li in range(len(LEVELS) - 1):
        z_lo = LEVELS[li]
        z_hi = LEVELS[li + 1]
        for i in range(n):
            lo_here = post_at_level(posts[i][0], posts[i][1], base_z, TOWER_HEIGHT, z_lo)
            hi_next = post_at_level(
                posts[(i + 1) % n][0], posts[(i + 1) % n][1],
                base_z, TOWER_HEIGHT, z_hi
            )
            add_tube(lo_here, hi_next, TUBE_RADIUS * 0.5, collection, mat_aluminum)

            hi_here = post_at_level(posts[i][0], posts[i][1], base_z, TOWER_HEIGHT, z_hi)
            lo_next = post_at_level(
                posts[(i + 1) % n][0], posts[(i + 1) % n][1],
                base_z, TOWER_HEIGHT, z_lo
            )
            add_tube(hi_here, lo_next, TUBE_RADIUS * 0.5, collection, mat_aluminum)

    return collection


def main():
    clear_scene()

    mat_aluminum = make_material("Aluminum", COLOR_ALUMINUM)
    mat_concrete = make_material("Concrete", COLOR_CONCRETE)

    # 3 towers in equilateral triangle, 6m spacing
    # One corner pointing toward +Y
    for i in range(3):
        angle = i * 2 * math.pi / 3 + math.pi / 2  # first tower at +Y
        cx = math.cos(angle) * TOWER_SPACING / math.sqrt(3)
        cy = math.sin(angle) * TOWER_SPACING / math.sqrt(3)
        build_tower(Vector((cx, cy, 0)), mat_aluminum, mat_concrete)

    # Set viewport
    for area in bpy.context.screen.areas:
        if area.type == 'VIEW_3D':
            for space in area.spaces:
                if space.type == 'VIEW_3D':
                    space.clip_end = 100
                    space.shading.type = 'MATERIAL'

    print("Tower model created: 3 towers, 6m spacing, Z-up")


if __name__ == "__main__":
    main()
