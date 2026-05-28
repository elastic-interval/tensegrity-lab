# Tensegrity DSL

The Tensegrity DSL is a Rust-embedded domain-specific language for defining
tensegrity bricks and fabrics. It replaces the older S-expression-based
Tenscript language.

## Fabric Definitions

Fabrics are defined in `src/build/dsl/fabric_library.rs` using a fluent
builder API:

```rust
OpenClaw
    .build(
        FabricDimensions::default()
            .with_scale(M(0.80))
            .with_locked_bend_magnitudes(vec![12.0, 30.0, 49.0, 68.0]),
    )
    .seed(OmniSymmetrical, Seed(1))
    .faces([
        on(OmniBotX).column(4).tip_label().prism(Pct(200.0)),
        on(OmniBotY).column(4).tip_label().prism(Pct(200.0)),
        on(OmniBotZ).column(4).tip_label().prism(Pct(200.0)),
        on(OmniTop).prism(Pct(200.0)),
        on(OmniBot).open(),
    ])
    .prepare_vulcanize(0.5, VulcanizeMode::Linear)
    .space(Sec(2.8), [Tip(OmniBotX), Tip(OmniBotY), Tip(OmniBotZ)], Pct(46.0))
    .vulcanize(Sec(1.0))
    .pretense(Sec(3.0), Pct(1.0))
    .surface_frozen()
    .fall(Sec(1.5))
    .settle(Sec(1.5))
    .animate()
    .actuator_frequency(Hz(3.0))
    .amplitude(Pct(3.0))
    .stiffness(Pct(1.0))
    .sine()
    .actuators([
        phase(Pct(0.0)).between("CX2Z4", "BX4Z5"),
        phase(Pct(0.0)).between("AX2Z4", "CX4Z5"),
        phase(Pct(0.0)).between("BX2Z4", "AX4Z5"),
    ])
```

## Execution Phases

A fabric plan consists of sequential phases.

### 1. BUILD Phase

Construct the structure using hubs and columns (no gravity).

**Starting a fabric:**

```rust
FabricName
    .build(FabricDimensions::default()
        .with_altitude(M(7.5))   // optional, default 0
        .with_scale(M(1.03)))    // optional, default 1
    .seed(BrickName, BrickRole)
```

`FabricDimensions::default()` is the source of truth for altitude, scale,
joint mass, pull radius, push density, and connector geometry — see
`src/fabric/dimensions.rs`.

The seed brick's orientation also drives **joint naming** — see
[joint-naming.md](joint-naming.md) for how seed joints become labels like
`BAA` / `TOC`.

**Seed (root hub):**

```rust
.seed(BrickName, BrickRole)
    .shrink_by(Pct(10.0))    // optional, 90% scale
    .grow_by(Pct(10.0))      // optional, 110% scale
    .faces([...])            // define content for output faces
```

**Face array syntax:**

```rust
.seed(OmniSymmetrical, Seed(1))
.faces([
    on(OmniBotX).column(8).shrink_by(Pct(10.0)).tip_label().prism(Pct(100.0)),
    on(OmniBotY).column(8).shrink_by(Pct(10.0)).tip_label().prism(Pct(100.0)),
    on(OmniTop).column(1),
])
```

`on(FaceName)` starts a face entry. Method chains build a `Face`; the array
is parallel — order in the array does not imply ordering at runtime.

**Hub (multi-face brick attached to a parent face):**

```rust
hub(BrickName)               // role auto-derived from parent face spin
    .shrink_by(Pct(10.0))
    .grow_by(Pct(10.0))
    .faces([
        on(FaceName).column(n),
        on(FaceName).tip_label(),
    ])
```

The `BrickRole` for a hub is always `OnSpin(spin)` where `spin` matches the
parent face's spin (mirrored); `hub(brick)` derives this automatically at
attach time. Write `OnSpin(_)` explicitly only in brick prototype
definitions.

**Column (extending a chain of bricks):**

```rust
column(count)                // build `count` bricks in a chain
    .shrink_by(Pct(10.0))    // 90% scale per successive brick
    .grow_by(Pct(10.0))      // 110% scale per successive brick
    .rotate(Rotation::OneThird)  // optional 120° rotation step
    .tip_label()             // label the final face as Tip(start-face-name)
    .label(FaceLabel)        // or explicit label
    .prism(Pct(100.0))       // Pct(100) symmetric, Pct(200) outer extends 2×
    .then(node)              // continue with nested hub/column at the end
```

`.rotate(Rotation)` is only available on the per-face/column entry; the
enum is `Zero` (default), `OneThird`, `TwoThirds`. Chaining columns:

