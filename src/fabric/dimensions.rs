//! Fabric dimensions: structure size and interval geometry.

use crate::connector::ConnectorDimensions;
use crate::fabric::physics::Physics;
use crate::fabric::material::Material;
use crate::units::{Grams, GramsPerMeter, Meters};

/// All physical dimensions for a fabric: structure size and interval geometry.
#[derive(Clone, Debug)]
pub struct FabricDimensions {
    pub altitude: Meters,
    pub scale: Meters,
    pub push_radius: Meters,
    pub pull_radius: Meters,
    /// Plan-level connector configuration; `None` (the default) means the
    /// fabric has no physical connectors and they play no role anywhere.
    /// Opt in via `with_connector` or `with_locked_bend_magnitudes`. Moved
    /// into `Fabric::connector` (as a `ConnectorSystem`) at fabric
    /// construction; `None` afterwards.
    pub connector: Option<ConnectorDimensions>,
    /// Head + per-joint hardware share. Back-calibrated to total mass; refine when the full parts list lands.
    pub joint_mass: Grams,
    pub push_density: GramsPerMeter,
    /// Combined linear density of telescoping inner tubes (one outer-length per strut). Mass-reporting only.
    pub inner_push_density: GramsPerMeter,
    /// Mass of one cable-end fork termination; counted twice per `Role::Pulling` interval.
    pub pull_end_mass: Grams,
}

impl Default for FabricDimensions {
    fn default() -> Self {
        Self {
            altitude: Meters(7.5),
            scale: Meters(1.0),
            push_radius: Meters(0.02),
            pull_radius: Meters(0.007),
            connector: None,
            joint_mass: Grams(1800.0),
            push_density: GramsPerMeter(800.0),
            inner_push_density: GramsPerMeter(560.0),
            pull_end_mass: Grams(160.0),
        }
    }
}

impl FabricDimensions {
    pub fn with_altitude(mut self, altitude: Meters) -> Self {
        self.altitude = altitude;
        self
    }

    pub fn with_scale(mut self, scale: Meters) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_joint_mass(mut self, mass: Grams) -> Self {
        self.joint_mass = mass;
        self
    }

    pub fn with_push_density(mut self, density: GramsPerMeter) -> Self {
        self.push_density = density;
        self
    }

    /// Equip the fabric with physical connector hardware (default dimensions),
    /// enabling attachment points, bend optimisation, and connector rendering.
    pub fn with_connector(mut self) -> Self {
        self.connector = Some(ConnectorDimensions::default());
        self
    }

    /// Lock the snap magnitudes to a fixed inventory (e.g. parts already in production).
    /// `Fabric::recompute_bend_magnitudes` becomes a no-op and these values are used
    /// for snapping at every CSV export. Values are whole non-negative degrees, sorted
    /// ascending; the optimiser's normal output respects the same shape.
    pub fn with_locked_bend_magnitudes(mut self, magnitudes: Vec<f32>) -> Self {
        let connector = self.connector.get_or_insert_with(ConnectorDimensions::default);
        connector.bend_count = magnitudes.len();
        connector.bend_magnitudes = magnitudes;
        connector.bend_magnitudes_locked = true;
        self
    }

    /// Linear density for a material type, using configurable push density.
    pub fn linear_density(&self, material: Material, physics: &Physics) -> GramsPerMeter {
        let base = match material {
            Material::Push => self.push_density,
            _ => material.base_linear_density(),
        };
        GramsPerMeter(base.0 * physics.mass_multiplier())
    }
}
