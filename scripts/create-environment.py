"""
Open Claw Environment — Blender Scene Generator

Run in Blender: Scripting workspace > Open > Run Script
Creates:
  - 3 aluminum truss towers on concrete bases at OpenClaw foot positions
  - Textured ground circle (8m radius, hexagonal concrete paving)
  - Night sky HDRI panorama

Foot positions extracted from settled OpenClaw fabric (sim→Blender Z-up).
Spec: docs/tower-blender-spec.md
"""

import bpy
import bmesh
import math
import os
from mathutils import Vector, Matrix

# --- Resolve texture paths ---
# __file__ is unreliable in Blender's text editor. Try it first,
# fall back to known project location.
def _find_scripts_dir():
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
GROUND_TEXTURE = os.path.join(SCRIPT_DIR, "0002-concrete-paving-outdoor-hexagonal-texture-seamless.jpg")
HDRI_TEXTURE = os.path.join(SCRIPT_DIR, "moonless_golf_4k.exr")

# --- Tower positions from settled OpenClaw fabric ---
TOWER_POSITIONS = [
    Vector((-2.4529, 2.3107, 0.0)),
    Vector((3.2296, 0.9693, 0.0)),
    Vector((-0.7744, -3.2807, 0.0)),
]

# --- Configuration ---

TOWER_HEIGHT = 2.9
CONCRETE_HEIGHT = 0.3
TOTAL_HEIGHT = TOWER_HEIGHT + CONCRETE_HEIGHT

TOWER_BASE_RADIUS = 0.55
TOWER_TOP_RADIUS = 0.35
PLATFORM_RADIUS = 0.45
TUBE_RADIUS = 0.025
TUBE_SEGMENTS = 8

CONCRETE_SIZE = 1.5

GROUND_RADIUS = 8.0         # meters
GROUND_TEXTURE_SCALE = 4.0  # how many times the texture tiles across the diameter

LEVELS = [0.0, 0.7, 1.7, 2.6]

COLOR_ALUMINUM = (0.75, 0.75, 0.78, 1.0)
COLOR_CONCRETE = (0.37, 0.37, 0.37, 1.0)


# --- Cleanup ---

def remove_collection_recursive(collection):
    for child_col in list(collection.children):
        remove_collection_recursive(child_col)
    for obj in list(collection.objects):
        bpy.data.objects.remove(obj, do_unlink=True)
    bpy.data.collections.remove(collection)


def clear_environment():
    """Remove towers, ground, and environment collections from previous runs."""
    for col in list(bpy.data.collections):
        if col.name.startswith("Tower_") or col.name in ("Ground", "Lighting"):
            remove_collection_recursive(col)


# --- Materials ---

def make_material(name, color):
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    if bsdf:
        bsdf.inputs["Base Color"].default_value = color
        if "aluminum" in name.lower():
            bsdf.inputs["Metallic"].default_value = 1.0
            bsdf.inputs["Roughness"].default_value = 0.15
        else:
            bsdf.inputs["Metallic"].default_value = 0.0
            bsdf.inputs["Roughness"].default_value = 0.8
    return mat


def make_ground_material():
    """Create a material with the hexagonal concrete paving texture."""
    mat = bpy.data.materials.new("GroundPaving")
    mat.use_nodes = True
    nodes = mat.node_tree.nodes
    links = mat.node_tree.links
    nodes.clear()

    # Nodes: Texture Coordinate → Mapping → Image Texture → Principled BSDF → Output
    output = nodes.new('ShaderNodeOutputMaterial')
    output.location = (400, 0)

    bsdf = nodes.new('ShaderNodeBsdfPrincipled')
    bsdf.location = (100, 0)
    bsdf.inputs["Roughness"].default_value = 0.9
    links.new(bsdf.outputs["BSDF"], output.inputs["Surface"])

    tex_image = nodes.new('ShaderNodeTexImage')
    tex_image.location = (-300, 0)
    if os.path.exists(GROUND_TEXTURE):
        tex_image.image = bpy.data.images.load(GROUND_TEXTURE)
    else:
        print(f"WARNING: Ground texture not found: {GROUND_TEXTURE}")
    links.new(tex_image.outputs["Color"], bsdf.inputs["Base Color"])

    mapping = nodes.new('ShaderNodeMapping')
    mapping.location = (-500, 0)
    mapping.inputs["Scale"].default_value = (GROUND_TEXTURE_SCALE, GROUND_TEXTURE_SCALE, 1.0)
    links.new(mapping.outputs["Vector"], tex_image.inputs["Vector"])

    tex_coord = nodes.new('ShaderNodeTexCoord')
    tex_coord.location = (-700, 0)
    links.new(tex_coord.outputs["Generated"], mapping.inputs["Vector"])

    return mat


