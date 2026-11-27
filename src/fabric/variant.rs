//! What a fabric is made for. Its plan is the Large one, as designed; every
//! other way of making it is a variant of that, saying here everything it
//! changes, and nothing else changes it.

use crate::fabric::dimensions::{Hardware, MEDIUM_HEIGHT, MEDIUM_PRETENSION};
use crate::units::Meters;
use glam::Vec3;

/// The wobble table a variant stands on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Table {
    /// None: it stands on the ground.
    None,
    /// The plan's own (`FabricDimensions::with_table`), or one fitted to
    /// where it stands.
    Planned,
    /// This far from the middle to a corner, whatever the plan says.
    Given(Meters),
}

/// How a fabric is made unlike its plan.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Variant {
    /// What it is called, in the title.
    pub name: &'static str,
    /// Made this tall, every length shrunk with it, or as designed. A
    /// smaller one falls quicker and its guy lines reach less far.
    pub height: Option<Meters>,
    /// How much of its plan's pretension it keeps.
    pub pretension: f32,
    /// Made from these parts from pretensing on, drawn their own
    /// thickness from the start; or the plan's.
    pub materials: Option<Hardware>,
    /// Its plan's connector hardware and node plates (bridles), or none.
    pub connectors: bool,
    pub table: Table,
    /// Its lengths shown at this scale, a tenth for 1:10, with caliper
    /// readings; or at real size.
    pub model: Option<f32>,
    /// Built in fast-forward.
    pub fast_build: bool,
    /// Its seed placed this high, or where the plan places it; and moved
    /// this far across, metres.
    pub seed_altitude: Option<Meters>,
    pub seed_shift: Vec3,
}

impl Variant {
    /// The plan as designed.
    pub const LARGE: Variant = Variant {
        name: "Large",
        height: None,
        pretension: 1.0,
        materials: None,
        connectors: true,
        table: Table::None,
        model: None,
        fast_build: false,
        seed_altitude: None,
        seed_shift: Vec3::ZERO,
    };

    /// Built by hand as the 2021 Headless Hug was, about `MEDIUM_HEIGHT`
    /// tall: 8×1 mm aluminium tube, 1.5 mm Dyneema, hinges made by hand, no
    /// connectors or node plates, tied less tight than its plan, on its
    /// wobble table.
    pub const MEDIUM: Variant = Variant {
        name: "Medium",
        height: Some(MEDIUM_HEIGHT),
        pretension: MEDIUM_PRETENSION,
        materials: Some(Hardware::MEDIUM),
        connectors: false,
        table: Table::Planned,
        model: None,
        fast_build: true,
        seed_altitude: None,
        seed_shift: Vec3::ZERO,
    };

    /// A model to build from: the Large fabric with its lengths shown at
    /// `scale`, 10 for 1:10.
    pub fn small(scale: f32) -> Variant {
        Variant { name: "Small", model: Some(1.0 / scale), fast_build: true, ..Variant::LARGE }
    }

    /// As designed: neither made smaller nor a model. Only such a fabric
    /// animates, takes the wind, is taken apart, or shows its parts.
    pub fn as_designed(&self) -> bool {
        self.height.is_none() && self.model.is_none()
    }

    /// A fabric's title, saying how it is made.
    pub fn title(&self, name: &str) -> String {
        format!("{name} · {}", self.name)
    }

    /// On a wobble table, to quake.
    pub fn on_table(&self) -> bool {
        self.table != Table::None
    }
}
