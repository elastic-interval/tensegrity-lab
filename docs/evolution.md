# Evolution Framework

The evolution framework enables Darwinian evolution of tensegrity structures and behaviors. It implements Stuart Kauffman's "adjacent possible" concept—rather than searching a predefined parameter space, each genome defines what mutations are reachable from its current state.

## Architecture

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│     Genome      │────▶│     Fabric      │────▶│   TrialResult   │
│  (information)  │     │   (physical)    │     │   (evaluated)   │
└─────────────────┘     └─────────────────┘     └─────────────────┘
        │                       │                       │
        │ adjacent_possible()   │ physics iterations    │ fitness()
        ▼                       ▼                       ▼
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Vec<Genome>    │     │  settled state  │     │    f32 score    │
│   (variants)    │     │                 │     │                 │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

## Core Traits

### Genome

The hereditary information. A genome knows how to:
- Express itself into a physical Fabric
- Generate its "adjacent possible"—the set of reachable mutations

```rust
pub trait Genome: Clone + Send + Sync + Debug {
    fn id(&self) -> GenomeId;
    fn express(&self, context: &ExpressionContext) -> Fabric;
    fn adjacent_possible(&self, rng: &mut impl Rng) -> Vec<Self>;
    fn describe(&self) -> String;
    fn controllers(&self) -> Vec<ControllerAttachment> { vec![] }
}
```

The key insight is `adjacent_possible()`. Each genome type defines its own mutation operators. A genome that builds structures might sprout new bars. A genome that controls actuators might adjust frequencies or phases. The `controllers()` method optionally returns sensorimotor controllers to attach to intervals during trials.

### FitnessDimension

Evaluates one aspect of a trial result. Multiple dimensions combine into composite fitness.

```rust
pub trait FitnessDimension: Send + Sync {
    fn name(&self) -> &str;
    fn evaluate(&self, trial: &TrialResult) -> f32;  // 0.0 to 1.0
    fn weight(&self) -> f32 { 1.0 }
}
```

### PopulationStrategy

Manages selection and reproduction. The default `SimplePopulation` uses generational evolution with elitism.

```rust
pub trait PopulationStrategy<G: Genome>: Send {
    fn initialize(&mut self, seed_genomes: Vec<G>);
    fn next_for_trial(&mut self) -> Option<G>;
    fn record_result(&mut self, genome: G, fitness: f32);
    fn advance_generation(&mut self);
    fn best(&self) -> Option<&G>;
    fn should_terminate(&self) -> bool;
}
```

### IntervalController

Sensorimotor control for intervals. Controllers react to interval readings and set new target lengths:

```rust
pub trait IntervalController: Send + Sync {
    fn react(&mut self, reading: &IntervalReading) -> Option<f32>;
    fn reset(&mut self) {}
}
```

The `react()` method receives the current interval state and optionally returns a new target length. This enables closed-loop control where behavior emerges from sensing the physical state.

```rust
pub struct IntervalReading {
    pub interval_key: IntervalKey,
    pub role: Role,
    pub strain: f32,
    pub actual_length: Meters,
    pub ideal_length: Meters,
    pub unit_vector: Vec3,
    pub alpha_position: Vec3,
    pub alpha_velocity: Vec3,
    pub omega_position: Vec3,
    pub omega_velocity: Vec3,
}
```

Controllers are attached to intervals by index via `ControllerAttachment`:

```rust
pub struct ControllerAttachment {
    pub interval_index: usize,
    pub controller: Box<dyn IntervalController>,
}
```

## Trial Execution

A trial runs a genome through physics simulation with a sensorimotor loop:

1. **Express**: Genome creates a Fabric
2. **Attach**: Controllers from `genome.controllers()` are bound to intervals
3. **Loop**: For each physics iteration:
   - **Sense**: Read interval states
   - **React**: Controllers produce new target lengths
   - **Physics**: Apply forces and integrate
4. **Evaluate**: Fitness dimensions score the result

```rust
pub struct TrialResult {
    pub genome_id: GenomeId,
    pub fabric: Fabric,
    pub duration: Seconds,
    pub iteration_count: usize,
    pub max_strain: f32,
    pub structural_failure: bool,
    pub final_kinetic_energy: f32,
    pub initial_centroid: Vec3,
    pub final_centroid: Vec3,
}
```

## Current Implementation

### GrowthGenome

Grows structures by sprouting bars and joining endpoints with cables.

Mutations:
- **Sprout**: Add a new bar from an existing endpoint
- **Join**: Connect two endpoints with a pull interval

### StabilityFitness

Rewards structures that settle into stable configurations:
- Low kinetic energy (has stopped moving)
- Low strain (not over-stressed)
- Structural complexity bonus (prevents trivial solutions)

### SimplePopulation

Generational evolution:
- Keep best N individuals (elitism)
- Fill remaining slots with mutations of survivors
- "Demise of the least fit"

## Extending the Framework

### New Genome Types

Create a new genome by implementing the `Genome` trait:

```rust
pub struct MyGenome {
    // your hereditary data
}

impl Genome for MyGenome {
    fn express(&self, context: &ExpressionContext) -> Fabric {
        // build or modify a fabric
    }

    fn adjacent_possible(&self, rng: &mut impl Rng) -> Vec<Self> {
        // return reachable mutations
    }
}
```

### New Fitness Dimensions

```rust
pub struct LocomotionFitness;

impl FitnessDimension for LocomotionFitness {
    fn name(&self) -> &str { "Locomotion" }

    fn evaluate(&self, trial: &TrialResult) -> f32 {
        let displacement = trial.final_centroid - trial.initial_centroid;
        displacement.length() // reward movement
    }
}
```

### Composite Fitness

Combine multiple dimensions:

```rust
let mut fitness = CompositeFitness::new();
fitness.add_dimension(Box::new(StabilityFitness::new()));
fitness.add_dimension(Box::new(LocomotionFitness::new()));
```

## Future Directions

### Actuator Evolution

The framework separates structure from behavior. To evolve actuators on existing structures:

1. Build the structure using Tenscript DSL
2. Create a genome that encodes controller placement and parameters
3. The genome's `express()` returns the pre-built fabric
4. The genome's `controllers()` returns sensorimotor controllers for intervals
5. Fitness evaluates the emergent behavior (locomotion, etc.)

Example controller genome:

```rust
pub struct ControllerGenome {
    interval_indices: Vec<usize>,
    controller_params: Vec<ControllerParams>,
}

impl Genome for ControllerGenome {
    fn express(&self, context: &ExpressionContext) -> Fabric {
        // Return pre-built fabric from Tenscript
        load_fabric("walker.tenscript")
    }

    fn controllers(&self) -> Vec<ControllerAttachment> {
        self.interval_indices.iter()
            .zip(&self.controller_params)
            .map(|(&idx, params)| ControllerAttachment {
                interval_index: idx,
                controller: Box::new(MyController::new(params)),
            })
            .collect()
    }

    fn adjacent_possible(&self, rng: &mut impl Rng) -> Vec<Self> {
        // Mutate controller parameters, add/remove controllers
    }
}
```

### Headless Parallel Execution

The current visual runner executes one trial at a time. Adding headless parallel execution would dramatically speed up evolution for experiments that don't need visualization.

## Running Evolution

```bash
cargo run --release -- --evolve 42
```

The seed determines the random sequence for reproducibility.
