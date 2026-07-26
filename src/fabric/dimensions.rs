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
    /// Opt in via `with_connector`. Moved into `Fabric::connector` (as a
    /// `ConnectorSystem`) at fabric construction; `None` afterwards.
    pub connector: Option<ConnectorDimensions>,
    /// Head + per-joint hardware share. Back-calibrated to total mass; refine when the full parts list lands.
    pub joint_mass: Grams,
    pub push_density: GramsPerMeter,
    /// Combined linear density of telescoping inner tubes (one outer-length per strut).
    /// Only used for mass reporting in the length-based model (`strut_mass == None`).
    pub inner_push_density: GramsPerMeter,
    /// Set when every strut is the same telescoping hardware unit. Gives each strut
    /// this constant mass (outer + inner tubes) regardless of assembled length, and
    /// lumps cable-end terminals at the nodes — the real self-weight model. `None`
    /// keeps the legacy length-proportional `push_density × length`.
    pub strut_mass: Option<Grams>,
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
            strut_mass: None,
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

    /// Declare that every strut is the same telescoping unit of this mass, switching
    /// the fabric to the real self-weight model (constant strut mass + node terminals).
    pub fn with_strut_mass(mut self, mass: Grams) -> Self {
        self.strut_mass = Some(mass);
        self
    }

    /// Equip the fabric with physical connector hardware (default dimensions),
    /// enabling attachment points, slot assignment, and connector rendering.
    pub fn with_connector(mut self) -> Self {
        self.connector = Some(ConnectorDimensions::default());
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
