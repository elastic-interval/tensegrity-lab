# Connectors: How Cables Attach to Struts

> **2026-10: prototype dimensions in code.** The part described below (32 mm
> moment arm, 20 mm tube, 10 mm pin, 1 mm washers) is the first design and is
> now out of date. The code uses the Headless Hug prototype: POM washers
> 2 mm, pivot radius `R_tube` 30 mm (cap OD 40/2 + 1 clearance + tube 18/2),
> cross-tube 18 × 2.5 mm precision tube 13.5 mm long, 12 mm clevis pin in a
> 13 mm bore, jaw clearance 0.25 mm (jaw gap 14 mm). See
> `src/connector/dimensions.rs` and `src/connector/fork.rs`. Open Claw's
> connector is frozen on its own branch.

## Bridle Nodes: Node Plates and Hangers

Where a strut end would carry too many cables, or two cables leave it in
nearly the same direction, a **bridle** (`Fabric::bridle`, DSL `.bridle`)
takes two of them off: cables J–A and J–B become a link J–N and two cables
N–A and N–B, meeting at a free node N. Three tension lines meet at N.

- **Node plate** (`src/connector/node_plate.rs`) — a triangular steel plate
  at N with a hole for each of its three cables. Three forces holding a point
  are coplanar, so the plate lies square to its cables, each hole on its
  cable's line through N. Each cable is a whole cable with a jaw terminal at
  both ends; at N its terminal straddles the plate and pins through its hole.
- **Equilateral, one part.** Every plate is the same equilateral triangle,
  `PLATE_SIDE` (60 mm) hole to hole, with `PLATE_CORNER_RADIUS` (16 mm) of
  steel round each hole and `PLATE_THICKNESS` (12 mm) fitting the
  terminals' 14 mm jaw gap. Its corners sit on the three cables' lines, so
  where the lines meet (N) is the triangle's centre only when the cables
  are 120° apart. Two cables spreading to far struts can meet beyond the
  plate, on the third cable's side, with N outside the triangle. 60 mm is the largest side tried that leaves Headless Hug
  no cable too short; at 65 and 70 mm two were.
- **Always hung on a hanger** (`HANGER_LENGTH`, 82 mm pin to pin), a short fork
  rod from J's cross-tube, close to J, so the bridle keeps the forces much as
  the two cables it replaces had them. One rod fork pins through the
  cross-tube, the other through the plate's hole. Each is a U with the threaded
  rod through its web, a load nut inside and a lock nut outside. The two pins
  swing against each other by up to 2.9° as the figure moves (docs/parts.md), which
  only the play at the pins takes up. It was a jaw–jaw swivel until 2026-11-02, so that all
  tension is cable or threaded rod. (Setting nodes out on cable pendants instead
  made the arms sag 1.5 m and left a dozen cables slack.)
  `Fabric::bridle_hangers` records the hangers.
- **Hung a hanger's length out.** The node starts `BRIDLE_LINK` (100 mm)
  from J; the plate moves to hang its hole a hanger's length from the
  pivot, and its link is shortened to match as the shape falls and
  settles (`Fabric::hang_bridles`). A plate with no equilateral fit is
  flagged.
