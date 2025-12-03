# Blender Add-on: Tensegrity JSON Import
# Imports tensegrity structure from JSON and instances prototype objects

bl_info = {
    "name": "Tensegrity Import",
    "author": "Tensegrity Lab",
    "version": (1, 0),
    "blender": (3, 6, 0),
    "location": "View3D > Sidebar > Tensegrity",
    "description": "Import tensegrity structures from JSON using prototype objects",
    "category": "Import-Export",
}

import bpy
import json
import os
import mathutils
from bpy.props import StringProperty, BoolProperty, IntProperty, EnumProperty
from bpy_extras.io_utils import ImportHelper


def find_prototypes_blend():
    """Try to find prototypes.blend in common locations."""
    candidates = []

    # Check in addon/script directory first (same folder as this script)
    try:
        addon_dir = os.path.dirname(__file__)
        candidates.append(os.path.join(addon_dir, "prototypes.blend"))
    except NameError:
        pass  # __file__ not defined when running from Blender text editor

    # Check relative to current blend file
    if bpy.data.filepath:
        blend_dir = os.path.dirname(bpy.data.filepath)
        candidates.extend([
            os.path.join(blend_dir, "scripts", "prototypes.blend"),
            os.path.join(blend_dir, "..", "scripts", "prototypes.blend"),
            os.path.join(blend_dir, "prototypes.blend"),
        ])

    # Check relative to JSON file being imported (stored in scene property if available)
    if hasattr(bpy.context, 'scene') and bpy.context.scene:
        json_path = bpy.context.scene.get('tensegrity_last_json_dir')
        if json_path:
            candidates.extend([
                os.path.join(json_path, "scripts", "prototypes.blend"),
                os.path.join(json_path, "..", "scripts", "prototypes.blend"),
            ])

    # Common project locations
    home = os.path.expanduser("~")
    candidates.extend([
        os.path.join(home, "RustroverProjects", "tensegrity-lab", "scripts", "prototypes.blend"),
    ])

    for path in candidates:
        if os.path.exists(path):
            return os.path.abspath(path)

    return None


def get_or_create_prototypes_scene():
    """Get or create a scene to hold prototype objects."""
    scene_name = "Prototypes"
    if scene_name in bpy.data.scenes:
        return bpy.data.scenes[scene_name]

    # Create new scene
    proto_scene = bpy.data.scenes.new(scene_name)
    return proto_scene


def center_mesh_to_origin(obj):
    """Move mesh vertices so the geometry is centered at the object's origin."""
    if obj.type != 'MESH':
        return

    mesh = obj.data
    # Calculate the center of the mesh bounding box
    verts = [v.co for v in mesh.vertices]
    if not verts:
        return

    center = sum(verts, mathutils.Vector()) / len(verts)

    # Move all vertices so center is at origin
    for v in mesh.vertices:
        v.co -= center

    mesh.update()


def load_prototypes_from_blend(filepath):
    """Load prototype objects from a .blend file into a Prototypes scene."""
    if not os.path.exists(filepath):
        return None, f"File not found: {filepath}"

    proto_scene = get_or_create_prototypes_scene()
    # Push is a composite (may contain Bar+Holder as children or combined mesh)
    # Pull and Joint are simple objects
    prototype_names = ['Push', 'Pull', 'Joint']
    loaded = []

    # Load ALL objects from the file to get the hierarchy
    with bpy.data.libraries.load(filepath, link=False) as (data_from, data_to):
        # Load all objects so we get children too
        data_to.objects = data_from.objects[:]

    # Link imported objects to the prototypes scene
    for obj in data_to.objects:
        if obj is not None:
            # Check if already linked to scene
            if obj.name not in proto_scene.objects:
                proto_scene.collection.objects.link(obj)

            # Center mesh geometry for mesh objects
            center_mesh_to_origin(obj)

            # Only reset transform and track loading for top-level prototypes
            base_name = obj.name.split('.')[0]
            if base_name in prototype_names:
                obj.location = (0, 0, 0)
                obj.rotation_euler = (0, 0, 0)
                obj.scale = (1, 1, 1)
                loaded.append(obj.name)

    return loaded, None


def find_prototype_objects():
    """Find prototype objects in the current file and ensure they're at origin."""
    prototypes = {
        'Push': None,  # Composite prototype for push intervals
        'Pull': None,  # Prototype for pull intervals
        'Joint': None, # Prototype for joints
    }

    # First check in Prototypes scene
    if "Prototypes" in bpy.data.scenes:
        proto_scene = bpy.data.scenes["Prototypes"]
        for obj in proto_scene.objects:
            base_name = obj.name.split('.')[0]
            if base_name in prototypes and prototypes[base_name] is None:
                prototypes[base_name] = obj

    # Then check all objects
    for obj in bpy.data.objects:
        base_name = obj.name.split('.')[0]
        if base_name in prototypes and prototypes[base_name] is None:
            prototypes[base_name] = obj

    # Ensure all found prototypes are at origin (in case they were moved)
    for name, obj in prototypes.items():
        if obj is not None:
            obj.location = (0, 0, 0)
            obj.rotation_euler = (0, 0, 0)
            obj.scale = (1, 1, 1)

    return prototypes


