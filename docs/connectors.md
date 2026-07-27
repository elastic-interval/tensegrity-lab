# Connectors: How Cables Attach to Struts

## Physical Reality

In a tensegrity structure, every cable (pull interval) terminates at the end of a strut (push interval). The physical connection hardware at each strut end is called a **connector**. Each strut end can have multiple cables attached, each at a distinct **slot** along the strut axis.

The connector part itself — specified in full under [The Fabricated Part](#the-fabricated-part) below — is a **flat steel ring** that turns on an axial bolt at the strut end, carrying a **radial boss** that ends in a **cross-tube**. The cable's fork terminal is pinned through the tube.

The assembly at a strut end consists of:

1. **Cap** — A fixed end-cap on the strut tube; the bolt passes through it
2. **Rings** — One connector per cable, stacked on the bolt and retained by a nut
3. **Washers** — A thin divider washer between cap and first ring and between adjacent rings, so steel never bears on steel
4. **Pivot pins** — The cable's fork pivots on a pin through each connector's cross-tube

Two free rotations mean the cable naturally finds its exact direction:

- **Azimuth** — the ring turns freely on the bolt
- **Elevation** — the fork pivots freely on its pin

Because of this, **every connector is geometrically identical**. There are no per-position variants, no manufactured bend angles, and no bend-magnitude inventory — all of that machinery from the previous angled-disc design is gone.

## The Fabricated Part

Self-contained geometric specification for the fabricated steel part: a **flat ring** carrying a **radial boss** that ends in a **flat**, with a short **cross-tube** welded to that flat, the tube's axis perpendicular to the ring's axis. Everything needed to generate the solid — in code (CadQuery/OpenSCAD) or in Blender — is below. All dimensions in millimetres.

### Parameters

```
# ---- Ring ----
D_ring   = 40.0    # ring outer diameter
d_bore   = 13.0    # central through-hole diameter (turns on a 12 mm bolt, +1 clearance)
t_ring   = 5.0     # ring thickness (measured along the ring axis)

# ---- Boss (radial extension ending in a flat) ----
R_flat   = 24.0    # radius from ring centre to the flat face (4 mm past the ring edge)
w_boss   = 12.0    # boss width, tangential
t_boss   = 5.0     # boss thickness along the ring axis (flush with the ring)

# ---- Cross-tube (welded to the flat; axis perpendicular to the ring axis) ----
D_tube   = 20.0    # tube outer diameter (4 mm wall around the 12 mm bore)
d_tube   = 12.0    # tube bore diameter (takes a 10 mm pin, +2 clearance to pivot)
L_tube   = 10.0    # tube length along its own axis (fills a fork's jaws; estimated from site photo, verify with caliper)
seat     = 2.0     # depth the flat cuts into the tube wall (½ of the 4 mm wall) → flat weld land

# ---- Derived ----
R_tube   = R_flat + D_tube/2 - seat   # = 32.0  : tube-axis radius (= cable moment arm); tube seated 2 mm into the flat
```

The simulation's `ConnectorDimensions` carries the subset it needs: `ring_thickness` = t_ring, `pivot_radius` = R_tube, plus the assembly-level `cap_thickness` and `washer_thickness` (the divider washer is part of the stack, not of this part).

### Coordinate frame

- **Origin** at the ring centre, on the ring's mid-plane.
- **Z** = ring axis; the central hole runs along Z. The ring occupies z ∈ [−t_ring/2, +t_ring/2] = [−2.5, +2.5].
- **X** = radial direction toward the boss and tube.
- **Y** = the tube axis (and its bore axis). Y ⊥ Z, satisfying "tube axis perpendicular to the ring/bolt axis."

### Construction (ordered primitives + booleans)

1. **Ring** — solid cylinder, diameter `D_ring` (40), height `t_ring` (5), axis along Z, centred at the origin.
2. **Boss** — rectangular block spanning **x ∈ [0, R_flat] = [0, 24]**, **y ∈ [−w_boss/2, +w_boss/2] = [−6, +6]**, **z ∈ [−t_boss/2, +t_boss/2] = [−2.5, +2.5]**. **Union** with the ring. Its outer face is the flat at **x = R_flat = 24**.
3. **Central hole** — subtract a cylinder of diameter `d_bore` (13), axis along Z, passing fully through the body.
4. **Cross-tube** — build a cylinder of outer diameter `D_tube` (20), length `L_tube` (10), **axis along Y**, centred at **(x, y, z) = (R_tube, 0, 0) = (32, 0, 0)**; subtract its bore of diameter `d_tube` (12) along the same Y axis. **Union** with the body. The tube is **seated `seat` = 2 mm into the flat** (its outer wall reaches x = 22, so the flat at x = 24 cuts a flat land into the wall), leaving 2 mm of wall to the bore — a flat weld land ~12 mm wide (chord z = ±6) running the tube's full 10 mm length, instead of a tangent line. The boss flat (`w_boss` = 12 along Y) is wide enough to back the whole 10 mm tube.
5. **Result** — one solid with exactly two openings: the Ø13 central hole (axis Z) and the Ø12 tube bore (axis Y).

Optional for a display model: add a fillet weld bead around the land where the tube meets the flat (around x ≈ 24, over the boss height z ∈ [−2.5, +2.5]).

### Function (kinematics of the finished part)

- The ring **turns on the bolt** through its central hole → aims the tube in any direction around Z (azimuth).
- A fork end pinned through the tube bore **pivots on the pin** about Y → swings in the X–Z plane (elevation).
- The two together let a cable attach at any orientation, so **every connector is geometrically identical** — no per-position variants.

### Blender script

Paste into Blender's **Scripting** workspace and Run (▶). It **clears the scene**, builds the connector as one solid named `Connector`, gives it a steel material, and sets up floor + lighting + camera + Cycles — so you can just press **F12** to render. Re-run any time; it wipes and rebuilds. (Unit block only affects on-screen measurement.)

```python
import bpy, math

# ===== Parameters (mm) — from the spec above =====
D_ring  = 40.0     # ring outer diameter
d_bore  = 13.0     # central hole (turns on a 12 mm bolt)
t_ring  = 5.0      # ring thickness
R_flat  = 24.0     # radius to the flat face
w_boss  = 12.0     # boss width (tangential)
t_boss  = 5.0      # boss thickness (along axis)
D_tube  = 20.0     # tube outer diameter (4 mm wall)
d_tube  = 12.0     # tube bore (takes a 10 mm pin)
L_tube  = 10.0     # tube length
seat    = 2.0      # flat cuts 2 mm into the tube wall (weld land)
R_tube  = R_flat + D_tube/2.0 - seat   # = 32.0
SEG     = 96       # cylinder smoothness
# Frame: Z = ring axis, X = radial (boss), Y = tube axis
# =================================================

def clear_scene():
    if bpy.context.active_object and bpy.context.active_object.mode != 'OBJECT':
        bpy.ops.object.mode_set(mode='OBJECT')
    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.object.delete(use_global=False)
    for coll in (bpy.data.meshes, bpy.data.materials, bpy.data.lights, bpy.data.cameras):
        for item in list(coll):
            coll.remove(item)

def cyl(dia, depth, loc, rot=(0,0,0)):
    bpy.ops.mesh.primitive_cylinder_add(vertices=SEG, radius=dia/2.0, depth=depth, location=loc, rotation=rot)
    return bpy.context.active_object

def box(dx, dy, dz, loc):
    bpy.ops.mesh.primitive_cube_add(size=1, location=loc)
    o = bpy.context.active_object
    o.scale = (dx, dy, dz)
    bpy.ops.object.transform_apply(scale=True)
    return o

def boolean(target, tool, op):
    m = target.modifiers.new("bool", 'BOOLEAN')
    m.operation, m.solver, m.object = op, 'EXACT', tool
    bpy.ops.object.select_all(action='DESELECT')
    target.select_set(True)
    bpy.context.view_layer.objects.active = target
    bpy.ops.object.modifier_apply(modifier=m.name)
    bpy.data.objects.remove(tool, do_unlink=True)

clear_scene()

# ---- build the connector ----
part = cyl(D_ring, t_ring, (0, 0, 0))                                           # ring
boolean(part, box(R_flat, w_boss, t_boss, (R_flat/2.0, 0, 0)), 'UNION')         # boss to the flat
boolean(part, cyl(d_bore, t_ring + 2, (0, 0, 0)), 'DIFFERENCE')                 # bolt hole
boolean(part, cyl(D_tube, L_tube,     (R_tube, 0, 0), (math.radians(90),0,0)), 'UNION')       # tube (seated 2 mm)
boolean(part, cyl(d_tube, L_tube + 2, (R_tube, 0, 0), (math.radians(90),0,0)), 'DIFFERENCE')  # bore
part.name = "Connector"

# ---- smooth curved faces, keep flats crisp ----
bpy.context.view_layer.objects.active = part
part.select_set(True)
bpy.ops.object.shade_smooth()
try:
    bpy.ops.object.shade_auto_smooth(angle=math.radians(30))
except Exception:
    pass

# ---- steel material ----
steel = bpy.data.materials.new("Steel")
steel.use_nodes = True
b = steel.node_tree.nodes["Principled BSDF"]
b.inputs["Base Color"].default_value = (0.56, 0.58, 0.60, 1.0)
b.inputs["Metallic"].default_value  = 1.0
b.inputs["Roughness"].default_value = 0.40   # slight satin finish reads the form better
part.data.materials.append(steel)

# ---- floor ----
bpy.ops.mesh.primitive_plane_add(size=1000, location=(0, 0, -D_tube/2.0))
floor = bpy.context.active_object
fm = bpy.data.materials.new("Floor"); fm.use_nodes = True
fb = fm.node_tree.nodes["Principled BSDF"]
fb.inputs["Base Color"].default_value = (0.17, 0.17, 0.18, 1.0)
fb.inputs["Roughness"].default_value = 0.9
floor.data.materials.append(fm)

# ---- lighting: physical-sky environment (scale-independent — it lights the part,
#      gives the metal something to reflect, and casts the shadow) + a soft fill ----
world = bpy.context.scene.world or bpy.data.worlds.new("World")
bpy.context.scene.world = world
world.use_nodes = True
nt = world.node_tree
for n in list(nt.nodes):
    nt.nodes.remove(n)
out = nt.nodes.new("ShaderNodeOutputWorld")
bg  = nt.nodes.new("ShaderNodeBackground")
sky = nt.nodes.new("ShaderNodeTexSky")
sky.sky_type = 'NISHITA'
sky.sun_elevation = math.radians(28)      # sun height  -> raise for softer, lower for dramatic
sky.sun_rotation  = math.radians(-50)     # sun compass -> spins the highlight + shadow
bg.inputs["Strength"].default_value = 1.0 # overall brightness
nt.links.new(sky.outputs["Color"], bg.inputs["Color"])
nt.links.new(bg.outputs["Background"], out.inputs["Surface"])

fill = bpy.data.lights.new("Fill", 'SUN')  # opens the shadow side
fill.energy, fill.angle = 1.5, math.radians(8)
fo = bpy.data.objects.new("Fill", fill)
bpy.context.collection.objects.link(fo)
fo.rotation_euler = (math.radians(60), math.radians(-20), math.radians(-130))

# ---- camera aimed at the part ----
cam_data = bpy.data.cameras.new("Camera"); cam_data.lens = 60
cam = bpy.data.objects.new("Camera", cam_data)
bpy.context.collection.objects.link(cam)
cam.location = (95, -120, 75)
bpy.context.scene.camera = cam
target = bpy.data.objects.new("Target", None)
bpy.context.collection.objects.link(target)
target.location = (12, 0, 0)
tc = cam.constraints.new('TRACK_TO')
tc.target, tc.track_axis, tc.up_axis = target, 'TRACK_NEGATIVE_Z', 'UP_Y'

# ---- render settings ----
sc = bpy.context.scene
sc.render.engine = 'CYCLES'                 # nice metal; set to a Eevee engine for speed
try:
    sc.cycles.samples = 160
    sc.cycles.use_denoising = True
except Exception: pass
sc.render.resolution_x, sc.render.resolution_y = 1600, 1200

# ---- display measurements in mm ----
u = sc.unit_settings
u.system, u.length_unit, u.scale_length = 'METRIC', 'MILLIMETERS', 0.001

bpy.ops.object.select_all(action='DESELECT')
part.select_set(True); bpy.context.view_layer.objects.active = part
print("Connector built + scene ready. Press F12 to render.")
```

### Background (not part of the spec — how the numbers were chosen)

`d_bore` 13 = an M12 bolt + clearance · `t_ring` 5 = the existing steel plate thickness · `d_tube` 12 = the 10 mm clevis pin + 2 mm play · `D_ring` 40 = the existing cap-plate diameter · `L_tube` 10 ≈ the fork's inner jaw gap, estimated from a site photo of the old tab-in-fork connection (5 mm tab + ~2.5 mm shim washer each side) · `t_boss` 5, `t_tube_wall` 4 (→ `D_tube` 20), `e_boss` 4, `seat` 2 are working design choices.

**Weld seat:** the tube is set `seat` = 2 mm into the flat (½ the 4 mm wall), giving a flat land ~12 mm wide for a flat-on-flat weld instead of a tangent line, while keeping 2 mm of wall to the bore.

**Minimising the moment arm:** the cable acts at `R_tube` = 32 from the strut axis (ring radius 20 + boss 4 + tube radius 10, less the 2 mm seat). The boss is already minimal; the arm is dominated by the **ring radius** — shrink `D_ring` to reduce it further. Limited by ring strength around the bolt and how it seats on the cap.

Design load ≈ 8–9 kN per cable end (Peter's `Controle lip`/`Controle pen`). Verify with a caliper on a fork terminal: the **jaw inner gap → `L_tube`**. Source data: the Engineering notes, and the terminal + pin drawings in `ENS/Bouwboek Open Claw v20260421.zip`.

## Geometry and Dimensions

All physical dimensions are defined in `ConnectorDimensions` (`src/connector/dimensions.rs`).
Defaults reflect the values in `Default::default()` and are the source of truth;
this table is informational and can drift between code edits and doc updates.

| Parameter | Default | Description |
|-----------|---------|-------------|
| `ring_thickness` | 5 mm | Ring thickness along the bolt axis (t_ring) |
| `washer_thickness` | 1 mm | Divider washer between cap/ring and between adjacent rings |
| `cap_thickness` | 5 mm | Strut end-cap the bolt passes through |
| `pivot_radius` | 32 mm | Radial distance from the strut/bolt axis to the pivot pin (R_tube — the cable's moment arm) |

## Slot Positioning Along the Strut Axis

Slots are numbered starting from 0 (code-internal) or 1 (display/CSV). The position of each slot's ring center along the strut axis, measured from the strut endpoint, is calculated by `ConnectorDimensions::ring_center_offset()`:

```
offset = cap_thickness + washer + ring_thickness/2 + slot * (ring_thickness + washer)
```

For 0-indexed slots with the current defaults (cap=5 mm, t_ring=5 mm, washer=1 mm):
- Slot 0: 5 + 1 + 2.5 = 8.5 mm from the strut end
- Slot 1: 14.5 mm
- Slot 2: 20.5 mm

`ConnectorDimensions::ring_center()` uses `ring_center_offset()` to compute the 3D position of a ring center given a strut endpoint and axis direction. `generate_attachment_points()` (`src/connector/attachment.rs`) produces `ATTACHMENT_POINTS = 10` candidate slots per end from the same formula.

## How Cables Get Assigned to Slots

When the fabric is built and settled, each push interval determines which cables connect to it and at which slot (`ConnectorSystem::update_all_connections`, rebuilt on demand — Viewing entry, attachment-point toggle-ON, CSV export). The optimiser (`find_optimal_assignment`, `src/connector/attachment.rs`) works per push end:

- **Hard rule**: an outward-pulling cable may not occupy the topmost slot — the retaining nut would otherwise carry the full axial load.
- **Soft rules**: the cable whose arm points most opposite the outward cable sits directly above it (the "lid"), and the topmost outward cable sits second from the top.
- **Soft objective**: among permutations satisfying the above, maximise the minimum 3D distance between any pair of connector arms (segments from ring center to pivot pin).
- **Tie-break**: minimum rotational moment about the slot-0 ring center.

## Pivot Geometry: How Cable Endpoints are Positioned

Once a cable is assigned to a slot, its attachment is calculated by
`ConnectorDimensions::pivot_geometry()`, returning `(pivot_pos, elevation_deg)`:

1. **Ring center**: position along the strut axis at this slot
2. **Radial direction**: perpendicular to the strut axis, pointing toward the cable's far end (the ring turns to face it)
3. **Pivot pin position** (`pivot_pos`): ring center + radial direction × `pivot_radius` — this *is* the cable endpoint
4. **Elevation** (`elevation_deg`): the free angle the fork takes toward the far end, `asin(pull_direction · push_axis)` measured from the radial plane (0° = radial, positive tilts outward along the strut axis)

The elevation is purely informational — nothing is manufactured to an angle; it is reported so articulation limits of the fork can be checked.

## Rendering

The connector assembly is rendered by `ConnectorRenderer` (`src/wgpu/connector_renderer.rs`) as symbolic solid steel parts:

| Part | Shape | Dimensions |
|------|-------|------------|
| **Cap** | Flush cylindrical continuation of the strut tube | `cap_thickness` long, strut radius |
| **Ring + boss** | One flat plate per occupied slot: a disc with the boss as part of its outline, aimed at the cable | `ring_thickness` thick, disc radius = strut radius (D_ring/2), boss to R_flat |
| **Cross-tube** | Stubby cylinder along the tangent at the pivot | D_tube/2 × L_tube |
| **Fork jaws** | Two rounded-nose plates astride the tube (same stadium outline family as the ring+boss, rendered by the plate pipeline), long axis along the cable — so forks visibly articulate with the free pivot | ~4.5 mm thick, ~9.5 mm nose radius (estimated; awaits terminal drawings) |
| **Clevis pin** | Cylinder along the tangent through tube and jaws, protruding past each jaw | ⌀10 mm |
| **Swage shank** | Cylinder along the cable beyond the jaws; the cable disappears into it | ⌀12 mm × 45 mm |

The plate uses its own mesh and pipeline (`create_connector_plate`, `plate_vertex` in `shader.wgsl`) because its instances need a full orientation — the boss must point toward the cable, whereas a plain cylinder instance leaves the azimuth arbitrary. Washer gaps are left as empty space, so the stack reads as separate rings. The cable itself (drawn by `cylinder_renderer.rs`, at the true 6 mm cable diameter via `pull_radius`) ends at the pivot pin and disappears inside the cross-tube, so the attachment reads as a solid connection. Rendering is only visible when `show_attachment_points()` is enabled in the render style.

## CSV Export

The CSV export (`src/open_claw_symmetry.rs`) includes connector data:

- **Pull intervals**: exported with slot number (1-indexed) and free pivot elevation angle at each end; endpoints are pivot pin positions
- **Link intervals**: axial and radial links exported as separate rows for structural analysis
- **FEA intervals**: simplified push/pull elements where all connections at a joint converge at the highest ring center

The header comments report the connector parameters, a pivot-elevation summary (min/mean/max), and arm clearance statistics.

Format: `Index,Role,Length(m),Strain,AlphaX,AlphaY,AlphaZ,AlphaJoint,AlphaSlot,AlphaAngle,OmegaX,...`

## UI Display

When attachment points are visible, picking a cable shows its slot assignments:
- Example: `"Cable 152:3-48:5"` means the cable connects to slot 3 of strut 152's end and slot 5 of strut 48's end
- The free pivot elevation angles at each end are shown as `Pivot: α: …, ω: …`

## Key Source Files

| File | Purpose |
|------|---------|
| `src/connector/dimensions.rs` | `ConnectorDimensions`, `ring_center_offset()`, `ring_center()`, `pivot_geometry()` |
| `src/connector/attachment.rs` | `PullConnections`, attachment points, slot-assignment optimiser |
| `src/connector/system.rs` | `ConnectorSystem` (dims + per-push slot assignments) |
| `src/wgpu/connector_renderer.rs` | 3D rendering of connector links |
| `src/open_claw_symmetry.rs` | OpenClaw threefold-symmetry enforcement + CSV export |
| `src/camera.rs` | Picking logic for slot and angle display |