- **Fork rods for short cables.** A cable from a plate too short to
  make as a cable (`min_cable_length`, 262 mm pin to pin) is a fork rod
  instead: threaded rod cut to length between two rod forks, as for a hanger.
  They are chosen when the hardware is committed
  (`ConnectorSystem::link_length`); one under `FORK_ROD_MIN_LENGTH`, 81 mm,
  can't be made and is flagged short. Short cables
  elsewhere stay flagged. (Bridle clamps, one continuous cable over a
  saddle, were tried first and dropped on 2026-10-29: on a nearly
  straight cable they couldn't line up with their swivel.)
- **Pretensioned** (`Fabric::tighten_bridles`, after `slacken`), as the
  struts are, so a free node can't drift until one of its cables hangs
  slack.
- **Checked like any connector**: every cable pin to pin against the
  minimum length, every terminal, fork and plate against
  collisions.

All plate, fork and placement dimensions are estimates, in
`src/connector/fork.rs`.

## Physical Reality

In a tensegrity structure, every cable (pull interval) terminates at the end of a strut (push interval). The physical connection hardware at each strut end is called a **connector**. Each strut end can have multiple cables attached, each at a distinct **slot** along the strut axis.

The connector part itself — specified in full under [The Fabricated Part](#the-fabricated-part) below — is a **flat steel ring** that turns on an axial bolt at the strut end, carrying a **radial boss** that ends in a **cross-tube**. The cable's fork terminal is pinned through the tube.

The assembly at a strut end consists of:

1. **Bolt** — Axial, anchored in the strut's end; there is no end cap (the original design had one)
2. **Rings** — One connector per cable, stacked on the bolt and retained by a nut
3. **Washers** — A thin POM divider washer between the tube end and first ring and between adjacent rings, so steel never bears on steel
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

### Materials

Real materials, for accurate rendering (and fabrication). **In hand / real (bought):** the fork terminal, its clevis pin, and the cable. **Connector itself:** not yet built, so its material is a choice — **default = hot-dip galvanized steel** (as the existing caps); stainless or aluminium if the redesign chases weight (see Alternative construction below).

Render values are approximate PBR — **linear base-colour RGB · metallic · roughness** — enough for believable metal, not measured data.

| Part | Real material | Look | RGB · metallic · roughness |
|---|---|---|---|
| **Connector — default** | Hot-dip **galvanized steel** (S235/S420) | matte cool grey, faint spangle | (0.60, 0.62, 0.64) · 1.0 · **0.55** |
| Connector — alt. stainless | AISI-316 | brighter satin | (0.56, 0.57, 0.58) · 1.0 · 0.30 |
| Connector — alt. aluminium | marine Al (5052 / 6082) | light matte grey | (0.62, 0.62, 0.63) · 1.0 · 0.40 |
| **Clevis pin** (in the bore) — *real* | AISI-316 stainless | bright satin | (0.56, 0.57, 0.58) · 1.0 · 0.28 |
| **Fork terminal** (mates on the pin) — *real* | AISI-316 stainless | bright satin | (0.56, 0.57, 0.58) · 1.0 · 0.28 |
| **Cable** — *real* | Galvanized steel, 7×19 | matte grey, stranded | (0.42, 0.44, 0.46) · 1.0 · 0.60 |

- **Galvanized ≠ chrome** — render it matte (roughness ~0.55), cool grey, or it reads as polished steel.
- The stainless fork + pin are **brighter/shinier** than the galvanized connector — a real, visible material contrast worth showing if you render the joint.
- Going stainless or aluminium for the connector just swaps its row.
- The in-app WGPU renderer is diffuse-only, so it approximates this contrast by lifting the stainless base colour above the galvanized one (`connector_renderer.rs`); the PBR values above are for Blender/Cycles renders.

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

# ---- material: hot-dip galvanized steel (the default build — see the Materials table) ----
steel = bpy.data.materials.new("Galvanized steel")
steel.use_nodes = True
b = steel.node_tree.nodes["Principled BSDF"]
b.inputs["Base Color"].default_value = (0.60, 0.62, 0.64, 1.0)
b.inputs["Metallic"].default_value  = 1.0
b.inputs["Roughness"].default_value = 0.55   # galvanized is matte, not chrome
part.data.materials.append(steel)
# swaps: stainless -> Roughness 0.30 ; aluminium -> Base Color (0.62,0.62,0.63,1), Roughness 0.40

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

### Alternative construction — single-piece formed eye (no weld)

*The spec above — laser-cut disc-with-boss + a separately **welded** steel tube — stays the **default**. This is an alternative to weigh with Peter / a fabricator.*

**The idea (ring–bridge–ring).** Laser-cut one flat blank — **ring — bridge — ring**, each ring at **half thickness** (~2.5 mm). Curl the bridge ~360° around a ~12 mm mandrel so the two rings fold into coincidence and restack to the full ring; the curled bridge becomes the pin eye, replacing the welded tube. One continuous bent piece — **no weld**. It's the same forming used for **hinge knuckles / rolled eyes**.

**Why consider it:** no welding (labour, distortion, and the weld-as-strength-limiter all gone); better fatigue life for a touring piece (continuous parent metal, no weld toe); at ~360 identical parts a forming die likely beats welding on cost; and it opens up materials (below).

**Make-or-break — the seam.** An unwelded curl can spring *open* under load. Keep it shut by (a) orienting the eye so the cable pull **closes** the seam, and (b) letting the **central M12 bolt clamp the two coincident rings** so the curl can't unroll. A single tack weld is cheap insurance. Without reliable closure it's much weaker than a closed tube — this is the crux, not the wall thickness.

**Materials it opens up** (forming isn't limited to weldable steel, and skips galvanizing):
- **Stainless 316/304** — very formable, steel-class strength, **no galvanizing step** (and matches the AISI-316 fork terminals). Cleanest swap.
- **Aluminium (marine 5052 / 6082)** — the **weight** option (~⅓ the density of steel). The ring/boss is low-stress, so it takes the density saving nearly for free; press a **stainless bushing** into the eye for pin bearing + galvanic isolation. Rough order **~15–20 kg** off the whole structure across ~360 parts (Peter to confirm). **Best if weight is the goal.**
- **Titanium** — light + strong + no coating, but cost likely rules it out at this quantity.
- *(Not the load path: nylon/POM would creep under the sustained cable pretension.)*
- Connectors bolt on independently, so they need **not** match the galvanized-steel caps — a drop-in material swap on just this one part. Shrinking the ring OD (Ø40 is oversized) trims weight **and** the moment arm in any material.

**How to order it — two routes:**
1. **Turnkey formed parts** — a sheet-metal-forming / hinge specialist builds a die from your flat-blank drawing + final bent geometry + tolerances and delivers finished parts. Best at full quantity; they own the springback and roundness.
2. **Flat "ring-bridge-ring" blanks, bend in-house** — order just the laser-cut flats (cheap, same route as the current discs) and curl them yourself over a 12 mm mandrel in a simple jig. Lowest outlay, ideal for prototyping; the catch is repeatable closure/roundness without a proper die, so the jig must **over-bend** to land the two rings coincident.

**Verify before committing:** min bend radius vs grade + thickness (cracking); springback → jig or die; eye roundness for the pin fit; and the eye's capacity at the **~8–9 kN** load (thinner wall than the welded tube). Prototype one and load-test it.

## Geometry and Dimensions

All physical dimensions are defined in `ConnectorDimensions` (`src/connector/dimensions.rs`).
Defaults reflect the values in `Default::default()` and are the source of truth;
this table is informational and can drift between code edits and doc updates.

| Parameter | Default | Description |
|-----------|---------|-------------|
| `ring_thickness` | 5 mm | Ring thickness along the bolt axis (t_ring) |
| `washer_thickness` | 2 mm | POM divider washer between cap/ring and between adjacent rings |
| `cap_thickness` | 0 mm | From the strut end to the first washer: nothing, as there are no strut caps; the first washer sits on the tube end (kept because Open Claw's CSV header reports it) |
| `pivot_radius` | 30 mm | Radial distance from the strut/bolt axis to the pivot pin (R_tube — the cable's moment arm) |

## Slot Positioning Along the Strut Axis

Slots are numbered starting from 0 (code-internal) or 1 (display/CSV). The position of each slot's ring center along the strut axis, measured from the strut endpoint, is calculated by `ConnectorDimensions::ring_center_offset()`:

```
offset = cap_thickness + washer + ring_thickness/2 + slot * (ring_thickness + washer)
```

For 0-indexed slots with the current defaults (cap=0, t_ring=5 mm, washer=2 mm):
- Slot 0: 0 + 2 + 2.5 = 4.5 mm from the strut end
- Slot 1: 11.5 mm
- Slot 2: 18.5 mm

At a **hinge** (two struts pinned where their axes cross, DSL `.hinge`) the
strut ends at the hinge pin, so each strut's stack is set back along it
until the top of its highest ring sits `HINGE_LENGTH` (60 mm) below the pin
(`ConnectorSystem::stack_ends`).

The **hinge** itself (`Part::Hinge`, dimensions in `fork.rs`, all estimates
for M12 catalogue parts) is a threaded clevis on one strut and a rod end on
the other, each screwed onto its strut's bolt above the stack as the
stack's nut, joined by a pin square to both struts exactly where their axes
meet. Each strut's compression runs straight down its axis into the pin,
with no lever. Each strut spins freely about its own axis (its rings turn
on the bolt), so the pin can always lie square to both and the hinge only
opens and closes; the rod end's ball takes any small misalignment. On the
parts list one hinge covers clevis, rod end and pin; the rod end is drawn
as its own placement (`Part::HingeEye`) because it turns on the pin.

**No cable ends on a hinge.** The hinge is kept pristine: it carries only
its two struts, so no awkward forces reach the pin. A cable attached to a
strut before the hinge stays on that strut's stack (`Fabric::hinge_strut`).
A cable with no strut of its own there (a bridle's cable, or one that merged
in the hinge) is split by a node plate along it, with a leg from the plate
to the stack on each strut (`Fabric::split_hinge_cables`), so its load
reaches both struts through their stacks. The plate lies where its three
forces balance: in the plane of the two legs' pivots and the far end, on the
line from the far end to the midpoint of the pivots, where its three members
come out about the same length, pin to pin. The simulation only knows the
legs as one strand to the pin, so this place is worked out from the stacks
(`ConnectorSystem::split_plate_center`), not taken from the simulated node. In the simulation the legs
are two parallel intervals from the plate's node to the pin, each recorded
as its strut's. Legs too short to be cables are fork rods, as for any
plate. A pin takes one such cable, as its plate is named after it (`Z5b`),
and a guy line at a hinge is refused; both are reported as failed plan steps.

`ConnectorDimensions::ring_center()` uses `ring_center_offset()` to compute the 3D position of a ring center given a strut endpoint and axis direction. `generate_attachment_points()` (`src/connector/attachment.rs`) produces `ATTACHMENT_POINTS = 10` candidate slots per end from the same formula.

## How Cables Get Assigned to Slots

When the fabric is built and settled, each push interval determines which cables connect to it and at which slot (`ConnectorSystem::update_all_connections`). This is the moment the hardware is **committed**: it runs on entering Viewing and for CSV export, and the slots, part shapes and fork rods it decides then stay put while the fabric animates or the C key is pressed (which only re-checks collisions). The optimiser (`find_optimal_assignment`, `src/connector/attachment.rs`) works per push end:

- **No ordering rules**: every permutation of the cables over the slots is tried, and any cable may take any slot. (An earlier hard rule kept outward-pulling cables off the topmost slot, so the retaining nut would not carry their full axial load; it was dropped to clear collisions, so that nut load needs checking.)
- **Objective**: fewest pairs of forks whose capsule models collide — the same test that marks culprits.
- **Then**: maximise the minimum 3D distance between any pair of connector arms (segments from ring center to pivot pin).
- **Tie-break**: minimum rotational moment about the slot-0 ring center.

For a mirror-symmetric fabric (Headless Hug) the assignment is then made
exactly symmetric, as the real build will be:

- **Symmetric shape first** — `Fabric::symmetrize_mirror` moves each pair
  of mirror twins to the mean of one and the other's reflection, so a
  borderline collision on one side can't pass unnoticed on the other.
- **Mirrored stacks** (`mirror_assignments`) — every stack takes the mirror
  image of its twin's slot order: the order with the fewest colliding forks
  on both stacks together.
- **Custom discs on the mirror plane** (`plane_assignments`) — a strut
  lying in the mirror plane is its own image, so each pair of twin cables
  shares one height on a single laser-cut **custom disc** with a boss for
  each (`Part::CustomDisc`). Bosses may reach out past the standard pivot
  radius (`DISC_REACHES`, 30–70 mm) to clear their forks, and a **spacer**
  of extra washers (`GAP_WASHERS`) can lift a disc clear of the one below.
  Order, gaps and reaches are searched together for the fewest colliding
  forks, then the fewest washers, then the shortest bosses. Each distinct
  disc shape is its own part.

## Pivot Geometry: How Cable Endpoints are Positioned

Once a cable is assigned to a slot, its attachment is calculated by
`ConnectorDimensions::pivot_geometry()`, returning `(pivot_pos, elevation_deg)`:

1. **Ring center**: position along the strut axis at this slot
2. **Radial direction**: perpendicular to the strut axis, pointing toward the cable's far end (the ring turns to face it)
3. **Pivot pin position** (`pivot_pos`): ring center + radial direction × `pivot_radius` — this *is* the cable endpoint
4. **Elevation** (`elevation_deg`): the free angle the fork takes toward the far end, `asin(pull_direction · push_axis)` measured from the radial plane (0° = radial, positive tilts outward along the strut axis)

The elevation is purely informational — nothing is manufactured to an angle; it is reported so articulation limits of the fork can be checked.

## Rendering

The hardware is drawn from the **parts list** (`src/connector/parts.rs`):
`placements(fabric)` places every copy of every `Part` (ring with boss and
cross-tube, custom disc, POM washer or spacer, jaw terminal, rod fork, node
plate, hinge) in its own frame, and `parts_list` counts them as a bill of
materials. A welded cross-tube belongs to its ring or custom disc and is
drawn as part of it. The rod ends of hinges and rod forks' stock nuts
are drawn on their own but left off the list (`Part::listed`). A fork rod's
threaded rod is drawn and reported like a cable, so only its forks are parts.
`ConnectorRenderer` (`src/wgpu/connector_renderer.rs`) builds one mesh per
part (`src/wgpu/part_meshes.rs`, in metres) and instances it at every
placement; a new kind of hardware is a new `Part` variant. Stainless parts
are drawn brighter than galvanized ones, POM washers off-white, and every
part of a colliding assembly red. The cable itself (`cylinder_renderer.rs`,
at the true 6 mm diameter) ends buried in its terminal's barrel, aimed at
the far end's *pivot*, so terminal and cable stay collinear; a fork rod's
threaded rod is drawn the same way, ending inside its fork. Nothing is drawn unless the attachment
points are shown (C key).

The **parts view** (P key, `src/wgpu/parts_view.rs`) takes over the window: every
distinct part with a letter and its count, turning in a strip down the left, and
the one clicked on large beside them.

### Collision culprits

The fork/tube dimensions and a capsule model of each assembled cable end live in `src/connector/fork.rs`. `ConnectorSystem::mark_culprits` (run when the hardware is committed and again when the C key shows it, never per frame) does a pairwise capsule scan and records every cable end whose assembly would physically intersect another in `ConnectorSystem::culprits`. The renderer paints all parts of a culprit assembly red — these are real construction issues to resolve, not rendering artifacts.

## CSV Export

Open Claw's CSV export (`src/open_claw_symmetry.rs`) includes connector data; Headless Hug's geometry-only CSV (`src/engineering_csv.rs`) does not (see `docs/csv-handoff.md`):

- **Pull intervals**: exported with slot number (1-indexed) and free pivot elevation angle at each end; endpoints are pivot pin positions
- **Link intervals**: axial and radial links exported as separate rows for structural analysis
- **FEA intervals**: simplified push/pull elements where all connections at a joint converge at the highest ring center

The header comments report the connector parameters, a pivot-elevation summary (min/mean/max), and arm clearance statistics.

Format: `Index,Role,Length(m),Strain,AlphaX,AlphaY,AlphaZ,AlphaJoint,AlphaSlot,AlphaAngle,OmegaX,...`

## UI Display

When attachment points are visible, picking a cable shows each end as
`<joint>:<slot>`, e.g. `L12:2` for slot 2 of the stack at `L12`, and the
free pivot elevation angle at each end on a `Pivot:` line.

## Key Source Files

| File | Purpose |
|------|---------|
| `src/connector/dimensions.rs` | `ConnectorDimensions`, `ring_center_offset()`, `ring_center()`, `pivot_geometry()` |
| `src/connector/attachment.rs` | `PullConnections`, attachment points, slot-assignment optimiser |
| `src/connector/system.rs` | `ConnectorSystem`: slot assignments, mirrored and plane stacks, node plates, fork rods, culprits |
| `src/connector/fork.rs` | Terminal, rod fork and plate dimensions; collision capsules |
| `src/connector/node_plate.rs` | `NodePlate`: a bridle node's equilateral plate and its holes |
| `src/connector/parts.rs` | The parts list: every distinct part and where each copy sits |
| `src/fabric/bridle.rs` | `Fabric::bridle`, `tighten_bridles`, `hang_bridles` |
| `src/fabric/symmetrize.rs` | `Fabric::symmetrize_mirror` |
| `src/wgpu/connector_renderer.rs`, `part_meshes.rs`, `parts_view.rs` | Drawing the parts and the parts view |
| `src/open_claw_symmetry.rs` | OpenClaw threefold-symmetry enforcement + CSV export |
| `src/camera.rs` | Picking logic for slot and angle display |
