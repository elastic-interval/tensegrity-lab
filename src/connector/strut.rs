//! A strut's aluminium tubes: an outer tube with an inner tube telescoping out
//! of each end, pinned at the length the strut needs. ENS's vernier: the inner
//! tube has holes all along it at 30 mm, the outer six at each end at 50 mm, and
//! two pins 150 mm apart (5 × 30 = 3 × 50) always go through both. Each 10 mm
//! of extension brings another pair of outer holes into line: 1-4, 2-5, 3-6,
//! then 1-4 again on the next inner hole. The layout is Open Claw's (its Rhino
//! model of the tubes). A fabric has one or more tube sets; each strut is made
//! from the first set it fits.

use crate::units::{Meters, Unit};

/// The outer tube, 40×3 mm EN AW-6060 T66.
pub const OUTER_RADIUS: Meters = Meters(0.020);
pub const OUTER_BORE_RADIUS: Meters = Meters(0.017);
/// The inner tubes, 33×3 mm, sliding in the outer's 34 mm bore.
pub const INNER_RADIUS: Meters = Meters(0.0165);
pub const INNER_BORE_RADIUS: Meters = Meters(0.0135);
/// The pin holes, Ø10.5 through both walls.
pub const HOLE_RADIUS: Meters = Meters(0.00525);

// Hole layout in millimetres, each first hole this far from its tube's end.
const OUTER_FIRST_HOLE: i32 = 25;
const OUTER_PITCH: i32 = 50;
const OUTER_HOLES: i32 = 6;
const INNER_FIRST_HOLE: i32 = 60;
const INNER_PITCH: i32 = 30;
// The two pins, 150 mm apart, in pitches of each tube.
const OUTER_PIN_STEP: i32 = 3;
const INNER_PIN_STEP: i32 = 5;

/// The most a strut may miss its length: half a 10 mm step.
const TOLERANCE: i32 = 5;

/// One end's pins: the inner tube's holes `inner` and `inner + 5`, counted
/// from its outer end, in the outer tube's holes `outer` and `outer + 3`,
/// counted from that end of it, both from 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PinSetting {
    pub inner: u8,
    pub outer: u8,
}

impl std::fmt::Display for PinSetting {
    /// As the build crew counts, from 1: the inner hole of the outer pin, then
    /// the outer pair, e.g. `12:3-6`.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let outer = self.outer as i32 + 1;
        write!(f, "{}:{}-{}", self.inner + 1, outer, outer + OUTER_PIN_STEP)
    }
}

/// How a strut is set: the pins at its two ends, and how far its tubes' length
/// is from the length wanted, positive when longer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrutSetting {
    pub ends: [PinSetting; 2],
    pub error: Meters,
}

/// One outer tube and the two inner tubes that telescope out of it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TubeSet {
    pub outer: Meters,
    pub inner: Meters,
}

fn mm(length: Meters) -> i32 {
    (length.f32() * 1000.0).round() as i32
}

fn inner_hole_count(length: Meters) -> i32 {
    (mm(length) - 2 * INNER_FIRST_HOLE) / INNER_PITCH + 1
}

/// Where an outer tube `length` long has its holes, from one end.
pub fn outer_holes(length: Meters) -> Vec<Meters> {
    (0..OUTER_HOLES)
        .flat_map(|hole| {
            let from_end = OUTER_FIRST_HOLE + OUTER_PITCH * hole;
            [from_end, mm(length) - from_end]
        })
        .map(|at| Meters(at as f32 / 1000.0))
        .collect()
}

/// Where an inner tube `length` long has its holes, from one end.
pub fn inner_holes(length: Meters) -> Vec<Meters> {
    (0..inner_hole_count(length))
        .map(|hole| Meters((INNER_FIRST_HOLE + INNER_PITCH * hole) as f32 / 1000.0))
        .collect()
}

impl TubeSet {

    /// How far, in millimetres, the inner tube sticks out of the outer at `setting`.
    fn protrusion(&self, setting: PinSetting) -> i32 {
        INNER_FIRST_HOLE + INNER_PITCH * setting.inner as i32 - OUTER_FIRST_HOLE - OUTER_PITCH * setting.outer as i32
    }

