# Implicit Strut Forces for Pre-Cable Arrangement

## Problem

When constructing a tensegrity structure from push intervals (struts), the struts need to:

1. **Not touch** each other anywhere along their lengths
2. **Maintain separation** — each strut suspended at a distance from all others
3. **Tend toward mutual perpendicularity** — nearby struts should be roughly perpendicular
4. **Occupy 3D space** — not collapse to a plane

The challenge: how do struts "find" good positions in space before any pull intervals (cables) are added?

## Core Insight: Implicit Mutual Forces Between Struts

Perpendicularity and non-crossing are not artifacts of an external scaffold network — they are **intrinsic properties of push intervals**. Struts in close proximity naturally want to:

- **Repel** when too close (proximity repulsion)
- **Torque toward perpendicularity** when nearby (perpendicularity torque)

These implicit forces are:
- **Local** — only active within a sphere of influence around each strut
- **Distance-weighted** — stronger when struts are close, fading with distance
- **Temporary** — active only during the zero-gravity arrangement phase, disabled once cables are installed

This reframes the problem: instead of building an explicit scaffold network to guide struts, we recognize that struts themselves carry the information needed to arrange properly. The question becomes how to implement these implicit forces.

### Sphere of Influence

Each strut endpoint defines a **sphere of influence** with radius equal to the strut length. A strut only exerts implicit forces on another strut when their spheres overlap — when any endpoint of one strut falls within the sphere of influence of any endpoint of the other.

This locality model means:
- Distant struts ignore each other entirely (no O(n^2) global coupling)
- As struts separate, forces fade naturally
- The interaction range scales with strut size (longer struts have wider influence)

### Perpendicularity Torque