def matrix_from_list(m):
    """Convert column-major 16-element list to Blender Matrix."""
    # Input is column-major: [c0x, c0y, c0z, c0w, c1x, c1y, c1z, c1w, ...]
    # Blender Matrix expects row-major in constructor
    return mathutils.Matrix((
        (m[0], m[4], m[8], m[12]),
        (m[1], m[5], m[9], m[13]),
        (m[2], m[6], m[10], m[14]),
        (m[3], m[7], m[11], m[15]),
    ))


def create_instance_from_prototype(prototype_obj, target_matrix, collection):
    """Create a linked duplicate of prototype with compensated transform.

    The prototype may be positioned away from the world origin in prototypes.blend
    (e.g., placed side-by-side for visibility). We need to compensate for this
    so the instance ends up at the correct world position.

    The mesh vertices are in the prototype's local space. When we copy the object,
    we get the same mesh data. To place it correctly:
    - target_matrix defines where we want the object (assumes mesh centered at origin)
    - prototype's matrix_world tells us where the prototype currently is
    - We need: target_matrix @ inverse(prototype.matrix_world) to cancel out prototype's transform

    But since we're doing a copy (not instance), the mesh is relative to object origin.
    The issue is prototype.location is non-zero. We apply target transform directly,
    but the object's origin offset from world origin causes the shift.

    Solution: The new object should have matrix_world = target_matrix, but we need
    to ensure the mesh is actually centered. Since mesh data is shared, we compensate
    by baking the prototype's offset into understanding that the prototype's local
    origin IS at world origin for the mesh, just the object is translated.

    Actually simpler: just set matrix_world = target_matrix. The issue is the
    prototypes.blend file - the objects should have their origins at their geometric center.
    """
    new_obj = prototype_obj.copy()
    new_obj.data = prototype_obj.data  # Linked data (shared mesh)

    # The target_matrix is what we want. Just apply it directly.
    # If prototypes appear offset, the fix is in prototypes.blend:
    # Select each prototype -> Object -> Set Origin -> Origin to Geometry
    new_obj.matrix_world = target_matrix

    collection.objects.link(new_obj)
    return new_obj


