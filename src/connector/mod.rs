//! Physical connector hardware for large-scale builds: ring/boss/pivot
//! geometry and cable-to-slot assignment. Adjacent to the simulation —
//! nothing here participates in the physics tick.

pub mod attachment;
pub mod dimensions;
pub mod system;

pub use dimensions::{pivot_angle, ConnectorDimensions};
pub use system::ConnectorSystem;
pub(crate) use dimensions::radial_unit_from_axis;
