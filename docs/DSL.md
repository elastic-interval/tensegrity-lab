# Tensegrity DSL

The Tensegrity DSL is a Rust-embedded domain-specific language for defining tensegrity bricks and fabrics. It replaces the older S-expression-based Tenscript language.

## Fabric Definitions

Fabrics are defined in `src/build/dsl/fabric_library.rs` using a fluent builder API:

```rust
Triped
    .altitude(M(7.5))
    .scale(M(1.03))
    .seed(OmniSymmetrical, Seed(1))
    .faces([
        on(OmniBotX).column(8).shrink_by(Pct(10.0)).mark(End).prism(),
        on(OmniBotY).column(8).shrink_by(Pct(10.0)).mark(End).prism(),
        on(OmniBotZ).column(8).shrink_by(Pct(10.0)).mark(End).prism(),
        on(OmniTop).column(1),
    ])
    .space(Sec(3.0), End, Pct(38.0))
    .vulcanize(Sec(1.0))
    .pretense()
    .surface_frozen()
    .fall(Sec(3.0))
    .settle(Sec(3.0))
    .animate()
    .period(Sec(0.8266))
    .amplitude(Pct(1.0))
    .stiffness(Pct(10.0))
    .pulse(Pct(10.0))
    .actuators(&[
        phase(Pct(0.0)).between(151, 48),
        phase(Pct(0.0)).between(157, 36),
        phase(Pct(0.0)).between(145, 42),
    ])
```

## Execution Phases

A fabric plan consists of sequential phases:

### 1. BUILD Phase

Construct the structure using hubs and columns (no gravity).

**Starting a fabric:**
```rust
FabricName
    .altitude(M(7.5))    // Initial altitude in meters
    .scale(M(1.03))      // Real-world scale in meters
    .seed(BrickName, BrickRole)
```

The typestate pattern enforces that `altitude()` and `scale()` must be called before `seed()`.

**Seed (starting hub at root):**
```rust
.seed(BrickName, BrickRole)
    .shrink_by(Pct(10.0))    // Optional: shrink by 10% (90% scale)
    .grow_by(Pct(10.0))      // Optional: grow by 10% (110% scale)
    .rotate()                // Optional rotation
    .faces([...])            // Define content for faces
```

**Face array syntax:**
```rust
.seed(OmniSymmetrical, Seed(1))
.faces([
    on(OmniBotX).column(8).shrink_by(Pct(10.0)).mark(End).prism(),
    on(OmniBotY).column(8).shrink_by(Pct(10.0)).mark(End).prism(),
    on(OmniTop).column(1),
])
.space(Sec(3.0), End, Pct(38.0))
```

The `.faces([...])` method takes an array of face definitions created with `on(FaceName)`. This makes the parallel nature of face construction explicit.

**Hub (placing a multi-face brick):**
```rust
hub(BrickName, BrickRole)    // Place a brick with multiple output faces
    .shrink_by(Pct(10.0))    // Optional: shrink by 10% (90% scale)
    .grow_by(Pct(10.0))      // Optional: grow by 10% (110% scale)
    .rotate()                // Optional rotation
    .faces([
        on(FaceName).column(n),
        on(FaceName).mark(Name),
    ])
```

**Column (extending a column of bricks):**
```rust
column(count)                // Build n bricks in a column
    .shrink_by(Pct(10.0))    // Shrink each successive brick by 10% (90% scale per brick)
    .grow_by(Pct(10.0))      // Grow each successive brick by 10% (110% scale per brick)
    .chiral()                // Same chirality (vs alternating default)
    .mark(MarkName)          // Tag the end face for later operations
    .prism()                 // Add prism reinforcement
    .then(node)              // Continue with nested structure at the end
```

**Chaining columns:**
```rust
column(4).chiral().shrink_by(Pct(8.0)).then(
    column(1).then(column(2).chiral().mark(Legs))
)
```

Both `hub()` and `column()` implement `Into<BuildNode>`, so no `.build()` call is needed.

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
.pretense()
    .step_duration(Sec(0.1))            // Optional: duration per extension step
    .rigidity(Pct(100.0))               // Optional rigidity
    .min_push_strain(Pct(1.0))          // Optional target compression
    .max_push_strain(Pct(3.0))          // Optional max compression per step
    .surface_frozen()                   // Required: specify surface interaction
```

**Surface choices (required - one must be called to complete the plan):**
- `.surface_frozen()` - Joints touching surface lock in place
- `.surface_bouncy()` - Joints bounce off surface
- `.floating()` - No surface interaction, fabric floats in space

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

```rust
.animate()
    .period(Sec(0.8266))     // Cycle period
    .amplitude(Pct(1.0))     // Contraction amplitude
    .stiffness(Pct(10.0))    // Actuator stiffness
    .pulse(Pct(10.0))        // Square wave with 10% duty cycle (or .sine())
    .actuators(&[
        phase(Pct(0.0)).between(151, 48),
        phase(Pct(0.0)).between(157, 36),
        phase(Pct(50.0)).between(145, 42),
    ])
```

**Actuator Phase:**

Actuators are created with `phase(Pct(offset))` where offset is a percentage of the cycle:
- `Pct(0.0)` - Contracts at start of cycle
- `Pct(50.0)` - Contracts at half cycle (opposite phase)
- `Pct(33.3)` - Contracts at one-third of cycle

**Actuator Attachments:**
- `.between(joint_a, joint_b)` - Actuator between two joints in the fabric
- `.surface(joint, (x, z))` - Actuator anchored to a surface point

**Waveforms:**
- `.sine()` - Smooth sinusoidal contraction (default)
- `.pulse(Pct(duty))` - Square wave, instantly on/off

### Completing the Plan

The fabric plan is automatically completed when you call a surface method (`.surface_frozen()`, `.surface_bouncy()`, or `.floating()`). After that, optional `.fall()`, `.settle()`, and `.animate()` can be chained to configure those phases. If using `.animate()`, the terminal `.actuators(&[...])` method completes the plan.

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
- `FabricName` enum - All fabric types (entry point for fabric definitions)
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
