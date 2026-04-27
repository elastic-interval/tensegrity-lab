# Blender Add-on: Tensegrity JSON Import (Fast Version)
# Imports tensegrity structure from JSON using batch keyframe creation

bl_info = {
    "name": "Tensegrity Fast Import",
    "author": "Tensegrity Lab",
    "version": (1, 0),
    "blender": (3, 6, 0),
    "location": "View3D > Sidebar > Tensegrity",
    "description": "Fast import of tensegrity structures using batch keyframe creation",
    "category": "Import-Export",
}

SCRIPT_VERSION = "1.0 - 2025-01-21"

# Fixed playback FPS - capture FPS controls slow-motion factor
PLAYBACK_FPS = 30

import bpy
import json
import os
import mathutils
from bpy.props import StringProperty, BoolProperty
from bpy_extras.io_utils import ImportHelper


def find_prototypes_blend():
    """Try to find prototypes.blend in common locations."""
    candidates = []

    try:
        addon_dir = os.path.dirname(__file__)
        candidates.append(os.path.join(addon_dir, "prototypes.blend"))
    except NameError:
        pass

    if bpy.data.filepath:
        blend_dir = os.path.dirname(bpy.data.filepath)
        candidates.extend([
            os.path.join(blend_dir, "scripts", "prototypes.blend"),
            os.path.join(blend_dir, "..", "scripts", "prototypes.blend"),
            os.path.join(blend_dir, "prototypes.blend"),
        ])

    if hasattr(bpy.context, 'scene') and bpy.context.scene:
        json_path = bpy.context.scene.get('tensegrity_last_json_dir')
        if json_path:
            candidates.extend([
                os.path.join(json_path, "scripts", "prototypes.blend"),
                os.path.join(json_path, "..", "scripts", "prototypes.blend"),
            ])

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
    return bpy.data.scenes.new(scene_name)


def center_mesh_to_origin(obj):
    """Move mesh vertices so the geometry is centered at the object's origin."""
    if obj.type != 'MESH':
        return

    mesh = obj.data
    verts = [v.co for v in mesh.vertices]
    if not verts:
        return

    center = sum(verts, mathutils.Vector()) / len(verts)
    for v in mesh.vertices:
        v.co -= center
    mesh.update()


def clear_existing_prototypes():
    """Remove existing prototype objects and their data to allow fresh reload."""
    prototype_names = ['Push', 'Pull', 'Joint', 'Bar', 'Holder']  # Include child names

    # Find and remove prototype objects
    objects_to_remove = []
    for obj in bpy.data.objects:
        base_name = obj.name.split('.')[0]
        if base_name in prototype_names:
            objects_to_remove.append(obj)

    # Remove objects
    for obj in objects_to_remove:
        bpy.data.objects.remove(obj, do_unlink=True)

    # Clean up orphaned meshes with prototype names
    meshes_to_remove = []
    for mesh in bpy.data.meshes:
        base_name = mesh.name.split('.')[0]
        if base_name in prototype_names and mesh.users == 0:
            meshes_to_remove.append(mesh)

    for mesh in meshes_to_remove:
        bpy.data.meshes.remove(mesh)

    # Clean up orphaned materials with prototype-related names
    materials_to_remove = []
    for mat in bpy.data.materials:
        if mat.users == 0:
            materials_to_remove.append(mat)

    for mat in materials_to_remove:
        bpy.data.materials.remove(mat)

    # Remove the Prototypes scene if it exists (will be recreated)
    if "Prototypes" in bpy.data.scenes:
        bpy.data.scenes.remove(bpy.data.scenes["Prototypes"])

    return len(objects_to_remove)


def remove_collection_recursive(collection):
    """Remove a collection and everything inside it: objects, children, subcollections."""
    for child_col in list(collection.children):
        remove_collection_recursive(child_col)
    for obj in list(collection.objects):
        bpy.data.objects.remove(obj, do_unlink=True)
    bpy.data.collections.remove(collection)


