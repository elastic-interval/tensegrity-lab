//! Articulation evolution — evolve *articulating bricks*: face-centric
//! tensegrity cells (pushes + faces, like the static library bricks) that
//! own one or more **actuators** — contracting pulls between two face
//! centres — and are tuned to be compliant mechanisms with a single
//! stable equilibrium: rest → contract (large deflection, little effort)
//! → release → back to rest.
//!
//! Articulating bricks may grow richer than the static ones (more parts,
//! more moving parts). Evolution lives here only; nothing is baked into
//! the static brick library yet.
//!
//! See `docs/articulation-evolution.md` for the design rationale.

pub mod fitness;
pub mod genome;
pub mod metric;
pub mod structure;
pub mod trial;
pub mod visual_runner;

#[cfg(test)]
mod tests;

pub use genome::ArticulationGenome;
pub use visual_runner::ArticulationVisualRunner;