    /// Every way one end can be pinned, with its protrusion, without the two
    /// inner tubes meeting inside the outer.
    fn pin_settings(&self) -> Vec<(i32, PinSetting)> {
        let (outer, inner) = (mm(self.outer), mm(self.inner));
        let mut settings = Vec::new();
        for outer_hole in 0..OUTER_HOLES - OUTER_PIN_STEP {
            for inner_hole in 0..inner_hole_count(self.inner) - INNER_PIN_STEP {
                let setting = PinSetting { inner: inner_hole as u8, outer: outer_hole as u8 };
                let protrusion = self.protrusion(setting);
                if 2 * (inner - protrusion) <= outer {
                    settings.push((protrusion, setting));
                }
            }
        }
        settings
    }

    /// The pins at both ends making the strut nearest `length`, the two ends
    /// as nearly alike as can be.
    pub fn setting(&self, length: Meters) -> StrutSetting {
        let wanted = mm(length) - mm(self.outer);
        let settings = self.pin_settings();
        let mut best = ((i32::MAX, i32::MAX), [settings[0].1; 2], 0);
        for &(alpha, alpha_setting) in &settings {
            for &(omega, omega_setting) in &settings {
                let error = alpha + omega - wanted;
                let rank = (error.abs(), (alpha - omega).abs());
                if rank < best.0 {
                    best = (rank, [alpha_setting, omega_setting], error);
                }
            }
        }
        StrutSetting { ends: best.1, error: Meters(best.2 as f32 / 1000.0) }
    }
}

/// Open Claw's tubes (247TailorSteel order NL26140673): struts of 1561–2561 mm.
pub const OPEN_CLAW_TUBES: &[TubeSet] = &[TubeSet { outer: Meters(1.471), inner: Meters(0.780) }];

/// Headless Hug: Open Claw's tubes for its long struts, and a PROPOSED short
/// set, drilled the same way, for 1200–1630 mm.
pub const HEADLESS_HUG_TUBES: &[TubeSet] = &[
    TubeSet { outer: Meters(1.471), inner: Meters(0.780) },
    TubeSet { outer: Meters(0.900), inner: Meters(0.600) },
];

/// The set a strut `length` long is made from and how it's set: the first set
/// that makes it to within half a step, or if none does, the nearest, with
/// `fits` false.
pub fn tube_set(sets: &[TubeSet], length: Meters) -> (TubeSet, StrutSetting, bool) {
    let fits = |setting: &StrutSetting| mm(setting.error).abs() <= TOLERANCE;
    let settings: Vec<(TubeSet, StrutSetting)> = sets.iter().map(|set| (*set, set.setting(length))).collect();
    let (set, setting) = settings
        .iter()
        .find(|(_, setting)| fits(setting))
        .or_else(|| settings.iter().min_by_key(|(_, setting)| mm(setting.error).abs()))
        .copied()
        .expect("a fabric has at least one tube set");
    (set, setting, fits(&setting))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Open Claw's tubes make 1561 to 2561 mm in 10 mm steps, and nothing
    /// outside that.
    #[test]
    fn open_claw_range() {
        let set = OPEN_CLAW_TUBES[0];
        let protrusions: Vec<i32> = set.pin_settings().iter().map(|(p, _)| *p).collect();
        assert_eq!(protrusions.iter().min(), Some(&45));
        assert_eq!(protrusions.iter().max(), Some(&545));
        assert!(protrusions.iter().all(|p| p % 10 == 5));
        for length in [1561, 1600, 2000, 2083, 2561] {
            assert!(tube_set(OPEN_CLAW_TUBES, Meters(length as f32 / 1000.0)).2, "{length} mm");
        }
        assert!(!tube_set(OPEN_CLAW_TUBES, Meters(1.54)).2);
        assert!(!tube_set(OPEN_CLAW_TUBES, Meters(2.58)).2);
    }

    /// Each 10 mm of extension brings the next outer pair into line: 1-4, 2-5,
    /// 3-6, then 1-4 on the next inner hole.
    #[test]
    fn vernier_steps() {
        let settings = OPEN_CLAW_TUBES[0].pin_settings();
        let at = |protrusion: i32| settings.iter().find(|(p, _)| *p == protrusion).unwrap().1.to_string();
        assert_eq!([at(245), at(255), at(265), at(275)], ["8:1-4", "10:2-5", "12:3-6", "9:1-4"]);
    }

    /// At full stretch the last pin pair of the inner tube is in the outer's
    /// holes 1 and 4.
    #[test]
    fn full_stretch_pins() {
        let longest = OPEN_CLAW_TUBES[0].setting(Meters(2.561));
        assert_eq!(longest.ends, [PinSetting { inner: 17, outer: 0 }; 2]);
        assert_eq!(mm(longest.error), 0);
    }
}