def load_prototypes_from_blend(filepath, force_reload=False):
    """Load prototype objects from a .blend file into a Prototypes scene."""
    if not os.path.exists(filepath):
        return None, f"File not found: {filepath}"

    # Clear existing prototypes if force reloading
    if force_reload:
        cleared = clear_existing_prototypes()
        print(f"Cleared {cleared} existing prototype objects")

    proto_scene = get_or_create_prototypes_scene()
    prototype_names = ['Push', 'Pull', 'Joint']
    loaded = []

    with bpy.data.libraries.load(filepath, link=False) as (data_from, data_to):
        data_to.objects = data_from.objects[:]

    for obj in data_to.objects:
        if obj is not None:
            if obj.name not in proto_scene.objects:
                proto_scene.collection.objects.link(obj)

            center_mesh_to_origin(obj)

            base_name = obj.name.split('.')[0]
            if base_name in prototype_names:
                obj.location = (0, 0, 0)
                obj.rotation_euler = (0, 0, 0)
                obj.scale = (1, 1, 1)
                loaded.append(obj.name)

    return loaded, None


def find_prototype_objects(reset_transforms=False):
    """Find prototype objects in the current file."""
    prototypes = {'Push': None, 'Pull': None, 'Joint': None}

    if "Prototypes" in bpy.data.scenes:
        proto_scene = bpy.data.scenes["Prototypes"]
        for obj in proto_scene.objects:
            base_name = obj.name.split('.')[0]
            if base_name in prototypes and prototypes[base_name] is None:
                prototypes[base_name] = obj

    for obj in bpy.data.objects:
        base_name = obj.name.split('.')[0]
        if base_name in prototypes and prototypes[base_name] is None:
            prototypes[base_name] = obj

    # Only reset transforms when explicitly requested (not during UI draw)
    if reset_transforms:
        for name, obj in prototypes.items():
            if obj is not None:
                obj.location = (0, 0, 0)
                obj.rotation_euler = (0, 0, 0)
                obj.scale = (1, 1, 1)

    return prototypes


def matrix_from_list(m):
    """Convert column-major 16-element list to Blender Matrix."""
    return mathutils.Matrix((
        (m[0], m[4], m[8], m[12]),
        (m[1], m[5], m[9], m[13]),
        (m[2], m[6], m[10], m[14]),
        (m[3], m[7], m[11], m[15]),
    ))


def batch_create_keyframes(obj, transform_data, total_frames):
    """
    Create all keyframes for an object using batch operations.

    transform_data: list of (blender_frame, loc, rot, scale) tuples
                   where loc is Vector, rot is Quaternion, scale is Vector
    """
    if not transform_data:
        return

    # Ensure object has animation data
    if obj.animation_data is None:
        obj.animation_data_create()

    if obj.animation_data.action is None:
        obj.animation_data.action = bpy.data.actions.new(name=f"{obj.name}_action")

    action = obj.animation_data.action

    # Set rotation mode
    obj.rotation_mode = 'QUATERNION'

    # Prepare data arrays for each channel
    # Format for foreach_set("co", ...): [frame1, value1, frame2, value2, ...]
    n = len(transform_data)

    loc_x = [0.0] * (n * 2)
    loc_y = [0.0] * (n * 2)
    loc_z = [0.0] * (n * 2)
    rot_w = [0.0] * (n * 2)
    rot_x = [0.0] * (n * 2)
    rot_y = [0.0] * (n * 2)
    rot_z = [0.0] * (n * 2)
    scale_x = [0.0] * (n * 2)
    scale_y = [0.0] * (n * 2)
    scale_z = [0.0] * (n * 2)

    for i, (frame, loc, rot, scale) in enumerate(transform_data):
        idx = i * 2
        loc_x[idx] = float(frame)
        loc_x[idx + 1] = loc.x
        loc_y[idx] = float(frame)
        loc_y[idx + 1] = loc.y
        loc_z[idx] = float(frame)
        loc_z[idx + 1] = loc.z

        rot_w[idx] = float(frame)
        rot_w[idx + 1] = rot.w
        rot_x[idx] = float(frame)
        rot_x[idx + 1] = rot.x
        rot_y[idx] = float(frame)
        rot_y[idx + 1] = rot.y
        rot_z[idx] = float(frame)
        rot_z[idx + 1] = rot.z

        scale_x[idx] = float(frame)
        scale_x[idx + 1] = scale.x
        scale_y[idx] = float(frame)
        scale_y[idx + 1] = scale.y
        scale_z[idx] = float(frame)
        scale_z[idx + 1] = scale.z

    # Create fcurves and batch-set keyframes
    channels = [
        ("location", 0, loc_x),
        ("location", 1, loc_y),
        ("location", 2, loc_z),
        ("rotation_quaternion", 0, rot_w),
        ("rotation_quaternion", 1, rot_x),
        ("rotation_quaternion", 2, rot_y),
        ("rotation_quaternion", 3, rot_z),
        ("scale", 0, scale_x),
        ("scale", 1, scale_y),
        ("scale", 2, scale_z),
    ]

    for data_path, index, co_data in channels:
        # Find or create fcurve
        fcurve = action.fcurves.find(data_path, index=index)
        if fcurve is None:
            fcurve = action.fcurves.new(data_path, index=index)

        # Add keyframe points
        fcurve.keyframe_points.add(n)
        fcurve.keyframe_points.foreach_set("co", co_data)

        # Update the fcurve
        fcurve.update()


