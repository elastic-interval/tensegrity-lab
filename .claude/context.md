# Tensegrity Lab - Claude Code Context

## Project Overview

Tensegrity Lab is a high-performance Rust application for designing, simulating, and physically building tensegrity structures using Elastic Interval Geometry (EIG). Tensegrity structures are spatial systems composed entirely of compression elements (bars/struts) and tension elements (cables) that maintain their shape through balanced push-pull forces.

**Key Goals:**
- Fast implementation of Elastic Interval Geometry physics simulation
- Design tensegrities by composing modular "bricks" using Tenscript language
- Enable physical construction of designed structures
- Support Darwinian evolution of tensegrity structures

**Dual Architecture:**
1. **Native version** (WGPU rendering) - Design-focused with Tenscript file watching
2. **Web version** (WASM) - Build-focused with interval selection for physical construction

## Core Architecture

### The Fabric Model

The `Fabric` (src/fabric/mod.rs) is the central data structure representing a tensegrity structure:
- **Joints**: Points in 3D space with position, velocity, and mass
- **Intervals**: Connections between joints with role (push/pull), stiffness, and strain
- **Faces**: Triangular surfaces for visual representation
- **Age**: Tracks simulation time (each iteration = 50 microseconds of fabric time)
- **Scale**: Converts fabric units to millimeters for real-world construction

### Physics System

**Physics Presets** (src/fabric/physics/presets.rs):
- `CONSTRUCTION` - Used during building phase (5× time acceleration)
- `PRETENSING` - Used when applying tension to reach equilibrium
- `VIEWING` - Physics for frozen viewing state
- `PHYSICS_TEST` - Real-time (1×) physics for drop tests

**Key Physics Concepts:**
- Each iteration represents 50 microseconds of fabric time
- Gravity: 9.8 m/s² in real time
- Convergence: Gradual damping increase to reach equilibrium
- Pretenst: Target tension percentage for intervals

### Dynamic Time Scaling System

**Critical Recent Implementation:**

The simulation uses **dynamic iteration calculation** to maintain target time scales regardless of frame rate:

```rust
// Formula: iterations_per_frame = target_scale × 20000 / FPS
// At 100 FPS with 5× target: 5.0 × 20000 / 100 = 1000 iterations
```

**Target Time Scales:**
- **5.0** during construction/building (5× speedup)
- **1.0** during physics testing (real time, 1:1)
- **0.0** during viewing (frozen, no iteration)

**How it works:**
1. Application tracks current FPS
2. Crucible provides `target_time_scale()` based on current stage
3. Application calculates `iterations_per_frame` dynamically each frame
4. Internal components use nominal 1000 iterations, outer loop adjusts

**Key Files:**
- `src/crucible.rs:60-68` - Target time scale logic
- `src/application.rs:554-564` - Dynamic iteration calculation
- `src/lib.rs:70` - `ITERATION_DURATION` constant (50µs)

**Philosophy:** No hardcoded iteration constants. System self-corrects to maintain target time scale. Start with nominal estimate, quickly adjust to target.

### The Crucible

The `Crucible` (src/crucible.rs) manages the entire simulation lifecycle through stages:

**Stages:**
1. **Initialization** - Setting up
2. **Animator** - Running build animations
3. **Converger** - Settling to equilibrium with gradual damping
4. **PhysicsTester** - Real-time physics testing with gravity
5. **Viewing** - Frozen final state

**CrucibleContext:**
Bundles commonly-passed data:
- `fabric` - The structure being simulated
- `physics` - Current physics parameters
- `brick_library` - Reusable structural components
- `radio` - Event broadcasting system

### Build System

**Tenscript Language** (docs/Tenscript.md):
- Custom DSL for procedurally generating tensegrity structures
- Defines bricks (modular components) and fabrics (complete structures)
- Supports phases: build, shape, pretense, converge

**Build Phases:**
1. **Build Phase** - Grows structure by placing bricks and creating intervals
2. **Shape Phase** - Applies transformations to reach target geometry
3. **Pretense Phase** - Applies tension to reach equilibrium
4. **Converge Phase** - Settles structure over specified time