# --- Geometry helpers ---

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
    rot = Matrix.Rotation(rotation_angle, 3, 'Z')
    return rot @ point + center


def add_platform(center, z, radius, rotation_angle, collection, material, name_prefix):
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


# --- Builders ---

def build_tower(center, rotation_angle, tower_index, mat_aluminum, mat_concrete):
    name = f"Tower_{tower_index}"
    collection = bpy.data.collections.new(name)
    bpy.context.scene.collection.children.link(collection)

    z_off = -TOTAL_HEIGHT

    # Concrete base
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

    # Tower posts
    base_z = z_off + CONCRETE_HEIGHT
    local_posts = tower_posts_local(base_z, TOWER_BASE_RADIUS, TOWER_TOP_RADIUS, TOWER_HEIGHT)
    posts = [
        (transform_point(b, center, rotation_angle),
         transform_point(t, center, rotation_angle))
        for b, t in local_posts
    ]

    for bottom, top in posts:
        add_tube(bottom, top, TUBE_RADIUS, collection, mat_aluminum)

    n = len(posts)

    for z_rel in LEVELS:
        points = [post_at_level(b, t, base_z, TOWER_HEIGHT, z_rel) for b, t in posts]
        for i in range(n):
            add_tube(points[i], points[(i + 1) % n], TUBE_RADIUS * 0.7, collection, mat_aluminum)

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


def build_ground():
    """Create a textured ground circle at the base of the towers."""
    collection = bpy.data.collections.new("Ground")
    bpy.context.scene.collection.children.link(collection)

    bpy.ops.mesh.primitive_circle_add(
        radius=GROUND_RADIUS,
        vertices=64,
        fill_type='NGON',
        location=(0, 0, -TOTAL_HEIGHT),
    )
    ground = bpy.context.active_object
    ground.name = "Ground_circle"

    mat = make_ground_material()
    ground.data.materials.append(mat)

    for c in ground.users_collection:
        c.objects.unlink(ground)
    collection.objects.link(ground)

    return ground


def setup_hdri_sky():
    """Set the world background to the night sky HDRI panorama."""
    world = bpy.context.scene.world
    if world is None:
        world = bpy.data.worlds.new("World")
        bpy.context.scene.world = world

    world.use_nodes = True
    nodes = world.node_tree.nodes
    links = world.node_tree.links
    nodes.clear()

    output = nodes.new('ShaderNodeOutputWorld')
    output.location = (300, 0)

    background = nodes.new('ShaderNodeBackground')
    background.location = (0, 0)
    background.inputs["Strength"].default_value = 0.1
    links.new(background.outputs["Background"], output.inputs["Surface"])

    env_tex = nodes.new('ShaderNodeTexEnvironment')
    env_tex.location = (-300, 0)
    if os.path.exists(HDRI_TEXTURE):
        env_tex.image = bpy.data.images.load(HDRI_TEXTURE)
    else:
        print(f"WARNING: HDRI texture not found: {HDRI_TEXTURE}")
    links.new(env_tex.outputs["Color"], background.inputs["Color"])


def make_emissive_material(name, color, strength=10.0):
    """Glowing material for light fixture housing."""
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    nodes = mat.node_tree.nodes
    links = mat.node_tree.links
    nodes.clear()
    output = nodes.new('ShaderNodeOutputMaterial')
    output.location = (200, 0)
    emission = nodes.new('ShaderNodeEmission')
    emission.location = (0, 0)
    emission.inputs["Color"].default_value = (*color, 1.0)
    emission.inputs["Strength"].default_value = strength
    links.new(emission.outputs["Emission"], output.inputs["Surface"])
    return mat