def batch_create_visibility_keyframes(obj, first_frame, last_frame, total_frames):
    """Create visibility keyframes using batch operations."""
    if obj.animation_data is None:
        obj.animation_data_create()

    if obj.animation_data.action is None:
        obj.animation_data.action = bpy.data.actions.new(name=f"{obj.name}_action")

    action = obj.animation_data.action

    for data_path in ("hide_viewport", "hide_render"):
        fcurve = action.fcurves.find(data_path)
        if fcurve is None:
            fcurve = action.fcurves.new(data_path)

        # Build keyframe data
        keyframes = []

        if first_frame > 1:
            keyframes.append((1, 1.0))  # Hidden at frame 1

        keyframes.append((first_frame, 0.0))  # Visible when it appears

        if last_frame < total_frames:
            keyframes.append((last_frame + 1, 1.0))  # Hidden after it disappears

        n = len(keyframes)
        co_data = [0.0] * (n * 2)
        for i, (frame, value) in enumerate(keyframes):
            co_data[i * 2] = float(frame)
            co_data[i * 2 + 1] = value

        fcurve.keyframe_points.add(n)
        fcurve.keyframe_points.foreach_set("co", co_data)

        # Set CONSTANT interpolation
        for kp in fcurve.keyframe_points:
            kp.interpolation = 'CONSTANT'

        fcurve.update()

    # Set initial state
    if first_frame > 1:
        obj.hide_viewport = True
        obj.hide_render = True


