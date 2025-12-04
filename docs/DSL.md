# Tensegrity DSL

The Tensegrity DSL is a Rust-embedded domain-specific language for defining tensegrity bricks and fabrics. It replaces the older S-expression-based Tenscript language.

## Brick Definitions

Bricks are defined using a fluent builder API in `src/build/dsl/brick_library.rs`:

```rust
proto(SingleRightBrick, [Seed, OnSpinRight])
    .pushes(3.204, [(AlphaX, OmegaX), (AlphaY, OmegaY), (AlphaZ, OmegaZ)])
    .pulls(2.0, [(AlphaX, OmegaZ), (AlphaY, OmegaX), (AlphaZ, OmegaY)])
    .face(Spin::Right, [AlphaZ, AlphaY, AlphaX], [
        OnSpinRight.calls_it(Attach(Spin::Right)),
        Seed.calls_it(SingleBot),
        Seed.calls_it(Downwards),
    ])
    .face(Spin::Right, [OmegaX, OmegaY, OmegaZ], [
        OnSpinRight.calls_it(SingleTop),
        OnSpinRight.calls_it(AttachNext),
        Seed.calls_it(SingleTop),
    ])
    .baked()
    .joints([...])
    .pushes([...])
    .pulls([...])
    .build()
```

### Key Concepts

**Prototype Phase:**
- `.proto(name, roles)` - Define brick name and roles it can be used in
- `.pushes(ideal, pairs)` - Grouped compression elements with shared ideal length
- `.pulls(ideal, pairs)` - Grouped tension elements with shared ideal length
- `.face(spin, joints, aliases)` - Define triangular faces with their chirality and role-based names

**Baked Phase:**
- `.baked()` - Switch to defining the settled geometry
- `.joints([...])` - Final 3D positions after physics simulation
- `.pushes([...])` / `.pulls([...])` - Final interval strains
- `.build()` - Construct the complete Brick

### Face Aliases

Faces have multiple names depending on the brick's **role** in the construction:
- `Seed` - When this brick is the first/base brick
- `OnSpinLeft`/`OnSpinRight` - When attaching to a left/right chirality face
- `SeedFourDown`/`SeedFaceDown` - Different seed orientations

Each face can have multiple aliases for different roles:
```rust
OnSpinRight.calls_it(Attach(Spin::Right)),  // Where other bricks attach
Seed.calls_it(SingleBot),                    // Bottom face when seed
Seed.calls_it(Downwards),                    // Orientation marker
```

## Fabric Definitions

Fabrics are defined in `src/build/dsl/fabric_library.rs`:

```rust
fabric("Triped", Mm(11000.0))
    .build(
        branching(OmniSymmetrical, Seed(1))
            .on_face(OmniBotX, growing(8).scale(0.9).mark(End).prism().build())
            .on_face(OmniBotY, growing(8).scale(0.9).mark(End).prism().build())
            .on_face(OmniBotZ, growing(8).scale(0.9).mark(End).prism().build())
            .on_face(OmniTop, growing(1).build())
            .build(),
    )
    .shape([
        during(Sec(25.0), [space(End, 0.38)]),
        during(Sec(15.0), [vulcanize()]),
    ])
    .pretense(pretense(Sec(15.0)).surface(SurfaceCharacter::Frozen))
    .fall(Sec(2.0))
    .settle(Sec(10.0))
    .scale(Mm(1030.0))
    .build_plan()
```

### Execution Phases

1. **BUILD** - Construct the structure by growing/branching bricks (no gravity)
   - `branching(brick, role)` - Place a seed brick at the specified altitude
   - `growing(n)` - Grow a column of n bricks
   - `.on_face(alias, ...)` - Specify growth from specific faces
   - `.scale(factor)` - Scale each successive brick
   - `.mark(name)` - Tag faces for later operations
   - `.chiral()` - Alternate left/right chirality
   - `.prism()` - Add prism reinforcement

2. **SHAPE** - Manipulate the structure while still in construction physics
   - `space(mark, factor)` - Adjust spacing at marked faces
   - `join(mark)` - Connect faces with the same mark
   - `vulcanize()` - Add reinforcing intervals

3. **PRETENSE** - Apply pretension to cables (no gravity)
   - Removes construction faces, leaving only the tensegrity structure
   - Applies pretension percentage to all cable intervals
   - `.surface(character)` - Surface interaction for later phases
   - Duration controls how gradually pretension is applied

4. **FALL** - Drop the structure with gravity (minimal damping)
   - Gravity is enabled based on surface_character
   - Structure falls freely and bounces/wobbles
   - Duration should be enough for the structure to hit ground (~2s from 10m)

5. **SETTLE** - Calm the structure down (progressive damping)
   - Gradually increases damping over time
   - Structure wobbles less and less until stable
   - Joints that touch Frozen surface lock in place

## Baking Process

The "baking" process converts a logical `Prototype` into a physical `BakedBrick`:

1. **Prototype → Fabric** - Create a physics simulation with joints at origin
2. **Physics Iteration** - Let forces settle the structure into equilibrium
3. **Fabric → BakedBrick** - Extract final geometry and strains
4. **Validation** - Check face intervals have proper strain (~0.1)

The `Oven` (in `src/build/oven.rs`) manages this process, running physics until `max_velocity < 3e-6`.

## Type Safety

The DSL is fully type-checked by Rust:
- `BrickName` enum - All brick types
- `BrickRole` enum - All usage contexts
- `FaceName` enum - All face aliases
- `JointName` enum - All joint identifiers
- `Spin` enum - Left/Right chirality

This catches errors at compile time that would be runtime errors in Tenscript.

---

*The Rust DSL provides better tooling, type safety, and performance than the older S-expression approach.*
