# Connectors: How Cables Attach to Struts

## Physical Reality

In a tensegrity structure, every cable (pull interval) terminates at the end of a strut (push interval). The physical connection hardware at each strut end is called a **connector**. Each strut end can have multiple cables attached, each at a distinct **slot** along the strut axis.

The physical connector assembly at a strut end consists of:

1. **Cap** — A fixed end-cap on the strut tube
2. **Discs** — Rotating ring-shaped connectors stacked along the strut axis, separated by thin spacers
3. **Hinges** — Arms that extend radially outward from each disc, with a hole at the tip where the cable attaches
4. **Separators** — Thin spacers between cap and first disc, and between adjacent discs

Each disc can rotate freely around the strut axis, allowing its hinge to point in any radial direction. This means the cable naturally finds its preferred angle around the strut.

## Geometry and Dimensions

All physical dimensions are defined in `HingeDimensions` (`src/fabric/mod.rs:35-43`):

| Parameter | Default | Description |
|-----------|---------|-------------|
| `push_radius` | 30mm | Radius of the strut tube |
| `push_radius_margin` | 3mm | Gap between tube surface and disc center |
| `disc_thickness` | 10mm | Thickness of each connector disc |
| `disc_separator_thickness` | 3mm | Spacer between adjacent discs |
| `cap_thickness` | 6mm | Thickness of the end-cap closing the strut tube |
| `hinge_extension` | 12mm | Length of the hinge arm beyond the disc edge |
| `hinge_hole_diameter` | 17mm | Diameter of the hole at the hinge tip |

Derived values:
- **Hinge offset** = `push_radius + push_radius_margin + disc_thickness/2` = 38mm (radial distance from strut axis to hinge center)
- **Hinge length** = `disc_thickness/2 + hinge_extension + hinge_hole_diameter` = 34mm (from hinge center to cable attachment point)

## Slot Positioning Along the Strut Axis

Slots are numbered starting from 0 (code-internal) or 1 (display/CSV). The position of each slot's disc center along the strut axis, measured from the strut endpoint, is calculated by `HingeDimensions::disc_center_offset()`:

```
offset = cap_thickness + separator + disc_thickness/2 + slot * (disc_thickness + separator)
```

The first disc sits on the far side of the **cap** that closes the strut tube, with a separator between cap and disc. After that, each subsequent disc is spaced by the full disc thickness plus a separator.

For 0-indexed slots with the default dimensions (cap=6mm, disc=10mm, separator=3mm):
- Slot 0: 6 + 3 + 5 = 14mm from the strut end
- Slot 1: 6 + 3 + 5 + 13 = 27mm from the strut end
- Slot 2: 6 + 3 + 5 + 26 = 40mm from the strut end

`FabricDimensions::ring_center()` uses `disc_center_offset()` to compute the 3D position of a disc center given a strut endpoint and axis direction.

## Attachment Points and Ring Centers

Both attachment points and ring centers use the same `disc_center_offset()` formula to position along the strut axis:

1. **Ring centers** — Used by `hinge_geometry()` for rendering and CSV export. Represent the axial center of each disc at slots 0, 1, 2.

2. **Attachment points** — Used by the moment optimization algorithm in `generate_attachment_points()` (`src/fabric/attachment.rs`). There are 10 of these per end (constant `ATTACHMENT_POINTS = 10`), representing candidate positions for cable assignment. The first 3 match the ring center positions exactly.

## How Cables Get Assigned to Slots

When the fabric is built and settled, each push interval determines which cables connect to it and at which slot. This happens in `PullConnections::reorder_connections()` (`src/fabric/attachment.rs:122-205`):

### Step 1: Identify Connections

For each push interval, find all pull intervals (cables) that share a joint with it. Each such cable connects to one end (alpha or omega) of the push interval.

### Step 2: Separate Outward vs Inward Pulls