class TENSEGRITY_OT_fast_import_json(bpy.types.Operator, ImportHelper):
    """Import tensegrity structure from JSON file (fast batch method)"""
    bl_idname = "tensegrity.fast_import_json"
    bl_label = "Fast Import Tensegrity JSON"
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
        description="Handle objects appearing/disappearing during construction",
        default=False,
    )

    def invoke(self, context, event):
        # Initialize from scene property
        self.construction_mode = context.scene.tensegrity_construction_mode
        context.window_manager.fileselect_add(self)
        return {'RUNNING_MODAL'}

    def execute(self, context):
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

        print(f"\n=== Tensegrity Fast Import v{SCRIPT_VERSION} ===")
        print(f"File: {self.filepath}")
        print(f"Frames: {len(frames)}, Construction mode: {self.construction_mode}")

        # Always reload prototypes fresh from prototypes.blend so edits
        # to the .blend file take effect immediately.
        clear_existing_prototypes()
        proto_path = find_prototypes_blend()
        if not proto_path:
            proto_path = os.path.join(json_dir, "scripts", "prototypes.blend")
        loaded, error = load_prototypes_from_blend(proto_path)
        if error:
            self.report({'ERROR'}, f"Failed to load prototypes: {error}")
            return {'CANCELLED'}
        prototypes = find_prototype_objects(reset_transforms=True)
        missing = [name for name, obj in prototypes.items() if obj is None]
        if missing:
            self.report({'ERROR'}, f"Missing prototypes: {', '.join(missing)}")
            return {'CANCELLED'}

        # Remove previous import (entire hierarchy) before re-importing
        json_name = os.path.splitext(os.path.basename(self.filepath))[0]
        collection_name = f"Tensegrity_{json_name}"
        if collection_name in bpy.data.collections:
            remove_collection_recursive(bpy.data.collections[collection_name])

        main_collection = bpy.data.collections.new(collection_name)
        context.scene.collection.children.link(main_collection)

        joints_collection = bpy.data.collections.new("Joints")
        main_collection.children.link(joints_collection)
        push_collection = bpy.data.collections.new("Push")
        main_collection.children.link(push_collection)
        pull_collection = bpy.data.collections.new("Pull")
        main_collection.children.link(pull_collection)

        is_animation = len(frames) > 1
        construction_mode = self.construction_mode and is_animation
        total_frames = len(frames)

        # Phase 1: Collect ALL transform data
        # Maps name -> {
        #   'type': 'Joint'|'Push'|'Pull',
        #   'transforms': [(blender_frame, loc, rot, scale), ...],
        #   'first_frame': int,
        #   'last_frame': int
        # }
        print("Phase 1: Collecting transform data...")
        object_data = {}

        for frame_num, frame in enumerate(frames):
            blender_frame = frame_num + 1

            if frame_num % 100 == 0:
                print(f"  Reading frame {frame_num + 1}/{total_frames}...")

            # Joints
            for joint in frame.get('joints', []):
                name = joint['name']
                matrix = matrix_from_list(joint['matrix'])
                loc, rot, scale = matrix.decompose()

                if name not in object_data:
                    object_data[name] = {
                        'type': 'Joint',
                        'transforms': [],
                        'first_frame': blender_frame,
                        'last_frame': blender_frame
                    }

                object_data[name]['transforms'].append((blender_frame, loc, rot, scale))
                object_data[name]['last_frame'] = blender_frame

            # Push intervals
            for push in frame.get('intervals', {}).get('push', []):
                name = push['name']
                matrix = matrix_from_list(push['matrix'])
                loc, rot, scale = matrix.decompose()

                if name not in object_data:
                    object_data[name] = {
                        'type': 'Push',
                        'transforms': [],
                        'first_frame': blender_frame,
                        'last_frame': blender_frame
                    }

                object_data[name]['transforms'].append((blender_frame, loc, rot, scale))
                object_data[name]['last_frame'] = blender_frame

            # Pull intervals
            for pull in frame.get('intervals', {}).get('pull', []):
                name = pull['name']
                matrix = matrix_from_list(pull['matrix'])
                loc, rot, scale = matrix.decompose()

                if name not in object_data:
                    object_data[name] = {
                        'type': 'Pull',
                        'transforms': [],
                        'first_frame': blender_frame,
                        'last_frame': blender_frame
                    }

                object_data[name]['transforms'].append((blender_frame, loc, rot, scale))
                object_data[name]['last_frame'] = blender_frame

        print(f"  Collected data for {len(object_data)} objects")

        # Phase 2: Create all objects
        print("Phase 2: Creating objects...")
        created_objects = {}
        obj_count = 0

        for name, info in object_data.items():
            obj_count += 1
            if obj_count % 100 == 0:
                print(f"  Created {obj_count}/{len(object_data)} objects...")

            obj_type = info['type']
            first_transform = info['transforms'][0]
            _, loc, rot, scale = first_transform

            if obj_type == 'Joint':
                new_obj = prototypes['Joint'].copy()
                new_obj.data = prototypes['Joint'].data
                new_obj.name = name
                new_obj.hide_render = False
                joints_collection.objects.link(new_obj)
            elif obj_type == 'Push':
                proto = prototypes['Push']
                new_obj = proto.copy()
                if proto.data:
                    new_obj.data = proto.data
                new_obj.name = name
                new_obj.hide_render = False
                push_collection.objects.link(new_obj)
                # Handle children
                for child in proto.children:
                    child_copy = child.copy()
                    if child.data:
                        child_copy.data = child.data
                    child_copy.name = f"{name}_{child.name}"
                    child_copy.parent = new_obj
                    child_copy.matrix_parent_inverse = child.matrix_parent_inverse.copy()
                    child_copy.hide_render = False
                    push_collection.objects.link(child_copy)
            else:  # Pull
                new_obj = prototypes['Pull'].copy()
                new_obj.data = prototypes['Pull'].data
                new_obj.name = name
                new_obj.hide_render = False
                pull_collection.objects.link(new_obj)

            # Set initial transform
            new_obj.location = loc
            new_obj.rotation_mode = 'QUATERNION'
            new_obj.rotation_quaternion = rot
            new_obj.scale = scale

            created_objects[name] = new_obj

        print(f"  Created {len(created_objects)} objects")

        # Phase 3: Batch create keyframes
        if is_animation:
            print("Phase 3: Creating keyframes (batch method)...")
            obj_count = 0

            for name, info in object_data.items():
                obj_count += 1
                if obj_count % 100 == 0:
                    print(f"  Keyframed {obj_count}/{len(object_data)} objects...")

                obj = created_objects[name]

                # Create transform keyframes
                batch_create_keyframes(obj, info['transforms'], total_frames)

                # Create visibility keyframes if in construction mode
                if construction_mode:
                    batch_create_visibility_keyframes(
                        obj,
                        info['first_frame'],
                        info['last_frame'],
                        total_frames
                    )

                    # Also handle Push children visibility
                    if info['type'] == 'Push':
                        for child in obj.children:
                            batch_create_visibility_keyframes(
                                child,
                                info['first_frame'],
                                info['last_frame'],
                                total_frames
                            )

            print(f"  Keyframed {len(object_data)} objects")

        # Set animation range
        if is_animation:
            context.scene.frame_start = 1
            context.scene.frame_end = total_frames
            context.scene.render.fps = PLAYBACK_FPS
            context.scene.frame_set(1)

            capture_fps = data.get('fps', 30.0)
            slowmo_factor = capture_fps / PLAYBACK_FPS
            if slowmo_factor > 1.01:
                print(f"Slow-motion: {slowmo_factor:.1f}x (captured at {capture_fps} FPS, playing at {PLAYBACK_FPS} FPS)")

            # Count keyframes
            keyframe_count = 0
            for obj in created_objects.values():
                if obj.animation_data and obj.animation_data.action:
                    for fcurve in obj.animation_data.action.fcurves:
                        keyframe_count += len(fcurve.keyframe_points)
            print(f"Total keyframes created: {keyframe_count}")

        mode_str = " (construction mode)" if construction_mode else ""
        if is_animation:
            self.report({'INFO'}, f"Imported {len(created_objects)} objects with {total_frames} frames{mode_str}")
        else:
            self.report({'INFO'}, f"Imported {len(created_objects)} objects")

        print(f"=== Import complete ===\n")
        return {'FINISHED'}