```rust
on(OmniTop).column(4).then(
    hub(OmniSymmetrical).faces([
        on(OmniTopX).column(12).shrink_by(Pct(8.0)).tip_label(),
        on(OmniTopY).column(11).shrink_by(Pct(8.0)).tip_label(),
    ]),
)
```

Both `hub(...)` and `column(...)` (and `on(...).column(...)`) implement
`Into<BuildNode>`, so `.build()` is unnecessary on them.

### Face Labels

`FaceLabel` is a rich enum that uniquely names a face for later shape
operations. Variants:

- `Tip(FaceName)` — a column's terminal face, named by the face it grew
  from. `.tip_label()` is shorthand for `.label(Tip(start_face))`.
- `Top(u8)`, `Bottom(u8)` — small integer-indexed pairs (used by Diamond).
- `Foot(Side)`, `Hand(Side)`, `ChestUpper(Side)`, `ChestLower(Side)` —
  body-side mirror pairs (used by HeadlessHug). `Side` is `Left | Right`.

Each label must be unique within a fabric — duplicates panic at build
time.

### 2. SHAPE Phase

Adjust the structure under construction physics. Each shape op takes its
duration as the first argument:

| Operation | Description |
|-----------|-------------|
| `.space(Sec, [FaceLabel; N], Pct)` | Spacer intervals between every pair of labeled faces |
| `.space_parallel(Sec, [spacer(...); N])` | Multiple `space` specs run together |
| `.join(Sec, alpha, omega)` | Join two faces into one |
| `.join_parallel(Sec, iter_of_pairs)` | Multiple joins run together |
| `.vulcanize(Sec)` | Add reinforcing bow-tie intervals |
| `.prepare_vulcanize(contraction, mode)` | Set per-bow-tie target before `vulcanize` |
| `.down(Sec, [FaceLabel; N])` | Rotate so the average normal of these faces points down |
| `.centralize(Sec)` | Center horizontally |
| `.centralize_at(Sec, M)` | Center at the given altitude |
| `.omit([(name, name); N])` | Remove specific intervals by joint-label pair |
| `.add(Sec, [(name, name, Pct); N])` | Add an interval; Pct<100 → pull, Pct>100 → push |

**Parallel join with `tips(...)` helper:**

```rust
.join_parallel(Sec(2.0), tips([
    (RightFrontTop, RightFrontBottom),
    (LeftBackTop,   LeftBackBottom),
]))
```

`tips([...])` lifts pairs of `FaceName` into pairs of `FaceLabel::Tip(...)`,
saving the wrapper repetition.

**Parallel spacers:**

```rust
.space_parallel(Sec(8.0), [
    spacer([Foot(Side::Left),  Hand(Side::Left)],  Pct(100.0)),
    spacer([Foot(Side::Right), Hand(Side::Right)], Pct(100.0)),
])
```

### 3. PRETENSE Phase

Apply pretension to cables (no gravity). Removes construction faces, leaving
only the tensegrity structure. Pretensing grows every push interval's rest
length by `pretenst%` over `seconds`; pulls absorb the displacement and
tension up.

```rust
.pretense(Sec(0.1), Pct(1.0))           // duration, percent push lengthening
    .rigidity(Pct(100.0))               // optional rigidity multiplier
    .surface_frozen()                   // required: pick one surface mode
```

**Surface modes (one required to complete the plan):**

- `.surface_frozen()` — joints touching surface lock in place
- `.surface_bouncy()` — joints bounce off the surface
- `.surface_slippery()` — joints slide along the surface
- `.floating()` — no surface interaction

### 4. FALL Phase (optional)

Drop the structure with gravity enabled.

```rust
.fall(Sec(duration))
```

### 5. SETTLE Phase (optional)

Calm the structure with progressive damping until stable.

```rust
.settle(Sec(duration))
```

### 6. ANIMATE Phase (optional)

Add actuators that rhythmically contract to animate the structure.

```rust
.animate()
    .actuator_frequency(Hz(1.21))   // cycle frequency
    .amplitude(Pct(1.0))            // contraction amplitude
    .stiffness(Pct(10.0))           // actuator stiffness
    .pulse(Pct(10.0))               // square wave with 10% duty (or .sine())
    .actuators([
        phase(Pct(0.0)).between("joint-a", "joint-b"),
        phase(Pct(50.0)).between("joint-c", "joint-d"),
    ])
```

At runtime, `F` / `f` keys raise / lower the frequency while Animating;
each new value is logged in `actuator_frequency(Hz(...))` form ready to
paste back into the DSL.

