//! Physical connector hardware for large-scale builds: disc/cap/tab geometry,
//! cable-to-slot assignment, and bend-magnitude optimisation. Adjacent to the
//! simulation — nothing here participates in the physics tick.

pub mod attachment;
pub mod bend_optimizer;
pub mod dimensions;
pub mod system;

pub use dimensions::{tab_angle, ConnectorDimensions};
pub use system::ConnectorSystem;
pub(crate) use dimensions::radial_unit_from_axis;