**PlanRunner** (src/build/tenscript/plan_runner.rs):
- Executes Tenscript plans through distinct stages
- Stages: Initialize → GrowStep → GrowApproach → GrowCalm → Shaping → Completed
- Each stage transition is synchronized with fabric progress system
- Uses dynamic iteration count (nominal 1000, outer loop adjusts)

**FabricPlanExecutor** (src/build/tenscript/fabric_plan_executor.rs):
- Frame-independent execution of fabric plans
- Transitions from BUILD → PRETENSE → CONVERGE phases
- Works for both UI (real-time) and tests (headless)

### Component Lifecycle Managers

**Animator** (src/build/animator.rs):
- Runs build animations that create structure over time
- Uses nominal 1000 iterations per call, outer loop adjusts dynamically

**Oven** (src/build/oven.rs):
- "Bakes" brick prototypes into stable configurations
- Waits for fabric to settle (max velocity < 3e-6)
- Validates structure can be used as reusable brick

**Converger** (src/build/converger.rs):
- Handles convergence phase where fabric settles to equilibrium
- Gradually increases damping over specified time period
- Zeros velocities and freezes fabric when complete
- Disables convergence physics when transitioning to Viewing

**Pretenser** (src/build/tenscript/pretenser.rs):
- Applies pretension to intervals
- Stages: Start → Slacken → Pretensing → Pretenst
- Centralizes structure and sets target altitude

**PhysicsTester** (src/fabric/physics_test.rs):
- Real-time physics testing with gravity
- Accepts dynamic `iterations_per_frame` parameter
- Supports features: gravity, freeze, reset, speed tracking

### Rendering System

**WGPU-based 3D rendering** (src/wgpu/):
- `render_state.rs` - Main render coordinator
- `fabric_state.rs` - Renders intervals (cylinders) and joints (spheres)
- `mark_state.rs` - Renders marks (spherical indicators)
- `floor_state.rs` - Ground plane reference
- `text_state.rs` - UI text overlay (FPS, age, time scale)

**Camera System** (src/camera.rs):
- Spherical coordinate camera (azimuth, altitude, radius)
- Focus point with smooth transitions
- Synchronized with fabric centralization

### Event System

**Radio** - Broadcast-style event system (crossbeam channels)
- Events flow from Crucible → Application → UI
- Types: `LabEvent` (app-level) and `StateChange` (UI updates)

**Key Events:**
- `FabricBuilt` - Structure complete, transition to viewing
- `UpdateState` - UI state changes (camera, appearance, stage label)
- `Time` - FPS and time scale updates

## Important Design Patterns

### Progress System

The `Progress` struct (src/units.rs) manages time-based operations:
- Tracks remaining seconds for current operation
- `start(seconds)` - Begin countdown
- `decrement(delta)` - Advance by time delta
- `is_busy()` - Check if operation in progress

Used for stage transitions, shaping operations, and pretensing.

### Interval Roles

Intervals have distinct roles (src/fabric/material.rs):
- **Push** - Compression elements (bars/struts)
- **Pull** - Tension elements (cables)
- Each role has appearance properties (color, radius)

### Brick Library

Reusable structural components stored in `BrickLibrary`:
- Bricks are pre-baked, stable configurations
- Created by baking prototypes in Oven
- Used by PlanRunner during build phase
- Enables modular tensegrity design

### State Management

The application maintains separation between:
- **Simulation state** (Crucible, Fabric) - Pure computation
- **Rendering state** (WGPU states) - GPU resources
- **UI state** (Camera, controls) - User interaction

## File Structure Guide

