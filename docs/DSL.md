# Tensegrity DSL

The Tensegrity DSL is a Rust-embedded domain-specific language for defining tensegrity bricks and fabrics. It replaces the older S-expression-based Tenscript language.

## Fabric Definitions

Fabrics are defined in `src/build/dsl/fabric_library.rs` using a fluent builder API:

```rust
fabric("Triped")
    .altitude(M(7.5))
    .scale(M(1.03))
    .seed(OmniSymmetrical, Seed(1))
    .on_face(OmniBotX, column(8).scale(Pct(90.0)).mark(End).prism().build())
    .on_face(OmniBotY, column(8).scale(Pct(90.0)).mark(End).prism().build())
    .on_face(OmniBotZ, column(8).scale(Pct(90.0)).mark(End).prism().build())
    .on_face(OmniTop, column(1).build())
    .space(Sec(3.0), End, Pct(38.0))
    .vulcanize(Sec(1.0))
    .pretense(Sec(1.0))
    .surface(SurfaceCharacter::Frozen)
    .fall(Sec(3.0))
    .settle(Sec(3.0))
    .animate_pulse(
        Sec(0.8266),
        Pct(1.0),
        0.1,
        Pct(10.0),
        vec![
            ActuatorSpec::Alpha.between(151, 48),
            ActuatorSpec::Alpha.between(157, 36),
            ActuatorSpec::Alpha.between(145, 42),
        ],
    )
    .build_plan()
```

## Execution Phases

A fabric plan consists of sequential phases:

### 1. BUILD Phase

Construct the structure using hubs and columns (no gravity).

**Starting a fabric:**
```rust
fabric("Name")
    .altitude(M(7.5))    // Initial altitude in meters
    .scale(M(1.03))      // Real-world scale in meters
    .seed(BrickName, BrickRole)
```

The typestate pattern enforces that `altitude()` and `scale()` must be called before `seed()`.

**Seed (starting hub at root):**
```rust
.seed(BrickName, BrickRole)
    .scale(Pct(90.0))        // Optional scale
    .rotate()                // Optional rotation
    .on_face(FaceName, node) // Specify what builds from each face
```

**Hub (placing a multi-face brick):**
```rust
hub(BrickName, BrickRole)    // Place a brick with multiple output faces
    .scale(Pct(90.0))        // Optional scale
    .rotate()                // Optional rotation
    .on_face(FaceName, node) // Specify what builds from each face
    .build()
```

**Column (extending a column of bricks):**
```rust
column(count)                // Build n bricks in a column
    .scale(Pct(90.0))        // Scale each successive brick
    .chiral()                // Same chirality (vs alternating default)
    .mark(MarkName)          // Tag the end face for later operations
    .prism()                 // Add prism reinforcement
    .build_node(node)        // Add nested build operations
    .build()
```

**Marking without building:**
```rust
mark(MarkName)               // Just mark a location, no column
```

### 2. SHAPE Phase

Manipulate the structure while still in construction physics. Each shape operation includes its duration as the first argument:

```rust
.space(Sec(3.0), End, Pct(38.0))
.vulcanize(Sec(1.0))
.join(Sec(10.0), HaloEnd)
.centralize_at(Sec(1.0), M(0.075))
```

**Shape Operations:**

| Operation | Description |
|-----------|-------------|
| `.space(Sec, mark, Pct)` | Adjust spacing at marked faces |
| `.join(Sec, mark)` | Connect faces with the same mark together |
| `.vulcanize(Sec)` | Add reinforcing intervals to strengthen the structure |
| `.down(Sec, mark)` | Point marked faces downward |
| `.centralize(Sec)` | Center the structure horizontally |
| `.centralize_at(Sec, M)` | Center at specific altitude in meters |

### 3. PRETENSE Phase

Apply pretension to cables (no gravity). Removes construction faces, leaving only the tensegrity structure.

```rust
.pretense(Sec(duration))
    .surface(SurfaceCharacter::Frozen)  // Surface interaction for later
    .altitude(M(height))                // Optional altitude in meters
    .pretenst(Pct(1.0))                 // Optional pretension
    .rigidity(Pct(100.0))               // Optional rigidity
```

**Surface Characters:**
- `SurfaceCharacter::Frozen` - Joints touching surface lock in place
- `SurfaceCharacter::Bouncy` - Joints bounce off surface

### 4. FALL Phase

Drop the structure with gravity enabled (minimal damping).

```rust
.fall(Sec(duration))         // Duration for free fall
```

### 5. SETTLE Phase (Optional)

Calm the structure with progressive damping until stable.

```rust
.settle(Sec(duration))       // Duration for settling
```

### 6. ANIMATE Phase (Optional)

Add actuators that rhythmically contract to animate the structure.

**Sine wave animation (smooth oscillation):**
```rust
.animate_sine(
    Sec(period),             // Cycle period
    Pct(1.0),                // Contraction amplitude
    Pct(10.0),               // Stiffness
    vec![actuators...],
)
```

**Pulse animation (solenoid-like snap):**
```rust
.animate_pulse(
    Sec(period),             // Cycle period
    Pct(1.0),                // Contraction amplitude
    0.3,                     // Duty cycle (proportion "on")
    Pct(10.0),               // Stiffness
    vec![actuators...],
)
```

**Actuator Specifications:**

```rust
// Connect two existing joints
ActuatorSpec::Alpha.between(joint_a, joint_b)
ActuatorSpec::Omega.between(joint_a, joint_b)

// Connect joint to a point on the surface
ActuatorSpec::Alpha.to_surface(joint, (x, z))
```

- `Alpha` actuators contract when the oscillator is high
- `Omega` actuators contract when the oscillator is low (opposite phase)

**Waveforms:**
- `Sine` - Smooth sinusoidal contraction (default)
- `Pulse { duty_cycle }` - Square wave, instantly on/off

### Final Step

```rust
.build_plan()                // Finalize the plan
```

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

### Brick Building Phases

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

## Baking Process

The "baking" process converts a logical `Prototype` into a physical `BakedBrick`:

1. **Prototype -> Fabric** - Create a physics simulation with joints at origin
2. **Physics Iteration** - Let forces settle the structure into equilibrium
3. **Fabric -> BakedBrick** - Extract final geometry and strains
4. **Validation** - Check face intervals have proper strain (~0.1)

The `Oven` (in `src/build/oven.rs`) manages this process, running physics until `max_velocity < 3e-6`.

## Type Safety

The DSL is fully type-checked by Rust:
- `BrickName` enum - All brick types
- `BrickRole` enum - All usage contexts
- `FaceName` enum - All face aliases
- `MarkName` enum - All mark identifiers
- `JointName` enum - All joint identifiers
- `Spin` enum - Left/Right chirality

This catches errors at compile time that would be runtime errors in Tenscript.

## Unit Types

The DSL uses type-safe units:
- `M(value)` - Length in meters
- `Sec(value)` - Time in seconds
- `Pct(value)` - Percentage (scale, spacing, amplitude, stiffness, etc.)

---

*The Rust DSL provides better tooling, type safety, and performance than the older S-expression approach.*