class TENSEGRITY_OT_load_prototypes(bpy.types.Operator, ImportHelper):
    """Load prototype objects from prototypes.blend"""
    bl_idname = "tensegrity_fast.load_prototypes"
    bl_label = "Load Prototypes"
    bl_options = {'REGISTER', 'UNDO'}

    filename_ext = ".blend"
    filter_glob: StringProperty(default="*.blend", options={'HIDDEN'})

    force_reload: BoolProperty(
        name="Force Reload",
        description="Clear existing prototypes before loading (use when you've modified prototypes.blend)",
        default=False,
    )

    def execute(self, context):
        loaded, error = load_prototypes_from_blend(self.filepath, force_reload=self.force_reload)
        if error:
            self.report({'ERROR'}, error)
            return {'CANCELLED'}

        if loaded:
            action = "Reloaded" if self.force_reload else "Loaded"
            self.report({'INFO'}, f"{action} prototypes: {', '.join(loaded)}")
        else:
            self.report({'WARNING'}, "No prototype objects found in file")

        return {'FINISHED'}

    def invoke(self, context, event):
        auto_path = find_prototypes_blend()
        if auto_path:
            self.filepath = auto_path
        context.window_manager.fileselect_add(self)
        return {'RUNNING_MODAL'}


class TENSEGRITY_OT_force_reload_prototypes(bpy.types.Operator):
    """Force reload prototypes from prototypes.blend (clears cached versions)"""
    bl_idname = "tensegrity_fast.force_reload_prototypes"
    bl_label = "Force Reload Prototypes"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, context):
        proto_path = find_prototypes_blend()
        if not proto_path:
            self.report({'ERROR'}, "Could not find prototypes.blend")
            return {'CANCELLED'}

        loaded, error = load_prototypes_from_blend(proto_path, force_reload=True)
        if error:
            self.report({'ERROR'}, error)
            return {'CANCELLED'}

        if loaded:
            self.report({'INFO'}, f"Force reloaded prototypes: {', '.join(loaded)}")
        else:
            self.report({'WARNING'}, "No prototype objects found in file")

        return {'FINISHED'}


