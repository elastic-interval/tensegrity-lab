# Open Claw Truss Tower — Blender Model Specification

**Purpose:** Generate idealized aluminum truss tower geometry for Blender visualization of Open Claw structure support system.

## Physical Specifications

### Dimensions
- **Height:** 2.9m (3m with concrete base) — structure reaches 7.1m total, towers bring to ~10m
- **Base footprint:** Triangular arrangement (equilateral triangle)
- **Tower spacing:** 6m center-to-center between 3 towers
- **Individual tower base width:** ~1.0–1.5m (square or circular footprint)
- **Aluminum tube diameter:** ~50mm OD (typical stage truss)
- **Concrete ballast block:** 1.5m × 1.5m × 0.3m (or ~500kg equivalent)

### Structure
- **Frame type:** Triangular lattice aluminum truss
- **Levels:** 4 horizontal bracing levels
  - Base level (ground + concrete block, Z=0.3m)
  - Mid-lower level (Z=1.0m)
  - Mid-upper level (Z=2.0m)
  - Top level (Z=2.9m)
- **Vertical struts:** 3 main corner posts (triangular arrangement)
- **Internal bracing:** Diagonal cross-bracing between levels for structural appearance
- **Connections:** Simple junction points (visualization only, not detailed bolts/welds)

### Materials & Colors
- **Aluminum tubes:** Silver/chrome (#C0C0C0 or similar)
- **Concrete base blocks:** Dark gray (#606060 or similar)
- **Transparency:** Opaque (no transparency needed)

## Positioning

### Tower Arrangement
- **Number of towers:** 3 (supporting triangular base of structure)
- **Pattern:** Equilateral triangle
- **Spacing:** 6m center-to-center (all 3 pairs)
- **Orientation:** One corner pointing toward [0, Y+] direction (align with structure apex)

### Coordinate System
- **Ground plane:** Z = 0
- **Concrete blocks:** Z = 0 to 0.3m
- **Tower base:** Z = 0.3m
- **Tower top:** Z = 3.0m (including base)
- **Concrete block placement:** Centered under each tower

## Blender Import Requirements

### Output Format Options
1. **Python Blender Script** (`.py` file)
   - Runs natively in Blender (scripting console or File > Open Script)
   - Creates native Blender geometry (meshes, materials)
   - Easy to modify/tweak
   - **Recommended**

2. **glTF/glb File** (`.glb`)
   - Standard 3D format
   - Import: File > Import > glTF 2.0
   - Less flexible but portable

3. **OBJ File** (`.obj`)
   - Simple format, widely supported
   - Limited material/texture support

### Blender Scene Integration
- Position towers in world space
- Each tower should be a separate object (for easy manipulation)
- Use reasonable polygon count (idealized = low detail acceptable)
- Materials should be simple (diffuse color, no complex shaders)

## Visual Style

This is an **idealized rendering**, not a technical CAD model. Acceptable characteristics:
- Simplified lattice geometry (fewer segments than real truss)
- Clean, geometric appearance
- Approximate proportions (not exact measurements)
- Focus on visual plausibility from distance
- Aluminum tower + concrete base clearly distinguishable

## Use Case

Rendering Open Claw structure in Blender with realistic truss tower support. The structure alone (7.1m) sits on top of these towers, bringing total height to ~10m. Towers are visible in full-scene renders and help convey scale/stability.

---

**Generated for:** Open Claw project  
**Date:** April 2026  
**Status:** Specification ready for Blender model generation
