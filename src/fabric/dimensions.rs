//! Fabric dimensions: structure size and interval geometry.

use crate::connector::strut::TubeSet;
use crate::connector::ConnectorDimensions;
use crate::fabric::physics::Physics;
use crate::fabric::interval::Role;
use crate::fabric::material::Material;
use crate::units::{Degrees, Grams, GramsPerMeter, Meters, Newtons, Unit};

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
    /// The cable itself, per metre.
    pub cable_density: GramsPerMeter,
    /// At each cable end: its terminal, and on a strut, its ring and POM washer.
    pub cable_end: Grams,
    /// A node plate, less the rings its cable ends are counted with.
    pub plate: Grams,
    /// A hinge's clevis, pin and eye.
    pub hinge: Grams,
    /// A strut's axial stiffness, EA.
    pub strut_stiffness: Newtons,
    /// A cable's axial stiffness, EA.
    pub cable_stiffness: Newtons,
}


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
        // 6 mm steel cable.
        cable_density: GramsPerMeter(130.0),
        // Louis Ruys 37260006 jaw terminal (160 g, vault figure), ring with
        // boss and cross-tube (61 g), POM washer (3 g), 7 mm more bolt (6 g).
        cable_end: Grams(230.0),
        // 12 mm equilateral plate, 60 mm hole to hole (469 g), less the three
        // rings and washers (70 g each) its cable ends were counted with.
        plate: Grams(259.0),
        // Ganter GN 751-12-24-M12 clevis with its pin (168 g, as Festo's
        // SG-M12) and GN 752-12-M12 eye (100 g, from its dimensions).
        hinge: Grams(268.0),
        // E 69 GPa: outer tube 349 mm² (24 MN), inner 283 mm² (20 MN), about
        // half the length each. The locking pins' play isn't included.
        strut_stiffness: Newtons(2.1e7),
        // 6 mm 7×19 galvanized (Louis Ruys 36001106): about 14 mm² of steel at a
        // rope modulus near 100 GPa. ESTIMATE until a datasheet gives it.
        cable_stiffness: Newtons(1.4e6),
    };

    /// A Medium piece, as the 2021 Headless Hug was made: 8×1 mm aluminium
    /// tube, an M6 bolt, nut and washer at each strut end, Dyneema cord tied
    /// on, no plates, and hinges made by hand.
    pub const MEDIUM: Hardware = Hardware {
        // 22 mm² of aluminium at 2700 kg/m³.
        strut_density: GramsPerMeter(59.4),
        // A long M6 bolt, nut and washers: what makes the 2021 Headless Hug
        // weigh about 4 kg.
        strut_end: Grams(20.0),
        // 1.5 mm Dyneema braid. ESTIMATE until weighed.
        cable_density: GramsPerMeter(1.2),
        // A knot or spliced eye.
        cable_end: Grams(0.5),
        plate: Grams(0.0),
        // As much as a bolt, nut and washers.
        hinge: Grams(20.0),
        // E 69 GPa on 22 mm² is 1.5 MN, but a strut this short and light
        // rings faster than the 50 µs tick can follow: a third of it is
        // stable, still ten times the cords', which give the piece its give.
        strut_stiffness: Newtons(5.0e5),
        // About 1 mm² of fibre in the braid at an effective 50 GPa. ESTIMATE
        // until a pull test gives it.
        cable_stiffness: Newtons(5.0e4),
    };
}

/// How big a fabric is made. Large is the plan as designed; Medium is the
/// same plan made about `MEDIUM_HEIGHT` tall from `Hardware::MEDIUM`,
/// simulated at that size and standing on the wobble table; Small is the
/// Large fabric with its lengths shown at a model's scale.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, strum::Display, strum::EnumString)]
#[strum(ascii_case_insensitive)]
pub enum Size {
    #[default]
    Large,
    Medium,
    Small,
}