def ensure_prototypes_loaded():
    """Auto-load prototypes if missing."""
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


class TENSEGRITY_PT_fast_panel(bpy.types.Panel):
    """Panel in the 3D View sidebar"""
    bl_label = "Tensegrity Fast Import"
    bl_idname = "TENSEGRITY_PT_fast_panel"
    bl_space_type = 'VIEW_3D'
    bl_region_type = 'UI'
    bl_category = 'Tensegrity'

    def draw(self, context):
        layout = self.layout

        try:
            prototypes, message = ensure_prototypes_loaded()
            missing = [name for name, obj in prototypes.items() if obj is None]

            if message:
                layout.label(text=message, icon='INFO')

            if missing:
                box = layout.box()
                box.label(text="Missing Prototypes:", icon='ERROR')
                for name in missing:
                    box.label(text=f"  - {name}")
                layout.operator("tensegrity_fast.load_prototypes", icon='IMPORT')
            else:
                box = layout.box()
                box.label(text="Prototypes Ready", icon='CHECKMARK')
                for name, obj in prototypes.items():
                    box.label(text=f"  {name}: {obj.name}")

            layout.separator()

            # Force reload button - always visible
            layout.operator("tensegrity_fast.force_reload_prototypes", icon='FILE_REFRESH')

            layout.separator()

            layout.label(text="Fast Import (Batch Keyframes):")
            layout.prop(context.scene, "tensegrity_construction_mode")
            layout.operator("tensegrity.fast_import_json", icon='IMPORT')

        except Exception as e:
            layout.label(text=f"Error: {e}", icon='ERROR')
            import traceback
            traceback.print_exc()


def menu_func_import(self, context):
    self.layout.operator(TENSEGRITY_OT_fast_import_json.bl_idname, text="Tensegrity JSON Fast (.json)")


classes = (
    TENSEGRITY_OT_fast_import_json,
    TENSEGRITY_OT_load_prototypes,
    TENSEGRITY_OT_force_reload_prototypes,
    TENSEGRITY_PT_fast_panel,
)


def register():
    for cls in classes:
        bpy.utils.register_class(cls)
    bpy.types.TOPBAR_MT_file_import.append(menu_func_import)

    # Add scene property for construction mode
    bpy.types.Scene.tensegrity_construction_mode = BoolProperty(
        name="Construction Animation",
        description="Handle objects appearing/disappearing during construction",
        default=False,
    )


def unregister():
    bpy.types.TOPBAR_MT_file_import.remove(menu_func_import)
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)

    # Remove scene property
    if hasattr(bpy.types.Scene, 'tensegrity_construction_mode'):
        del bpy.types.Scene.tensegrity_construction_mode


if __name__ == "__main__":
    register()