Two nearby struts of equal length have lowest energy when perpendicular. This can be understood geometrically: perpendicular struts maximize the uniformity of distances between their endpoints. Parallel struts at the same centroid distance produce two short and two long endpoint-to-endpoint distances, which is always higher energy than four equal distances (by Jensen's inequality on any convex potential).

This is not an explicit "make perpendicular" force — it emerges from the geometry of symmetric distance-based interactions between endpoints.

### Proximity Repulsion

When struts are too close (line segments approaching intersection or contact), a repulsive force pushes them apart. This prevents crossing and maintains the physical constraint that compression members cannot occupy the same space.

### 3D Emergence

If all struts start perfectly coplanar, symmetric forces remain in-plane. In practice:
- Initialize strut endpoints with small perturbations in all three dimensions
- The implicit forces amplify any 3D tendency — once struts are slightly out of plane, perpendicularity torque drives further separation into the third dimension
- Verify the result: endpoints must not be coplanar

## Implementation Approaches

### Approach A: Physics-Level Forces (Preferred Conceptual Model)

Implement the implicit forces directly in the physics iteration:

1. Each frame, identify strut pairs within each other's sphere of influence
2. For each interacting pair, compute:
   - **Repulsion**: force proportional to inverse distance between the line segments
   - **Perpendicularity torque**: force derived from the energy gradient of endpoint distance uniformity
3. Apply forces to the relevant joint endpoints
4. Forces are only computed during the arrangement phase

**Advantages**: Clean conceptual model, forces are truly implicit, no extra intervals to manage.

**Challenges**: Requires line-segment distance computation, torque calculation is more complex than spring forces.

### Approach B: Scaffold Springs (Simpler, Uses Existing Infrastructure)

Implement the implicit forces via temporary **spring intervals** (`Role::Springy`) between endpoints of nearby strut pairs. The springs approximate the implicit forces using existing interval infrastructure.

For two struts A-B and C-D within each other's sphere of influence, create **four** scaffold springs: AC, AD, BC, BD.

Each scaffold spring:
- **Role**: `Springy` (bidirectional — never goes slack)
- **Material**: `Spring` (spring constant 9e4 N/m at 1m, about 200,000x softer than Push at 2e10)
- **Stiffness**: Further reduced via the `Percent` field (e.g., 20%) for very subtle forces
- **Ideal length**: `ratio x avg_strut_length` where ratio is tunable (must be > 0.707, start with 0.8)

**Why this approximates the implicit forces:**
- Four equal-ideal-length springs between endpoint pairs create an energy landscape whose minimum is at perpendicular orientation (the perpendicularity torque)
- Springs shorter than ideal push apart (proximity repulsion)
- Springs longer than ideal pull together (prevents unbounded expansion)
- The spring network is a discrete approximation of the continuous implicit force field

**The critical ratio constraint**: For two equal perpendicular struts with coinciding centroids, all four springs have length `sqrt(0.5) x L ~ 0.707L`. The ideal length must exceed 0.707L for repulsion to occur in the perpendicular configuration.

| ratio | behavior |
|-------|----------|
| 0.6   | Fails — always attractive for perpendicular struts |
| 0.7   | Borderline — minimal repulsion |
| 0.8   | Good — moderate clearance |
| 0.9   | Generous spacing |

**Equal-length centering**: With equal ideal lengths, the system naturally centers external joints on the perpendicular bisector of each strut, preventing clustering near endpoints.

**Advantages**: Uses existing interval/physics infrastructure, easier to implement and debug, springs are visible in the renderer.

**Challenges**: Creates many temporary intervals, approximation rather than exact implicit forces.

## Pipeline

### Phase 1: Strut Placement
1. Place struts at initial positions (random or semi-random, with 3D perturbation)
2. No cables, no gravity — struts only

### Phase 2: Implicit Force Settling
1. Activate implicit strut forces (either physics-level or scaffold springs)
2. Let the system settle using CONSTRUCTION physics (smooth, no abrupt movements)
3. Struts find separated, roughly perpendicular, 3D positions
4. Verify: struts are not crossing, not coplanar, roughly perpendicular

### Phase 3: Cable Installation (Convex Hull)
1. Compute the convex hull of all strut endpoints
2. Install pull intervals along convex hull edges
3. Optionally add interior cables based on proximity or triangulation
4. Cables are poised to maintain the structure

### Phase 4: Disable Implicit Forces
1. Remove scaffold springs (Approach B) or disable implicit force computation (Approach A)
2. Let the structure settle under cable forces alone
3. Evaluate: did the struts maintain separation, perpendicularity, and 3D structure?

### Phase 5 (Future): Evolution
1. Use the evolution framework to discover optimal cable subsets
2. Genome encodes which candidate cables to install (bitmask)
3. Fitness function evaluates: strut separation + perpendicularity + structural stability
4. Incremental growth: upon success with n struts, add more

## Key Quantities

- **Strut separation**: Minimum distance between two struts as line segments in 3D (not just endpoint distance)
- **Parallelism**: `|dot(unit_dir_1, unit_dir_2)|` — 0.0 is perpendicular, 1.0 is parallel
- **Spatial thickness**: Extent of the endpoint cloud in the thinnest dimension (0 = coplanar)
- **Sphere of influence radius**: Strut length, centered on each endpoint
- **Fabric time**: All settling and convergence measured in fabric age (seconds at 50us per iteration), never iteration counts

## Open Questions

- **Which implementation approach?** Physics-level forces (cleaner) vs. scaffold springs (simpler)? Start with springs, graduate to physics-level if needed?
- **Sphere of influence sizing**: Is strut length the right radius, or should it be a multiple (e.g., 1.5x)?
- **What scaffold stiffness** gives the best balance between strong guidance and smooth settling? (Approach B)
- **Convex hull sufficiency**: Are convex hull cables enough, or are interior cables needed for stability?
- **How much pretension** (cable ideal / actual distance ratio) keeps the structure intact after implicit force removal?
- **Packing frustration**: Does the perpendicularity tendency hold with many struts, or does packing produce different geometries?
- **Can cables alone maintain the structure**, or is some permanent implicit force needed?
- **Incremental growth**: When adding struts, should existing cables be preserved or re-evaluated?
