//! Evolving the library bricks from a simple specification and rules
//! (docs/brick-evolution.md).

pub mod measure;
pub mod neighbours;

#[cfg(test)]
mod fixed_point;

use crate::build::dsl::brick::BrickPrototype;
use crate::build::dsl::brick_library::equilibrium::{settle, Bake};
use neighbours::Shape;

/// The prototype at equilibrium, and its geometry by joint name.
pub(crate) fn bake(proto: &BrickPrototype, initial_scale: f32) -> (Bake, Shape) {
    let (bake, names, scale) = settle(proto, initial_scale);
    let location = names[..bake.structural]
        .iter()
        .copied()
        .zip(bake.positions.iter().copied())
        .collect();
    (bake, Shape { location, scale })
}
