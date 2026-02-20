# Scaffold Intervals for Strut Spacing

## Problem

When constructing a tensegrity structure from push intervals (struts), the struts need to:

1. **Not touch** each other anywhere along their lengths
2. **Maintain separation** — each strut suspended at a distance from all others
3. **Tend toward mutual perpendicularity** — nearby struts should be roughly perpendicular
4. **Occupy 3D space** — not collapse to a plane

The challenge: how do struts "find" good positions in space before any pull intervals (cables) are added?

## Approach: Scaffold Springs

Temporary **spring intervals** (`Role::Springy`) are placed between the endpoints of every pair of struts. These springs are bidirectional (push when compressed, pull when stretched) and very soft compared to the struts themselves. After the struts have settled into good positions guided by the scaffold, permanent cables are added and the scaffold is removed.

### Setup

For two struts A-B and C-D, create **four** scaffold springs: AC, AD, BC, BD.

Each scaffold spring:
- **Role**: `Springy` (bidirectional — never goes slack)
- **Material**: `Spring` (spring constant 9e4 N/m at 1m, about 200,000x softer than Push at 2e10)
- **Stiffness**: Further reduced via the `Percent` field (e.g., 20%) for very subtle forces
- **Ideal length**: `ratio x avg_strut_length` where ratio is tunable (must be > 0.707, start with 0.8)

For n struts, there are n(n-1)/2 pairs, each producing 4 scaffold springs.

### The Ellipsoid Analogy

The original intuition: for strut A-B (length L) and an external joint C, the two scaffold springs A-C and B-C define something like an ellipsoid with A and B as foci. If the total path AC + BC exceeds the strut length by some margin, then C is kept outside the exclusion zone around the strut.

**What springs actually create is different from a true ellipsoid** — and in some ways better. A true ellipsoid allows C to get very close to endpoint A (as long as BC compensates). Two springs with individual ideal lengths don't allow this: if C approaches A, the A-C spring fires a strong repulsive force regardless of what B-C is doing. This gives more robust protection at strut endpoints.

The minimum-energy locus for the spring pair is a **circle** on the perpendicular bisector plane of the strut. Points inside this circle get pushed outward; points far outside get pulled inward. This creates a gentle basin around each strut.

### Why Perpendicularity Emerges

Two perpendicular struts of equal length produce four scaffold springs of **equal** length. Two parallel struts at the same distance produce two short and two long springs. By Jensen's inequality on the convex spring potential `(x - ideal)^2`, equal lengths always produce lower total energy than non-uniform lengths. The scaffold naturally creates an energetic preference for perpendicular orientations.

This is not an explicit perpendicularity mechanism — it falls out of the spring geometry.

### The Critical Ratio Constraint

For two equal perpendicular struts of length L with coinciding centroids, all four scaffold springs have length `sqrt(0.5) x L ~ 0.707L`. If the ideal length is **less** than 0.707L, these springs are in tension (pulling the struts together), providing no repulsion. The struts would be pulled through each other.

**The scaffold ideal must exceed 0.707L for any repulsion to occur in the perpendicular case.**

| ratio | behavior |
|-------|----------|
| 0.6   | Fails — always attractive for perpendicular struts |
| 0.7   | Borderline — minimal repulsion |
| 0.8   | Good — moderate clearance |
| 0.9   | Generous spacing |

### Equal-Length Centering

With equal ideal lengths for springs A-C and B-C, the system naturally centers C on the perpendicular bisector of A-B (where AC = BC). If C drifts toward A: the A-C spring compresses (pushes away from A) while B-C stretches (pulls toward B). Both forces restore C to the equatorial region. This prevents joints of other struts from clustering near strut endpoints.

### 3D Behavior

If all struts and springs start perfectly coplanar, all forces remain in-plane. The scaffold cannot break planar symmetry on its own. In practice:
- Initialize strut endpoints with small perturbations in all three dimensions
- The scaffold amplifies any 3D tendency — once struts are slightly out of plane, the spring forces drive further separation into the third dimension
- The result should be verified: endpoints must not be coplanar

## Pipeline (Planned)

### Phase 1: Scaffold Settling
1. Place struts at initial positions (random or semi-random, with 3D extent)
2. Create scaffold springs between all strut pairs
3. Let the system settle using CONSTRUCTION physics (smooth, no abrupt movements)
4. Struts find separated, roughly perpendicular, 3D positions

### Phase 2: Cable Installation
1. Identify candidate cable positions (e.g., all cross-strut endpoint pairs, triangulation, or proximity graph)
2. Install pull intervals with pretension (ideal length shorter than current distance)
3. The cables are poised to maintain the structure

### Phase 3: Scaffold Removal
1. Remove all scaffold spring intervals
2. Let the structure settle under cable forces alone
3. Evaluate: did the struts maintain separation, perpendicularity, and 3D structure?

### Phase 4 (Future): Evolution
1. Use the evolution framework to discover optimal cable subsets
2. Genome encodes which candidate cables to install (bitmask)
3. Fitness function evaluates: strut separation + perpendicularity + structural stability
4. Incremental growth: upon success with n struts, add more

## Key Quantities

- **Strut separation**: Minimum distance between two struts as line segments in 3D (not just endpoint distance)
- **Parallelism**: `|dot(unit_dir_1, unit_dir_2)|` — 0.0 is perpendicular, 1.0 is parallel
- **Spatial thickness**: Extent of the endpoint cloud in the thinnest dimension (0 = coplanar)
- **Fabric time**: All settling and convergence measured in fabric age (seconds at 50us per iteration), never iteration counts

## Open Questions

- What scaffold stiffness gives the best balance between strong guidance and smooth settling?
- How much pretension (cable ideal / actual distance ratio) keeps the structure intact after scaffold removal?
- Does the scaffold's perpendicularity tendency hold with more than 2-3 struts, or does packing frustration produce different geometries?
- Can cables alone maintain the structure, or is some permanent scaffold needed?
- When adding struts incrementally, should existing cables be preserved or re-evaluated?