```
src/
├── lib.rs                    # Core types, constants (ITERATION_DURATION)
├── main.rs                   # Native entry point
├── application.rs            # Main app loop, dynamic iteration calculation
├── crucible.rs              # Simulation lifecycle manager, target_time_scale()
├── crucible_context.rs      # Bundles fabric/physics/library/events
│
├── fabric/
│   ├── mod.rs               # Core Fabric struct
│   ├── physics/             # Physics parameters and presets
│   ├── material.rs          # Interval roles and properties
│   └── physics_test.rs      # Physics testing mode
│
├── build/
│   ├── animator.rs          # Build animations
│   ├── oven.rs              # Brick baking
│   ├── converger.rs         # Convergence phase manager
│   ├── evolution.rs         # Evolutionary algorithms
│   └── tenscript/           # Tenscript language implementation
│       ├── plan_runner.rs           # Executes build/shape phases
│       ├── fabric_plan_executor.rs  # Frame-independent execution
│       ├── pretenser.rs             # Pretension phase
│       ├── build_phase.rs           # Growth logic
│       ├── shape_phase.rs           # Shaping operations
│       └── pretense_phase.rs        # Pretension configuration
│
├── wgpu/                    # Rendering system
│   ├── render_state.rs      # Main render coordinator
│   ├── fabric_state.rs      # Fabric rendering
│   └── text_state.rs        # UI text (includes time scale display)
│
└── camera.rs                # Camera control
```

## Recent Evolution and Key Changes

### Dynamic Time Scaling Implementation

**Problem:** Iteration counts were hardcoded for specific frame rates, making the system fragile and difficult to tune.

**Solution:** Implemented dynamic iteration calculation based on target time scales:
- Removed `ITERATIONS_PER_FRAME` and `PHYSICS_TEST_ITERATIONS_PER_FRAME` constants
- System now calculates iterations needed per frame to maintain target scale
- Formula: `target_scale × 20000 / FPS`
- At 100 FPS: 5× target → 1000 iterations, 1× target → 200 iterations
- At 60 FPS: Would automatically adjust to 1667 and 333 respectively

**Files Changed:**
- `src/lib.rs` - Removed constants
- `src/crucible.rs` - Added `target_time_scale()`, changed `iterate()` signature
- `src/application.rs` - Added `current_fps` field, dynamic calculation
- `src/fabric/physics_test.rs` - Accept dynamic iterations parameter
- `src/wgpu/text_state.rs` - Display target scale instead of fluctuating percentage
- All build components - Use nominal 1000, outer loop adjusts

**UI Changes:**
- Show "5×" during construction (target visible)
- Show nothing during physics testing (striving for 100% real time)
- Never show "0×" during viewing

### Convergence Lifecycle Fix

**Problem:** `disable_convergence()` was called when entering physics testing, but it's actually part of ending the build process.

**Solution:** Moved call from `ToPhysicsTesting` action to end of convergence phase (transition to Viewing).

**Location:** `src/crucible.rs:243`

### Authentic Gravity Implementation

**Previous Session Work:**
- Implemented 9.8 m/s² gravity in real time
- Added 'J' key to drop structures (tests gravity)
- Physics testing achieves true 1:1 real time
- Average speed tracking over 100 iterations

### Camera Animation and Interaction System

**Critical Architecture Fix:**

The camera must ALWAYS update via `scene.animate()`, regardless of control state. This was broken by short-circuit evaluation.

**Problem:** In `application.rs`, the condition for running animation was:
```rust
let animate = matches!(control_state, Viewing | PhysicsTesting(_)) || scene.animate(fabric)
```

When in `Viewing` mode, the `||` short-circuited and `scene.animate()` never ran, preventing camera from approaching its target.

**Solution:** Always call `scene.animate()` first (application.rs:527-533):
```rust
let camera_animating = scene.animate(fabric);  // Always call first
let animate = matches!(control_state, Viewing | PhysicsTesting(_)) || camera_animating;
```

**Camera Reset Pattern:**

Keep `camera.reset()` simple - just set the target (camera.rs:221-224):
```rust
pub fn reset(&mut self) {
    self.current_pick = Pick::Nothing;
    self.set_target(Target::FabricMidpoint);
}
```

Let `target_approach()` detect target changes naturally via thread_local state. Don't manually manipulate thread_local variables in reset().

**Picking Restrictions:**

Only allow picking when in Viewing mode. Check implemented in `scene.pointer_changed()` (scene.rs:223-237):
```rust
let pointer_changed = if !self.pick_allowed {
    match pointer_changed {
        PointerChange::Released(_) | PointerChange::TouchReleased(_) => PointerChange::NoChange,
        other => other,  // Allow rotation, zoom in all modes
    }
} else {
    pointer_changed
};
```