def build_lighting():
    """Festival-style lighting: colored spotlights with visible fixture
    housings on the ground, plus a soft moonlight fill from above."""
    collection = bpy.data.collections.new("Lighting")
    bpy.context.scene.collection.children.link(collection)

    centroid = sum(TOWER_POSITIONS, Vector((0, 0, 0))) / len(TOWER_POSITIONS)

    # RGB uplights — one per tower
    spot_colors = [
        ((1.0, 0.15, 0.1), "Red"),     # red
        ((0.1, 1.0, 0.15), "Green"),    # green
        ((0.15, 0.2, 1.0), "Blue"),     # blue
    ]

    fixture_radius = 0.075  # housing cylinder radius
    fixture_height = 0.125  # housing cylinder height
    lens_radius = 0.06      # glowing lens disc

    for i, pos in enumerate(TOWER_POSITIONS):
        color, color_name = spot_colors[i]

        # Mount at tower top (Z=0), slightly inward toward centroid
        to_center = (centroid - pos).normalized()
        mount_pos = Vector((
            pos.x + to_center.x * 0.3,
            pos.y + to_center.y * 0.3,
            0.0,
        ))

        # Aim at structure center, tilted upward
        target = Vector((centroid.x, centroid.y, 6.5))
        direction = target - mount_pos
        rot_quat = direction.to_track_quat('-Z', 'Y')
        rot_euler = rot_quat.to_euler()

        # Fixture housing — dark cylinder, hanging from tower top
        bpy.ops.mesh.primitive_cylinder_add(
            radius=fixture_radius,
            depth=fixture_height,
            vertices=12,
            location=mount_pos,
        )
        housing = bpy.context.active_object
        housing.name = f"Fixture_{color_name}"
        housing.rotation_euler = rot_euler
        mat_housing = make_material(f"FixtureHousing_{color_name}", (0.1, 0.1, 0.1, 1.0))
        housing.data.materials.append(mat_housing)
        for c in housing.users_collection:
            c.objects.unlink(housing)
        collection.objects.link(housing)

        # Glowing lens
        lens_pos = mount_pos + direction.normalized() * (fixture_height * 0.5)
        bpy.ops.mesh.primitive_circle_add(
            radius=lens_radius,
            vertices=16,
            fill_type='NGON',
            location=lens_pos,
        )
        lens = bpy.context.active_object
        lens.name = f"Lens_{color_name}"
        lens.rotation_euler = rot_euler
        mat_lens = make_emissive_material(f"Lens_{color_name}_mat", color, strength=50.0)
        lens.data.materials.append(mat_lens)
        for c in lens.users_collection:
            c.objects.unlink(lens)
        collection.objects.link(lens)

        # Spot light — shining inward and down onto structure
        spot_data = bpy.data.lights.new(f"Spotlight_{color_name}", 'SPOT')
        spot_data.energy = 10000
        spot_data.color = color
        spot_data.spot_size = math.radians(80)
        spot_data.spot_blend = 0.7
        spot_data.shadow_soft_size = 1.0

        spot_obj = bpy.data.objects.new(f"Spotlight_{color_name}", spot_data)
        spot_obj.location = lens_pos
        spot_obj.rotation_euler = rot_euler
        collection.objects.link(spot_obj)

    # Soft overhead fill light (moonlight stand-in)
    sun_data = bpy.data.lights.new("Moonlight", 'SUN')
    sun_data.energy = 1.0
    sun_data.color = (0.7, 0.75, 1.0)
    sun_data.angle = math.radians(10)

    sun_obj = bpy.data.objects.new("Moonlight", sun_data)
    sun_obj.location = (0, 0, 20)
    sun_obj.rotation_euler = (math.radians(30), math.radians(10), 0)
    collection.objects.link(sun_obj)

    return collection


# --- Main ---

def main():
    clear_environment()

    mat_aluminum = make_material("Aluminum", COLOR_ALUMINUM)
    mat_concrete = make_material("Concrete", COLOR_CONCRETE)

    centroid = sum(TOWER_POSITIONS, Vector((0, 0, 0))) / len(TOWER_POSITIONS)

    for i, pos in enumerate(TOWER_POSITIONS):
        to_center = centroid - pos
        rotation_angle = math.atan2(to_center.y, to_center.x)
        build_tower(pos, rotation_angle, i, mat_aluminum, mat_concrete)

    build_ground()
    build_lighting()
    setup_hdri_sky()

    # Set viewport
    for area in bpy.context.screen.areas:
        if area.type == 'VIEW_3D':
            for space in area.spaces:
                if space.type == 'VIEW_3D':
                    space.clip_end = 100
                    space.shading.type = 'MATERIAL'

    print(f"Environment created: 3 towers, ground circle ({GROUND_RADIUS}m), night sky HDRI")
    print(f"Scripts dir: {SCRIPT_DIR}")
    print(f"Ground texture: {'FOUND' if os.path.exists(GROUND_TEXTURE) else 'MISSING'} — {GROUND_TEXTURE}")
    print(f"HDRI texture:   {'FOUND' if os.path.exists(HDRI_TEXTURE) else 'MISSING'} — {HDRI_TEXTURE}")
    print(f"Centroid: ({centroid.x:.3f}, {centroid.y:.3f})")
    for i, pos in enumerate(TOWER_POSITIONS):
        print(f"  Tower {i}: ({pos.x:.4f}, {pos.y:.4f})")


if __name__ == "__main__":
    main()