Cables are classified by their pull direction relative to the strut axis (`is_outward_pulling()`, `src/fabric/attachment.rs:387-407`):

- **Outward-pulling**: Cable pulls along the strut axis (away from the structure). These are forced to slot 0 (innermost), because they need the strongest mechanical advantage.
- **Inward-pulling**: Cable pulls at an angle back toward the structure. These are optimized across the remaining slots.

### Step 3: Minimize Rotational Moment

The inward-pulling cables are assigned to slots using Heap's permutation algorithm (`find_optimal_assignment()`, `src/fabric/attachment.rs:412-517`). For each permutation of cable-to-slot assignments, the total rotational moment about the first attachment point (acting as a ball joint pivot) is calculated. The assignment with minimum total moment wins.

The moment calculation (`calculate_rotational_moment()`, `src/fabric/attachment.rs:329-383`) computes:
```
total_moment = sum of (moment_arm x force) for each cable
```
where `moment_arm` is the vector from the pivot (slot 0) to the attachment point, and `force` is `strain * pull_direction`.

## Hinge Geometry: How Cable Endpoints are Positioned

Once a cable is assigned to a slot, its exact 3D attachment position is calculated by `FabricDimensions::hinge_geometry()` (`src/fabric/mod.rs:152-174`):

1. **Ring center**: Position along the strut axis at this slot
2. **Radial direction**: Perpendicular to the strut axis, pointing toward the cable's far end
3. **Hinge position**: Ring center + radial direction * hinge_offset (38mm out from axis)
4. **Ideal angle**: Calculated from the pull direction relative to the strut axis
5. **Snapped angle**: Quantized to one of 5 allowed `HingeBend` values: -60, -30, 0, +30, +60 degrees
6. **Cable endpoint**: Hinge position + hinge arm rotated by the snapped angle

The 5 discrete hinge angles (`HingeBend` enum, `src/fabric/attachment.rs:14-74`) represent physical hinge positions that can be set during construction. The hinge arm rotates in the plane defined by the radial direction and the strut axis.

## Rendering

The connector assembly is rendered by `HingeRenderer` (`src/wgpu/hinge_renderer.rs`) as three types of colored links:

| Link Type | Color | Connects |
|-----------|-------|----------|
| **Axial** | Yellow | Previous ring center (or strut end) to current ring center |
| **Radial** | Orange | Ring center to hinge position |
| **Hinge** | Red | Hinge position to cable endpoint |

Rendering is only visible when `show_attachment_points()` is enabled in the render style.

## CSV Export

The CSV export (`src/fabric/csv_export.rs`) includes connector data:

- **Pull intervals**: Exported with slot number (1-indexed) and hinge bend angle at each end
- **Link intervals**: Axial, radial, and hinge links exported as separate rows for structural analysis
- **FEA intervals**: Simplified push/pull elements where all connections at a joint converge at the highest ring center (for finite element analysis)

Format: `Index,Role,Length(m),Strain,AlphaX,AlphaY,AlphaZ,AlphaJoint,AlphaSlot,AlphaAngle,OmegaX,...`

## UI Display

When attachment points are visible, picking a cable shows its slot assignments:
- Example: `"Cable 152:3-48:5"` means the cable connects to slot 3 of strut 152's end and slot 5 of strut 48's end
- Hinge angles are also displayed when available

## Key Source Files

| File | Purpose |
|------|---------|
| `src/fabric/mod.rs:35-174` | `HingeDimensions`, `FabricDimensions`, `disc_center_offset()`, `ring_center()`, `hinge_geometry()` |
| `src/fabric/attachment.rs` | `PullConnections`, attachment points, moment optimization |
| `src/fabric/interval.rs:490-683` | Interval's connection storage and attachment point access |
| `src/wgpu/hinge_renderer.rs` | 3D rendering of connector links |
| `src/fabric/csv_export.rs` | CSV export with hinge positions and angles |
| `src/camera.rs:550-702` | Picking logic for slot and angle display |