class TENSEGRITY_OT_import_json(bpy.types.Operator, ImportHelper):
    """Import tensegrity structure from JSON file"""
    bl_idname = "tensegrity.import_json"
    bl_label = "Import Tensegrity JSON"
    bl_options = {'REGISTER', 'UNDO'}

    filename_ext = ".json"
    filter_glob: StringProperty(default="*.json", options={'HIDDEN'})

    frame_index: IntProperty(
        name="Frame",
        description="Frame index to import (0 for first frame, -1 for all frames as animation)",
        default=0,
        min=-1,
    )

    import_camera: BoolProperty(
        name="Import Camera",
        description="Import camera position from JSON",
        default=True,
    )

    prototypes_path: StringProperty(
        name="Prototypes File",
        description="Path to prototypes.blend (leave empty to auto-detect)",
        subtype='FILE_PATH',
    )

    def execute(self, context):
        # Store JSON directory for prototype search
        json_dir = os.path.dirname(self.filepath)
        context.scene['tensegrity_last_json_dir'] = json_dir

        # Load JSON
        try:
            with open(self.filepath, 'r') as f:
                data = json.load(f)
        except Exception as e:
            self.report({'ERROR'}, f"Failed to load JSON: {e}")
            return {'CANCELLED'}

        frames = data.get('frames', [])
        if not frames:
            self.report({'ERROR'}, "No frames in JSON file")
            return {'CANCELLED'}

        # Find or load prototypes
        prototypes = find_prototype_objects()
        missing = [name for name, obj in prototypes.items() if obj is None]

        if missing:
            # Try to load from prototypes.blend - check multiple locations
            proto_path = self.prototypes_path or find_prototypes_blend()

            # Also check relative to the JSON file
            if not proto_path:
                json_relative = os.path.join(json_dir, "scripts", "prototypes.blend")
                if os.path.exists(json_relative):
                    proto_path = json_relative

            if proto_path:
                loaded, error = load_prototypes_from_blend(proto_path)
                if error:
                    self.report({'WARNING'}, f"Error loading prototypes: {error}")
                elif loaded:
                    self.report({'INFO'}, f"Auto-loaded prototypes from {proto_path}")
                    prototypes = find_prototype_objects()
                    missing = [name for name, obj in prototypes.items() if obj is None]
            else:
                self.report({'WARNING'}, "Could not find prototypes.blend")

        if missing:
            self.report({'ERROR'}, f"Missing prototypes: {', '.join(missing)}. Use the Tensegrity panel to load prototypes.blend.")
            return {'CANCELLED'}

        # Create collection for the structure
        json_name = os.path.splitext(os.path.basename(self.filepath))[0]
        collection_name = f"Tensegrity_{json_name}"
        if collection_name in bpy.data.collections:
            # Remove existing collection
            old_col = bpy.data.collections[collection_name]
            for obj in old_col.objects:
                bpy.data.objects.remove(obj, do_unlink=True)
            bpy.data.collections.remove(old_col)

        main_collection = bpy.data.collections.new(collection_name)
        context.scene.collection.children.link(main_collection)

        # Create sub-collections
        joints_collection = bpy.data.collections.new("Joints")
        main_collection.children.link(joints_collection)
        push_collection = bpy.data.collections.new("Push")
        main_collection.children.link(push_collection)
        pull_collection = bpy.data.collections.new("Pull")
        main_collection.children.link(pull_collection)

        # Determine which frames to import
        if self.frame_index == -1:
            # Animation mode: import all frames
            frame_indices = range(len(frames))
            is_animation = True
        else:
            # Single frame mode
            idx = min(self.frame_index, len(frames) - 1)
            frame_indices = [idx]
            is_animation = False

        # For animation, create objects on first frame then keyframe
        created_objects = {}
        fps = data.get('fps', 24.0)

        for frame_num, frame_idx in enumerate(frame_indices):
            frame = frames[frame_idx]
            blender_frame = frame_num + 1  # Blender frames start at 1

            if is_animation:
                context.scene.frame_set(blender_frame)

            # Import joints
            for joint in frame.get('joints', []):
                name = joint['name']
                matrix = matrix_from_list(joint['matrix'])

                if name not in created_objects:
                    # Create new object
                    new_obj = prototypes['Joint'].copy()
                    new_obj.data = prototypes['Joint'].data
                    new_obj.name = name
                    joints_collection.objects.link(new_obj)
                    created_objects[name] = new_obj

                obj = created_objects[name]
                obj.matrix_world = matrix

                if is_animation:
                    obj.keyframe_insert(data_path="location", frame=blender_frame)
                    obj.keyframe_insert(data_path="rotation_euler", frame=blender_frame)
                    obj.keyframe_insert(data_path="scale", frame=blender_frame)

            # Import push intervals (single matrix per push, prototype is composite)
            for push in frame.get('intervals', {}).get('push', []):
                name = push['name']
                matrix = matrix_from_list(push['matrix'])

                if name not in created_objects:
                    # Create instance of Push prototype (which may be a composite with children)
                    proto = prototypes['Push']
                    new_obj = proto.copy()
                    if proto.data:
                        new_obj.data = proto.data
                    new_obj.name = name
                    push_collection.objects.link(new_obj)
                    created_objects[name] = new_obj

                    # Also duplicate children and maintain parent relationship
                    for child in proto.children:
                        child_copy = child.copy()
                        if child.data:
                            child_copy.data = child.data
                        child_copy.name = f"{name}_{child.name}"
                        child_copy.parent = new_obj
                        # Preserve the relative transform
                        child_copy.matrix_parent_inverse = child.matrix_parent_inverse.copy()
                        push_collection.objects.link(child_copy)

                obj = created_objects[name]
                obj.matrix_world = matrix

                if is_animation:
                    obj.keyframe_insert(data_path="location", frame=blender_frame)
                    obj.keyframe_insert(data_path="rotation_euler", frame=blender_frame)
                    obj.keyframe_insert(data_path="scale", frame=blender_frame)

            # Import pull intervals
            for pull in frame.get('intervals', {}).get('pull', []):
                name = pull['name']
                matrix = matrix_from_list(pull['matrix'])

                if name not in created_objects:
                    new_obj = prototypes['Pull'].copy()
                    new_obj.data = prototypes['Pull'].data
                    new_obj.name = name
                    pull_collection.objects.link(new_obj)
                    created_objects[name] = new_obj

                obj = created_objects[name]
                obj.matrix_world = matrix

                if is_animation:
                    obj.keyframe_insert(data_path="location", frame=blender_frame)
                    obj.keyframe_insert(data_path="rotation_euler", frame=blender_frame)
                    obj.keyframe_insert(data_path="scale", frame=blender_frame)

            # Import camera
            if self.import_camera and frame.get('camera'):
                cam = frame['camera']
                cam_name = "TensegrityCamera"

                if cam_name not in created_objects:
                    # Create camera
                    cam_data = bpy.data.cameras.new(cam_name)
                    cam_obj = bpy.data.objects.new(cam_name, cam_data)
                    main_collection.objects.link(cam_obj)
                    created_objects[cam_name] = cam_obj

                cam_obj = created_objects[cam_name]
                pos = cam['position']
                target = cam['target']

                cam_obj.location = mathutils.Vector(pos)

                # Point camera at target
                direction = mathutils.Vector(target) - mathutils.Vector(pos)
                rot_quat = direction.to_track_quat('-Z', 'Y')
                cam_obj.rotation_euler = rot_quat.to_euler()

                if is_animation:
                    cam_obj.keyframe_insert(data_path="location", frame=blender_frame)
                    cam_obj.keyframe_insert(data_path="rotation_euler", frame=blender_frame)

        # Set animation range
        if is_animation:
            context.scene.frame_start = 1
            context.scene.frame_end = len(frames)
            context.scene.render.fps = int(fps)
            context.scene.frame_set(1)

        obj_count = len(created_objects)
        if is_animation:
            self.report({'INFO'}, f"Imported {obj_count} objects with {len(frames)} frames of animation")
        else:
            self.report({'INFO'}, f"Imported {obj_count} objects from frame {self.frame_index}")

        return {'FINISHED'}