**Phase offset** in `phase(Pct(offset))`: `Pct(0.0)` contracts at cycle
start, `Pct(50.0)` is opposite-phase.

**Attachments:** `.between(joint_a, joint_b)` or
`.surface(joint, (x, z))` (anchored to a ground point).

**Waveforms:** `.sine()` (default) or `.pulse(Pct(duty))`.

### Completing the Plan

The plan is complete once a surface method is called. After that,
`.fall()`, `.settle()`, and `.animate()` are all optional chains. When
using `.animate()`, the terminal `.actuators([...])` closes the plan.

## Brick Definitions

Bricks are defined using a fluent builder API in
`src/build/dsl/brick_library/`:

```rust
proto(SingleTwistLeft, [Seed(1), OnSpin(Spin::Left)])
    .pushes(3.204, [(AlphaX, OmegaX), (AlphaY, OmegaY), (AlphaZ, OmegaZ)])
    .pulls(2.0, [(AlphaX, OmegaZ), (AlphaY, OmegaX), (AlphaZ, OmegaY)])
    .face(Spin::Left, [AlphaX, AlphaY, AlphaZ], [
        OnSpin(Spin::Left).calls_it(Attach(Spin::Left)),
        Seed(1).calls_it(SingleBot),
        Seed(1).downwards(),
    ])
    .face(Spin::Left, [OmegaZ, OmegaY, OmegaX], [
        OnSpin(Spin::Left).calls_it(SingleTop),
        OnSpin(Spin::Left).calls_it(AttachNext),
        Seed(1).calls_it(SingleTop),
    ])
    .build()
```

### Brick Building Phases

**Prototype Phase:**

- `.proto(name, roles)` — name and roles this brick can be used in
- `.pushes(ideal, pairs)` — grouped compression intervals with shared
  ideal length
- `.pulls(ideal, pairs)` — grouped tension intervals with shared ideal
  length
- `.face(spin, joints, aliases)` — triangular face with chirality and
  role-based names

**Baked Phase:**

- `.baked()` — switch to defining the settled geometry
- `.joints([...])` — final 3D positions after physics
- `.pushes([...])` / `.pulls([...])` — final interval strains
- `.build()` — construct the complete `Brick`

### Face Aliases

Faces have multiple names depending on the brick's **role** in the
construction. `BrickRole` has two variants:

- `Seed(n)` — when this brick is the seed; `n` is how many faces point
  downward in that orientation (e.g. `Seed(1)`, `Seed(2)`, `Seed(4)`).
- `OnSpin(spin)` — when attaching the brick to a parent face. By
  convention the `spin` in the role equals the spin of the brick's
  `Attach` face. The hub auto-derives this role from the parent face's
  spin at attach time, so you rarely write `OnSpin(_)` outside brick
  prototype definitions.

Each face can have multiple aliases for different roles:

```rust
OnSpin(Spin::Right).calls_it(Attach(Spin::Right)),  // hub attach point
Seed(1).calls_it(SingleBot),                        // bottom face when seed
Seed(1).downwards(),                                // orientation marker
```

## Baking Process

The "baking" process converts a logical `Prototype` into a physical
`BakedBrick`:

1. **Prototype → Fabric** — create a physics simulation with joints at
   origin
2. **Physics iteration** — let forces settle the structure into
   equilibrium
3. **Fabric → BakedBrick** — extract final geometry and strains
4. **Validation** — face intervals must reach the target strain (~0.1)

The `Oven` (in `src/build/oven.rs`) manages this process, running physics
until `max_velocity < 3e-6`.

## Type Safety

The DSL is fully type-checked by Rust:

- `FabricName` — all fabric types (entry point for fabric definitions)
- `BrickName` — all brick types
- `BrickRole` — `Seed(usize)` or `OnSpin(Spin)`
- `FaceName` — face aliases inside a brick
- `FaceLabel` — uniquely named faces inside a fabric (rich enum, see
  *Face Labels* above)
- `JointName` — joints inside a brick
- `Spin` — `Left | Right` chirality
- `Side` — `Left | Right`, used inside body-pair `FaceLabel` variants
- `Rotation` — `Zero | OneThird | TwoThirds` (120° steps)

This catches errors at compile time that would be runtime errors in
Tenscript.

## Unit Types

The DSL uses type-safe units:

- `M(value)` — length in meters
- `Sec(value)` — time in seconds
- `Pct(value)` — percentage (scale, spacing, amplitude, stiffness, etc.)
- `Hz(value)` — frequency in cycles per second
- `Gm(value)` — mass in grams
- `GpmM(value)` — linear density (grams per meter)
