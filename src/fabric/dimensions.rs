//! Fabric dimensions: structure size and interval geometry.

use crate::connector::ConnectorDimensions;
use crate::fabric::physics::Physics;
use crate::fabric::interval::Role;
use crate::fabric::material::Material;
use crate::units::{Grams, GramsPerMeter, Meters, NewtonsPerMeter, Unit};

/// Masses counted from a fabric's real parts, in place of a flat mass per
/// joint. A strut's tubes are spread over its length and each end carries its
/// cap and bolt; every cable end carries its terminal and the ring it hangs
/// from; node plates and hinges sit on their own joints.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hardware {
    /// The telescoping tubes, per metre of assembled strut.
    pub strut_density: GramsPerMeter,
    /// At each strut end: its cap, bolt, nut and washers.
    pub strut_end: Grams,
    /// At each cable end: its terminal, and on a strut, its ring and POM washer.
    pub cable_end: Grams,
    /// A node plate, less the rings its cable ends are counted with.
    pub plate: Grams,
    /// A hinge's rod end, fork and pin bolt.
    pub hinge: Grams,
    /// A strut's axial stiffness, EA, in newtons.
    pub strut_stiffness: f32,
    /// A cable's axial stiffness, EA, in newtons.
    pub cable_stiffness: f32,
}

// The simulation's forces are in grams × m/s².
const SIMULATION_PER_NEWTON: f32 = 1000.0;

impl Hardware {
    /// Open Claw's struts and hardware (FluxeArt vault, 247TailorSteel order
    /// NL26140673 and Open Claw's order list), aluminium 2700 and steel
    /// 7850 kg/m³, with Headless Hug's rings, plates and hinges.
    pub const OPEN_CLAW: Hardware = Hardware {
        // An outer tube 40×3 × 1471 mm (1385 g) and two inner tubes
        // 33×3 × 780 mm (595 g each) per strut, over Open Claw's mean
        // assembled length of 2.074 m: telescoping keeps their proportions.
        strut_density: GramsPerMeter(1241.6),
        // Cap: Ø36 × 5 mm tapped plate (35 g) on a 40×3 × 40 mm steel tube
        // (110 g); M12×55 bolt (60 g), nyloc (17), DIN 9021 washer (22),
        // spring washer and nut cap (10).
        strut_end: Grams(254.0),
        // Louis Ruys 37260006 jaw terminal (160 g, vault figure), ring with
        // boss and cross-tube (61 g), POM washer (3 g), 7 mm more bolt (6 g).
        cable_end: Grams(230.0),
        // 12 mm equilateral plate, 60 mm hole to hole (469 g), less the three
        // rings and washers (70 g each) its cable ends were counted with.
        plate: Grams(259.0),
        // GIKR12-PW rod end (119 g), fork (about 250 g, ESTIMATE), pin bolt (60 g).
        hinge: Grams(429.0),
        // E 69 GPa: outer tube 349 mm² (24 MN), inner 283 mm² (20 MN), about
        // half the length each. The locking pins' play isn't included.
        strut_stiffness: 2.1e7,
        // 6 mm 7×19 galvanized (Louis Ruys 36001106): about 14 mm² of steel at a
        // rope modulus near 100 GPa. ESTIMATE until a datasheet gives it.
        cable_stiffness: 1.4e6,
    };
}

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
    /// When set, masses come from the real parts (`Hardware`) and joints have
    /// no flat `joint_mass` of their own.
    pub hardware: Option<Hardware>,
}

impl Default for FabricDimensions {
    fn default() -> Self {
        Self {
            altitude: Meters(7.5),
            scale: Meters(1.0),
            push_radius: Meters(0.02),
            // Real cables are 6 mm; render them true to size.
            pull_radius: Meters(0.003),
            connector: None,
            joint_mass: Grams(1800.0),
            push_density: GramsPerMeter(800.0),
            inner_push_density: GramsPerMeter(560.0),
            strut_mass: None,
            pull_end_mass: Grams(160.0),
            hardware: None,
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

    /// Count masses from these real parts instead of a flat mass per joint.
    pub fn with_hardware(mut self, hardware: Hardware) -> Self {
        self.hardware = Some(hardware);
        self.joint_mass = Grams(0.0);
        self
    }

    /// Equip the fabric with physical connector hardware (default dimensions),
    /// enabling attachment points, slot assignment, and connector rendering.
    pub fn with_connector(mut self) -> Self {
        self.connector = Some(ConnectorDimensions::default());
        self
    }

    /// An interval's own mass, spread along it, and the mass of the hardware at
    /// each of its ends.
    pub fn interval_masses(&self, material: Material, role: Role, length: Meters, physics: &Physics) -> (Grams, Grams) {
        let scaled = |mass: Grams| mass * physics.mass_multiplier();
        match (self.strut_mass, self.hardware) {
            (Some(strut_mass), _) if role == Role::Pushing => (strut_mass, Grams(0.0)),
            (_, Some(hardware)) if material == Material::Push => {
                (scaled(hardware.strut_density * length), scaled(hardware.strut_end))
            }
            (_, Some(hardware)) if material == Material::Pull => {
                (self.linear_density(material, physics) * length, scaled(hardware.cable_end))
            }
            _ => (self.linear_density(material, physics) * length, Grams(0.0)),
        }
    }

    /// The spring constant of an interval of `material`, `length` long: the
    /// real parts' stiffness when the fabric counts its hardware.
    pub fn spring_constant(&self, material: Material, length: Meters, physics: &Physics) -> NewtonsPerMeter {
        let real = self.hardware.and_then(|hardware| match material {
            Material::Push => Some(hardware.strut_stiffness),
            Material::Pull => Some(hardware.cable_stiffness),
            Material::Spring => None,
        });
        match real {
            Some(stiffness) => NewtonsPerMeter(
                stiffness * SIMULATION_PER_NEWTON / length.f32().max(0.001) * physics.rigidity_multiplier(),
            ),
            None => material.spring_constant(length, physics),
        }
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
