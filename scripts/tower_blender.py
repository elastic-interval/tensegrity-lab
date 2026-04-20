"""
Open Claw Truss Tower — Blender Model Generator

Run in Blender: Scripting workspace > Open > Run Script
Creates 3 aluminum truss towers on concrete bases, positioned exactly
at the Open Claw structure's ground contact points. Each tower is
rotated to face the triangle centroid.

Foot positions extracted from settled OpenClaw fabric (sim→Blender Z-up).
Spec: docs/tower-blender-spec.md
"""

import bpy
import bmesh
import math
from mathutils import Vector, Matrix

# --- Tower positions from settled OpenClaw fabric ---
# Extracted by test_open_claw_foot_positions (sim Y-up → Blender Z-up).
# Each tower's platform sits exactly at one of these XY positions, at Z=0.
TOWER_POSITIONS = [
    Vector((-2.4529, 2.3107, 0.0)),
    Vector((3.2296, 0.9693, 0.0)),
    Vector((-0.7744, -3.2807, 0.0)),
]

# --- Configuration ---

TOWER_HEIGHT = 2.9          # meters (aluminum truss only)
CONCRETE_HEIGHT = 0.3       # meters
TOTAL_HEIGHT = TOWER_HEIGHT + CONCRETE_HEIGHT

TOWER_BASE_RADIUS = 0.55    # meters (triangular footprint circumradius ~1.1m wide)
TOWER_TOP_RADIUS = 0.35     # meters (slight taper)
PLATFORM_RADIUS = 0.45      # meters (triangular platform at tower top)
TUBE_RADIUS = 0.025         # 50mm OD → 25mm radius
TUBE_SEGMENTS = 8           # polygon count per tube cross-section

CONCRETE_SIZE = 1.5         # meters (square side)

# Bracing levels (relative to tower base, above concrete top)
LEVELS = [0.0, 0.7, 1.7, 2.6]

# Colors
COLOR_ALUMINUM = (0.75, 0.75, 0.78, 1.0)
COLOR_CONCRETE = (0.37, 0.37, 0.37, 1.0)


def clear_towers():
    """Remove only tower objects/collections from previous runs, leave everything else."""
    for col in list(bpy.data.collections):
        if col.name.startswith("Tower_"):
            for obj in list(col.objects):
                bpy.data.objects.remove(obj, do_unlink=True)
            bpy.data.collections.remove(col)


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

    for c in obj.users_collection:
        c.objects.unlink(obj)
    collection.objects.link(obj)
    return obj


def tower_posts_local(base_z, base_radius, top_radius, height, n=3):
    """Corner posts in local space (centered at origin)."""
    posts = []
    for i in range(n):
        angle = i * 2 * math.pi / n
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
    t = z_rel / height
    return post_bottom.lerp(post_top, t)


def transform_point(point, center, rotation_angle):
    """Rotate point around Z axis by rotation_angle, then translate to center."""
    rot = Matrix.Rotation(rotation_angle, 3, 'Z')
    return rot @ point + center


def add_platform(center, z, radius, rotation_angle, collection, material, name_prefix):
    """Triangular platform at tower top, rotated to face centroid."""
    mesh = bpy.data.meshes.new(f"{name_prefix}_platform_mesh")
    obj = bpy.data.objects.new(f"{name_prefix}_platform", mesh)

    bm = bmesh.new()
    verts = []
    for i in range(3):
        angle = i * 2 * math.pi / 3 + rotation_angle
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


def build_tower(center, rotation_angle, tower_index, mat_aluminum, mat_concrete):
    """Build one truss tower at `center`, rotated by `rotation_angle` about Z
    so that towers arranged in a circle each face inward toward the centroid.
    Tower top (platform) is at Z=0; everything descends below."""
    name = f"Tower_{tower_index}"
    collection = bpy.data.collections.new(name)
    bpy.context.scene.collection.children.link(collection)

    z_off = -TOTAL_HEIGHT

    # Concrete base
    rot = Matrix.Rotation(rotation_angle, 4, 'Z')
    bpy.ops.mesh.primitive_cube_add(
        size=1,
        location=(center.x, center.y, z_off + CONCRETE_HEIGHT / 2),
        scale=(CONCRETE_SIZE, CONCRETE_SIZE, CONCRETE_HEIGHT),
    )
    base_obj = bpy.context.active_object
    base_obj.name = f"{name}_concrete"
    base_obj.rotation_euler = (0, 0, rotation_angle)
    base_obj.data.materials.append(mat_concrete)
    for c in base_obj.users_collection:
        c.objects.unlink(base_obj)
    collection.objects.link(base_obj)

    # Platform at Z=0
    add_platform(center, 0.0, PLATFORM_RADIUS, rotation_angle, collection, mat_aluminum, name)

    # Tower posts in local space, then transformed
    base_z = z_off + CONCRETE_HEIGHT
    local_posts = tower_posts_local(base_z, TOWER_BASE_RADIUS, TOWER_TOP_RADIUS, TOWER_HEIGHT)

    posts = [
        (transform_point(b, center, rotation_angle),
         transform_point(t, center, rotation_angle))
        for b, t in local_posts
    ]

    # Vertical struts
    for bottom, top in posts:
        add_tube(bottom, top, TUBE_RADIUS, collection, mat_aluminum)

    n = len(posts)

    # Horizontal bracing at each level
    for z_rel in LEVELS:
        points = [post_at_level(b, t, base_z, TOWER_HEIGHT, z_rel) for b, t in posts]
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
                base_z, TOWER_HEIGHT, z_hi,
            )
            add_tube(lo_here, hi_next, TUBE_RADIUS * 0.5, collection, mat_aluminum)

            hi_here = post_at_level(posts[i][0], posts[i][1], base_z, TOWER_HEIGHT, z_hi)
            lo_next = post_at_level(
                posts[(i + 1) % n][0], posts[(i + 1) % n][1],
                base_z, TOWER_HEIGHT, z_lo,
            )
            add_tube(hi_here, lo_next, TUBE_RADIUS * 0.5, collection, mat_aluminum)

    return collection


def main():
    clear_towers()

    mat_aluminum = make_material("Aluminum", COLOR_ALUMINUM)
    mat_concrete = make_material("Concrete", COLOR_CONCRETE)

    # Centroid of the three foot positions
    centroid = sum(TOWER_POSITIONS, Vector((0, 0, 0))) / len(TOWER_POSITIONS)

    for i, pos in enumerate(TOWER_POSITIONS):
        # Rotation: each tower faces inward toward the centroid
        to_center = centroid - pos
        rotation_angle = math.atan2(to_center.y, to_center.x)
        build_tower(pos, rotation_angle, i, mat_aluminum, mat_concrete)

    # Set viewport
    for area in bpy.context.screen.areas:
        if area.type == 'VIEW_3D':
            for space in area.spaces:
                if space.type == 'VIEW_3D':
                    space.clip_end = 100
                    space.shading.type = 'MATERIAL'

    print(f"Tower model created: 3 towers at OpenClaw foot positions, facing centroid")
    print(f"Centroid: ({centroid.x:.3f}, {centroid.y:.3f})")
    for i, pos in enumerate(TOWER_POSITIONS):
        print(f"  Tower {i}: ({pos.x:.4f}, {pos.y:.4f})")


if __name__ == "__main__":
    main()
