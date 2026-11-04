//! A strut's aluminium tubes: an outer tube with an inner tube telescoping out
//! of each end, pinned at the length the strut needs. A fabric has one or more
//! tube sets; each strut is made from the first set its length fits.

use crate::units::{Meters, Unit};

/// The outer tube, 40×3 mm EN AW-6060 T66.
pub const OUTER_RADIUS: Meters = Meters(0.020);
pub const OUTER_BORE_RADIUS: Meters = Meters(0.017);
/// The inner tubes, 33×3 mm, sliding in the outer's 34 mm bore.
pub const INNER_RADIUS: Meters = Meters(0.0165);
pub const INNER_BORE_RADIUS: Meters = Meters(0.0135);
/// Open Claw's limits: an inner tube stays at least this far inside the outer...
pub const MIN_OVERLAP: Meters = Meters(0.31);
/// ...and sticks out of it at least this far.
pub const MIN_PROTRUSION: Meters = Meters(0.17);

/// One outer tube and the two inner tubes that telescope out of it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TubeSet {
    pub outer: Meters,
    pub inner: Meters,
}

impl TubeSet {
    pub fn shortest(&self) -> Meters {
        self.outer + MIN_PROTRUSION * 2.0
    }

    pub fn longest(&self) -> Meters {
        Meters(self.outer.f32() + 2.0 * (self.inner.f32() - MIN_OVERLAP.f32()))
    }

    fn fits(&self, length: Meters) -> bool {
        self.shortest() <= length && length <= self.longest()
    }

    /// How far `length` is outside this set's range.
    fn miss(&self, length: Meters) -> f32 {
        (self.shortest().f32() - length.f32()).max(length.f32() - self.longest().f32()).max(0.0)
    }
}

/// Open Claw's tubes (247TailorSteel order NL26140673).
pub const OPEN_CLAW_TUBES: &[TubeSet] = &[TubeSet { outer: Meters(1.471), inner: Meters(0.780) }];

/// PROPOSAL for Headless Hug, whose struts are too short for Open Claw's: a
/// long set and a short one, awaiting ENS's hole scheme.
pub const HEADLESS_HUG_TUBES: &[TubeSet] = &[
    TubeSet { outer: Meters(1.350), inner: Meters(0.850) },
    TubeSet { outer: Meters(0.900), inner: Meters(0.700) },
];

/// The set a strut `length` long is made from, and whether it fits: if none
/// does, the one it misses least.
pub fn tube_set(sets: &[TubeSet], length: Meters) -> (TubeSet, bool) {
    match sets.iter().find(|set| set.fits(length)) {
        Some(set) => (*set, true),
        None => {
            let nearest = sets.iter().min_by(|a, b| a.miss(length).total_cmp(&b.miss(length)));
            (*nearest.expect("a fabric has at least one tube set"), false)
        }
    }
}
