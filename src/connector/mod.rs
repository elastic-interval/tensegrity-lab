//! Physical connector hardware for large-scale builds: ring/boss/pivot
//! geometry and cable-to-slot assignment. Adjacent to the simulation —
//! nothing here participates in the physics tick.

pub mod attachment;
pub mod bridle_clamp;
pub mod dimensions;
pub mod fork;
pub mod node_plate;
pub mod parts;
pub mod system;

pub use dimensions::{pivot_angle, ConnectorDimensions};
pub use system::ConnectorSystem;
