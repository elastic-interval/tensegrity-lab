# Blender Add-on: Tensegrity JSON Import
# Imports tensegrity structure from JSON and instances prototype objects

bl_info = {
    "name": "Tensegrity Import",
    "author": "Tensegrity Lab",
    "version": (1, 2),
    "blender": (3, 6, 0),
    "location": "View3D > Sidebar > Tensegrity",
    "description": "Import tensegrity structures from JSON using prototype objects",
    "category": "Import-Export",
}

SCRIPT_VERSION = "1.5 - 2024-12-05"

# Fixed playback FPS - capture FPS controls slow-motion factor
PLAYBACK_FPS = 30

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

    prototypes_path: StringProperty(
        name="Prototypes File",
        description="Path to prototypes.blend (leave empty to auto-detect)",
        subtype='FILE_PATH',
    )

    construction_mode: BoolProperty(
        name="Construction Animation",
        description="Handle objects appearing/disappearing during construction. "
                    "Creates all objects upfront and animates visibility",
        default=False,
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

        print(f"\n=== Tensegrity Import v{SCRIPT_VERSION} ===")
        print(f"File: {self.filepath}")
        print(f"Frames: {len(frames)}, Construction mode: {self.construction_mode}")

        # Set up progress indicator
        wm = context.window_manager
        wm.progress_begin(0, 100)

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

        # Import all frames; animate if more than one frame
        frame_indices = range(len(frames))
        is_animation = len(frames) > 1
        construction_mode = self.construction_mode and is_animation

        created_objects = {}
        fps = data.get('fps', 24.0)

        # For construction mode: first pass to find all objects and their visibility ranges
        # Maps name -> {'first': frame, 'last': frame, 'type': str, 'first_matrix': list}
        object_visibility = {}

        if construction_mode:
            print(f"Construction mode: scanning {len(frames)} frames for object visibility...")
            for frame_num, frame_idx in enumerate(frame_indices):
                frame = frames[frame_idx]
                blender_frame = frame_num + 1

                for joint in frame.get('joints', []):
                    name = joint['name']
                    if name not in object_visibility:
                        object_visibility[name] = {
                            'first': blender_frame,
                            'last': blender_frame,
                            'type': 'Joint',
                            'first_matrix': joint['matrix']
                        }
                    else:
                        object_visibility[name]['last'] = blender_frame

                for push in frame.get('intervals', {}).get('push', []):
                    name = push['name']
                    if name not in object_visibility:
                        object_visibility[name] = {
                            'first': blender_frame,
                            'last': blender_frame,
                            'type': 'Push',
                            'first_matrix': push['matrix']
                        }
                    else:
                        object_visibility[name]['last'] = blender_frame

                for pull in frame.get('intervals', {}).get('pull', []):
                    name = pull['name']
                    if name not in object_visibility:
                        object_visibility[name] = {
                            'first': blender_frame,
                            'last': blender_frame,
                            'type': 'Pull',
                            'first_matrix': pull['matrix']
                        }
                    else:
                        object_visibility[name]['last'] = blender_frame

                if frame_num % 10 == 0:
                    wm.progress_update(int(10 * frame_num / len(frames)))
                    bpy.ops.wm.redraw_timer(type='DRAW_WIN_SWAP', iterations=1)
                if frame_num % 100 == 0:
                    print(f"  Scanned {frame_num + 1}/{len(frames)} frames...")

            print(f"Found {len(object_visibility)} unique objects across {len(frames)} frames")
            wm.progress_update(10)

            def keyframe_visibility(obj, first_frame, last_frame, total_frames):
                """Set visibility keyframes for an object with CONSTANT interpolation."""
                # Always start hidden if object doesn't appear at frame 1
                if first_frame > 1:
                    obj.hide_viewport = True
                    obj.hide_render = True
                    obj.keyframe_insert(data_path="hide_viewport", frame=1)
                    obj.keyframe_insert(data_path="hide_render", frame=1)

                # Visible when it appears
                obj.hide_viewport = False
                obj.hide_render = False
                obj.keyframe_insert(data_path="hide_viewport", frame=first_frame)
                obj.keyframe_insert(data_path="hide_render", frame=first_frame)

                # Hidden after it disappears (if before end of animation)
                if last_frame < total_frames:
                    obj.hide_viewport = True
                    obj.hide_render = True
                    obj.keyframe_insert(data_path="hide_viewport", frame=last_frame + 1)
                    obj.keyframe_insert(data_path="hide_render", frame=last_frame + 1)

                # Set interpolation to CONSTANT for visibility keyframes
                # This prevents any interpolation between hidden/visible states
                if obj.animation_data and obj.animation_data.action:
                    for fcurve in obj.animation_data.action.fcurves:
                        if fcurve.data_path in ("hide_viewport", "hide_render"):
                            for keyframe in fcurve.keyframe_points:
                                keyframe.interpolation = 'CONSTANT'

                # Ensure the object's current state is hidden if it appears after frame 1
                # (Blender uses current property value for frames before first keyframe)
                if first_frame > 1:
                    obj.hide_viewport = True
                    obj.hide_render = True

            # Create all objects upfront at their first-appearance position, initially hidden
            print(f"Creating {len(object_visibility)} objects...")
            obj_count = 0
            total_objects = len(object_visibility)
            for name, info in object_visibility.items():
                obj_count += 1
                if obj_count % 10 == 0:
                    wm.progress_update(10 + int(20 * obj_count / total_objects))
                    bpy.ops.wm.redraw_timer(type='DRAW_WIN_SWAP', iterations=1)
                if obj_count % 100 == 0:
                    print(f"  Created {obj_count}/{total_objects} objects...")
                obj_type = info['type']
                first_matrix = matrix_from_list(info['first_matrix'])
                loc, rot, scale = first_matrix.decompose()
                first_frame = info['first']
                last_frame = info['last']

                if obj_type == 'Joint':
                    new_obj = prototypes['Joint'].copy()
                    new_obj.data = prototypes['Joint'].data
                    new_obj.name = name
                    joints_collection.objects.link(new_obj)
                    keyframe_visibility(new_obj, first_frame, last_frame, len(frames))
                elif obj_type == 'Push':
                    proto = prototypes['Push']
                    new_obj = proto.copy()
                    if proto.data:
                        new_obj.data = proto.data
                    new_obj.name = name
                    push_collection.objects.link(new_obj)
                    keyframe_visibility(new_obj, first_frame, last_frame, len(frames))
                    # Also keyframe visibility for children
                    for child in proto.children:
                        child_copy = child.copy()
                        if child.data:
                            child_copy.data = child.data
                        child_copy.name = f"{name}_{child.name}"
                        child_copy.parent = new_obj
                        child_copy.matrix_parent_inverse = child.matrix_parent_inverse.copy()
                        push_collection.objects.link(child_copy)
                        keyframe_visibility(child_copy, first_frame, last_frame, len(frames))
                else:  # Pull
                    new_obj = prototypes['Pull'].copy()
                    new_obj.data = prototypes['Pull'].data
                    new_obj.name = name
                    pull_collection.objects.link(new_obj)
                    keyframe_visibility(new_obj, first_frame, last_frame, len(frames))

                # Set initial position (where it will first appear)
                new_obj.location = loc
                new_obj.rotation_mode = 'QUATERNION'
                new_obj.rotation_quaternion = rot
                new_obj.scale = scale

                created_objects[name] = new_obj

        # Main loop: process each frame
        print(f"Processing {len(frames)} frames...")
        for frame_num, frame_idx in enumerate(frame_indices):
            frame = frames[frame_idx]
            blender_frame = frame_num + 1

            # Import joints
            for joint in frame.get('joints', []):
                name = joint['name']
                matrix = matrix_from_list(joint['matrix'])

                # In construction mode, objects are pre-created; otherwise create on demand
                if name not in created_objects:
                    if construction_mode:
                        continue  # Skip - should have been pre-created
                    new_obj = prototypes['Joint'].copy()
                    new_obj.data = prototypes['Joint'].data
                    new_obj.name = name
                    joints_collection.objects.link(new_obj)
                    created_objects[name] = new_obj

                obj = created_objects[name]
                loc, rot, scale = matrix.decompose()
                obj.location = loc
                obj.rotation_mode = 'QUATERNION'
                obj.rotation_quaternion = rot
                obj.scale = scale

                if is_animation:
                    obj.keyframe_insert(data_path="location", frame=blender_frame)
                    obj.keyframe_insert(data_path="rotation_quaternion", frame=blender_frame)
                    obj.keyframe_insert(data_path="scale", frame=blender_frame)

            # Import push intervals
            for push in frame.get('intervals', {}).get('push', []):
                name = push['name']
                matrix = matrix_from_list(push['matrix'])

                # In construction mode, objects are pre-created; otherwise create on demand
                if name not in created_objects:
                    if construction_mode:
                        continue  # Skip - should have been pre-created
                    proto = prototypes['Push']
                    new_obj = proto.copy()
                    if proto.data:
                        new_obj.data = proto.data
                    new_obj.name = name
                    push_collection.objects.link(new_obj)
                    created_objects[name] = new_obj

                    for child in proto.children:
                        child_copy = child.copy()
                        if child.data:
                            child_copy.data = child.data
                        child_copy.name = f"{name}_{child.name}"
                        child_copy.parent = new_obj
                        child_copy.matrix_parent_inverse = child.matrix_parent_inverse.copy()
                        push_collection.objects.link(child_copy)

                obj = created_objects[name]
                loc, rot, scale = matrix.decompose()
                obj.location = loc
                obj.rotation_mode = 'QUATERNION'
                obj.rotation_quaternion = rot
                obj.scale = scale

                if is_animation:
                    obj.keyframe_insert(data_path="location", frame=blender_frame)
                    obj.keyframe_insert(data_path="rotation_quaternion", frame=blender_frame)
                    obj.keyframe_insert(data_path="scale", frame=blender_frame)

            # Import pull intervals
            for pull in frame.get('intervals', {}).get('pull', []):
                name = pull['name']
                matrix = matrix_from_list(pull['matrix'])

                # In construction mode, objects are pre-created; otherwise create on demand
                if name not in created_objects:
                    if construction_mode:
                        continue  # Skip - should have been pre-created
                    new_obj = prototypes['Pull'].copy()
                    new_obj.data = prototypes['Pull'].data
                    new_obj.name = name
                    pull_collection.objects.link(new_obj)
                    created_objects[name] = new_obj

                obj = created_objects[name]
                loc, rot, scale = matrix.decompose()
                obj.location = loc
                obj.rotation_mode = 'QUATERNION'
                obj.rotation_quaternion = rot
                obj.scale = scale

                if is_animation:
                    obj.keyframe_insert(data_path="location", frame=blender_frame)
                    obj.keyframe_insert(data_path="rotation_quaternion", frame=blender_frame)
                    obj.keyframe_insert(data_path="scale", frame=blender_frame)

            # Progress reporting
            if frame_num % 10 == 0:
                wm.progress_update(30 + int(70 * frame_num / len(frames)))
                bpy.ops.wm.redraw_timer(type='DRAW_WIN_SWAP', iterations=1)
            if frame_num % 50 == 0 or frame_num == len(frames) - 1:
                print(f"Processing frame {frame_num + 1}/{len(frames)}...")

        # Set animation range
        if is_animation:
            context.scene.frame_start = 1
            context.scene.frame_end = len(frames)
            context.scene.render.fps = PLAYBACK_FPS
            context.scene.frame_set(1)

            # Calculate slow-motion factor
            capture_fps = data.get('fps', 30.0)
            slowmo_factor = capture_fps / PLAYBACK_FPS
            if slowmo_factor > 1.01:
                print(f"Slow-motion: {slowmo_factor:.1f}x (captured at {capture_fps} FPS, playing at {PLAYBACK_FPS} FPS)")

            keyframe_count = 0
            for obj in created_objects.values():
                if obj.animation_data and obj.animation_data.action:
                    for fcurve in obj.animation_data.action.fcurves:
                        keyframe_count += len(fcurve.keyframe_points)
            print(f"Total keyframes created: {keyframe_count}")

        wm.progress_end()

        obj_count = len(created_objects)
        mode_str = " (construction mode)" if construction_mode else ""
        if is_animation:
            self.report({'INFO'}, f"Imported {obj_count} objects with {len(frames)} frames{mode_str}")
        else:
            self.report({'INFO'}, f"Imported {obj_count} objects")

        print(f"=== Import complete ===\n")
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