class TENSEGRITY_OT_load_prototypes(bpy.types.Operator, ImportHelper):
    """Load prototype objects from prototypes.blend"""
    bl_idname = "tensegrity.load_prototypes"
    bl_label = "Load Prototypes"
    bl_options = {'REGISTER', 'UNDO'}

    filename_ext = ".blend"
    filter_glob: StringProperty(default="*.blend", options={'HIDDEN'})

    def execute(self, context):
        loaded, error = load_prototypes_from_blend(self.filepath)
        if error:
            self.report({'ERROR'}, error)
            return {'CANCELLED'}

        if loaded:
            self.report({'INFO'}, f"Loaded prototypes: {', '.join(loaded)}")
        else:
            self.report({'WARNING'}, "No prototype objects found in file")

        return {'FINISHED'}

    def invoke(self, context, event):
        # Try to find prototypes.blend automatically
        auto_path = find_prototypes_blend()
        if auto_path:
            self.filepath = auto_path
        context.window_manager.fileselect_add(self)
        return {'RUNNING_MODAL'}


def ensure_prototypes_loaded():
    """Auto-load prototypes if missing and prototypes.blend can be found."""
    prototypes = find_prototype_objects()
    missing = [name for name, obj in prototypes.items() if obj is None]

    if missing:
        proto_path = find_prototypes_blend()
        if proto_path:
            loaded, error = load_prototypes_from_blend(proto_path)
            if loaded:
                return find_prototype_objects(), f"Auto-loaded: {', '.join(loaded)}"
            elif error:
                return prototypes, f"Failed to load: {error}"
        return prototypes, None
    return prototypes, None


class TENSEGRITY_PT_panel(bpy.types.Panel):
    """Panel in the 3D View sidebar"""
    bl_label = "Tensegrity Import"
    bl_idname = "TENSEGRITY_PT_panel"
    bl_space_type = 'VIEW_3D'
    bl_region_type = 'UI'
    bl_category = 'Tensegrity'

    def draw(self, context):
        layout = self.layout

        # Auto-load prototypes if missing
        prototypes, message = ensure_prototypes_loaded()
        missing = [name for name, obj in prototypes.items() if obj is None]

        if message:
            layout.label(text=message, icon='INFO')

        if missing:
            box = layout.box()
            box.label(text="Missing Prototypes:", icon='ERROR')
            for name in missing:
                box.label(text=f"  - {name}")
            layout.operator("tensegrity.load_prototypes", icon='IMPORT')
        else:
            box = layout.box()
            box.label(text="Prototypes Ready", icon='CHECKMARK')
            for name, obj in prototypes.items():
                box.label(text=f"  {name}: {obj.name}")

        layout.separator()

        # Import button
        layout.label(text="Import Structure:")
        layout.operator("tensegrity.import_json", icon='IMPORT')


def menu_func_import(self, context):
    self.layout.operator(TENSEGRITY_OT_import_json.bl_idname, text="Tensegrity JSON (.json)")


classes = (
    TENSEGRITY_OT_import_json,
    TENSEGRITY_OT_load_prototypes,
    TENSEGRITY_PT_panel,
)


def register():
    for cls in classes:
        bpy.utils.register_class(cls)
    bpy.types.TOPBAR_MT_file_import.append(menu_func_import)


def unregister():
    bpy.types.TOPBAR_MT_file_import.remove(menu_func_import)
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)


if __name__ == "__main__":
    register()