impl Size {
    /// A fabric's title, saying how big it is made.
    pub fn title(self, name: &str) -> String {
        format!("{name} · {self}")
    }
}

/// A Medium piece's pretension, as part of its plan's: a piece tied by
/// hand is not as tight as one built to its plan, and wobbles more.
pub const MEDIUM_PRETENSION: f32 = 0.4;

/// How tall a Medium piece stands, near enough.
pub const MEDIUM_HEIGHT: Meters = Meters(1.5);

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
    /// Open the fabric with its connector hardware showing, as if C were
    /// pressed: for the fabrics that are, or are meant to be, built for real.
    pub connectors_shown: bool,
    pub size: Size,
    /// A Medium piece's wobble table, this far across its corners.
    pub table: Option<Meters>,
    /// How far it reaches from its middle once built, when known, for the
    /// camera to keep that far back while it grows.
    pub reach: Option<Meters>,
    /// Turned this far about the vertical from the start of a build on a
    /// table, so its symmetry stands square to the table's three hangers.
    pub table_turn: Degrees,
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
            connectors_shown: false,
            size: Size::Large,
            table: None,
            reach: None,
            table_turn: Degrees(0.0),
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

    /// On a wobble table, turned `turn` from the way it is built.
    pub fn with_table_turn(mut self, turn: Degrees) -> Self {
        self.table_turn = turn;
        self
    }

    /// Show the connector hardware as soon as the fabric is built.
    pub fn with_connectors_shown(mut self) -> Self {
        self.connectors_shown = true;
        self
    }

    /// Made Medium: from `Hardware::MEDIUM`, with no connector hardware, its
    /// struts and cords drawn their own thickness. A Medium plan is built
    /// small (`FabricPlan::at_size`) and turned Medium just before
    /// pretensing.
    pub fn medium(self) -> Self {
        let mut medium = self.drawn_medium();
        medium.size = Size::Medium;
        medium.strut_mass = None;
        medium.with_hardware(Hardware::MEDIUM)
    }

    /// Drawn as a Medium piece, from the start of its build: struts and
    /// cords their own thickness, and no connector hardware.
    pub fn drawn_medium(mut self) -> Self {
        self.connector = None;
        self.connectors_shown = false;
        self.push_radius = Meters(0.004);
        self.pull_radius = Meters(0.00075);
        self
    }

    /// Equip the fabric with physical connector hardware (default dimensions),
    /// enabling attachment points, slot assignment, and connector rendering.
    pub fn with_connector(mut self) -> Self {
        self.connector = Some(ConnectorDimensions::default());
        self
    }

    /// Make the struts from these tubes, equipping connectors if not yet.
    pub fn with_tube_sets(mut self, tube_sets: &'static [TubeSet]) -> Self {
        self.connector.get_or_insert_with(ConnectorDimensions::default).tube_sets = tube_sets;
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

    /// The spring constant of an interval of `material`, `length` long, in the
    /// simulation's millinewtons per metre: the real parts' stiffness when the
    /// fabric counts its hardware.
    pub fn spring_constant(&self, material: Material, length: Meters, physics: &Physics) -> f32 {
        let real = self.hardware.and_then(|hardware| match material {
            Material::Push => Some(hardware.strut_stiffness),
            Material::Pull => Some(hardware.cable_stiffness),
            Material::Spring => None,
        });
        match real {
            Some(stiffness) => stiffness.to_millinewtons() / length.f32().max(0.001) * physics.rigidity_multiplier(),
            None => material.spring_constant(length, physics),
        }
    }

    /// Linear density for a material type, using configurable push density.
    pub fn linear_density(&self, material: Material, physics: &Physics) -> GramsPerMeter {
        let base = match (material, self.hardware) {
            (Material::Pull, Some(hardware)) => hardware.cable_density,
            (Material::Push, _) => self.push_density,
            (_, _) => material.base_linear_density(),
        };
        GramsPerMeter(base.0 * physics.mass_multiplier())
    }
}
