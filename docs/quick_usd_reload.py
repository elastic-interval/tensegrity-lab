bl_info = {
    "name": "Quick USD Reload",
    "blender": (3, 0, 0),
    "category": "Import-Export",
}

import bpy

USD_PATH = "/Users/fluxe/RustRoverProjects/tensegrity_lab/animation.usda"

def delayed_import():
    # Set world background to black
    bpy.context.scene.world.node_tree.nodes["Background"].inputs[0].default_value = (0, 0, 0, 1)

    # Find a 3D view context
    for window in bpy.context.window_manager.windows:
        for area in window.screen.areas:
            if area.type == 'VIEW_3D':
                with bpy.context.temp_override(window=window, area=area):
                    bpy.ops.wm.usd_import(filepath=USD_PATH)
                break

    # Set imported camera as active
    for obj in bpy.context.scene.objects:
        if obj.type == 'CAMERA':
            bpy.context.scene.camera = obj
            break

    return None

class IMPORT_OT_quick_usd_reload(bpy.types.Operator):
    bl_idname = "import_scene.quick_usd_reload"
    bl_label = "Quick USD Reload"

    def execute(self, context):
        bpy.ops.wm.read_homefile(app_template="")
        bpy.app.timers.register(delayed_import, first_interval=0.5)
        return {'FINISHED'}

addon_keymaps = []

def register():
    bpy.utils.register_class(IMPORT_OT_quick_usd_reload)
    wm = bpy.context.window_manager
    km = wm.keyconfigs.addon.keymaps.new(name='Window', space_type='EMPTY')
    kmi = km.keymap_items.new(IMPORT_OT_quick_usd_reload.bl_idname, 'R', 'PRESS', ctrl=True, shift=True)
    addon_keymaps.append((km, kmi))

def unregister():
    for km, kmi in addon_keymaps:
        km.keymap_items.remove(kmi)
    addon_keymaps.clear()
    bpy.utils.unregister_class(IMPORT_OT_quick_usd_reload)

if __name__ == "__main__":
    register()