This allows camera rotation and zooming in all states, but restricts selection to Viewing mode only.

**WASM Compatibility:**

Removed `#[cfg(not(target_arch = "wasm32"))]` guards from `RefreshLibrary` and `UpdatedLibrary` handlers (application.rs:298-312). The code uses WASM-compatible `instant` crate and doesn't access filesystem, so reload fabric (Enter key) now works in browser.

## Common Operations

### Adding New Physics Features

1. Define feature flag in `PhysicsFeature` enum (src/lib.rs)
2. Add toggle handling in `PhysicsTester::toggle_feature()` (src/fabric/physics_test.rs)
3. Update physics application in `Fabric::iterate()` (src/fabric/mod.rs)
4. Add UI control in Application event handling

### Adding New Build Phases

1. Define phase configuration struct in `src/build/tenscript/`
2. Parse phase from Tenscript in `FabricPlan::from_source()`
3. Create phase manager (like Converger, Pretenser)
4. Add to FabricPlanExecutor state machine
5. Ensure uses nominal iterations, outer loop adjusts

### Modifying Time Scaling

The dynamic time scaling system is now complete and should rarely need modification:
- Change target scales in `Crucible::target_time_scale()`
- Formula is: `target_scale × 20000 / FPS`
- The 20000 constant comes from: (1 second = 1000ms) / (50µs per iteration) = 20000

### Adding Rendering Elements

1. Create state struct in `src/wgpu/` implementing `WgpuState` trait
2. Add to `RenderState` creation in `new()`
3. Add to render pass in `render()`
4. Handle updates in `redraw()` or via events

## Testing

**IMPORTANT: Always run tests in release mode for speed:**
```bash
cargo test --release
```

**Unit Tests:**
- Many modules have inline tests (see `#[cfg(test)]` blocks)

**Integration Tests:**
- `src/build/tenscript/plan_runner_test.rs` - Tests plan execution
- `src/build/tenscript/fabric_plan_executor_test.rs` - Tests frame-independent execution

**Physics Tests:**
- Interactive physics testing mode in application
- Drop test with 'J' key validates gravity
- Speed tracking validates real-time performance

## Key Insights and Philosophy

1. **No Hardcoded Iteration Constants** - System dynamically adjusts to maintain target time scales. Use nominal values, let outer loop correct.

2. **Frame Independence** - Build logic works same in UI and headless tests. FabricPlanExecutor enables this.

3. **Physics Presets** - Different phases need different physics. Don't tweak parameters directly, use/modify presets.

4. **Convergence is Build Phase** - Convergence happens at end of build, not beginning of testing. Disable it when transitioning to Viewing.

5. **Separation of Concerns** - Crucible manages lifecycle, Fabric manages structure, Physics manages forces, Rendering displays results.

6. **Event-Driven UI** - Simulation doesn't know about rendering. Radio events keep them loosely coupled.

7. **Real Time Means 1:1** - Physics testing should match real-world time. Each iteration = 50µs. Gravity = 9.8 m/s².

8. **Scale for Construction** - Fabric units are arbitrary. Scale factor converts to millimeters for building physical structures.

## Common Gotchas

1. **Don't add iteration constants** - Use dynamic calculation instead
2. **Check convergence state** - Disable it when transitioning from build to viewing
3. **Remember the scale** - Fabric coordinates != millimeters without scale factor
4. **Physics presets matter** - Construction physics != testing physics != viewing physics
5. **Progress must complete** - Stage transitions only happen when `progress.is_busy() == false`
6. **Clone when necessary** - Some contexts require cloning to avoid borrow checker issues (see `tester_physics`)

## Entry Points

**Native Application:**
```bash
cargo run --release -- --fabric "Halo by Crane"
```

**Web Version:**
```bash
trunk serve
```

## Contact

For questions: pretenst@gmail.com
Project: https://github.com/elastic-interval/tensegrity-lab
Related: https://pretenst.com/
