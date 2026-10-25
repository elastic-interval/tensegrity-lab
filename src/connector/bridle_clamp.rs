//! A bridle clamp: one continuous cable runs A → clamp → B over a grooved
//! saddle held between two cheek plates, a keeper bolted through the cheeks
//! pressing it into the groove so it can't slip. The clamp hangs from the
//! strut end's connector by a shackle through the cheeks' eye, on the side
//! the cable wraps — like a fixed sheave in a block. No terminals at the
//! clamp, and any bend up to what the saddle allows.
//!
//! The simulation knows the clamp only as the joint where the cable's two
//! straight legs meet. The saddle sits inside the angle between them, so
//! both legs leave it on tangents. The simulation fits the clamp's link
//! to within a few millimetres of its shackle (`Fabric::hang_clamps`); the
//! clamp is then slid the rest of the way.

use crate::connector::fork::{EYE_OFFSET, SADDLE_RADIUS, SHACKLE_LENGTH};
use crate::fabric::IntervalKey;
use crate::units::Unit;
use glam::Vec3;

#[derive(Clone, Debug)]
pub struct BridleClamp {
    /// Where the cable's straight legs meet: the simulated joint, slid
    /// along the shackle to hang its eye a shackle's length from the pivot.
    pub corner: Vec3,
    /// Centre of the saddle: the part's origin.
    pub saddle: Vec3,
    /// From the saddle toward the eye and the strut it hangs from.
    pub toward_link: Vec3,
    /// Square to the plane of the cable, through the cheeks.
    pub normal: Vec3,
    /// Where the shackle's pin passes through the cheeks.
    pub eye: Vec3,
    /// How far the cable turns at the clamp, in degrees.
    pub bend: f32,
    /// The cable's two sides: each with its far end's pivot and the point
    /// where it leaves the saddle.
    pub sides: [(IntervalKey, Vec3, Vec3); 2],
}

impl BridleClamp {
    /// The clamp at `corner`, its cable's sides aimed as given. Gripped by
    /// its keeper, the clamp may turn about its saddle in the plane of the
    /// cable, so it faces the pivot it `hangs` from, where that is known,
    /// and hangs a shackle's length from it.
    pub fn new(corner: Vec3, sides: [(IntervalKey, Vec3); 2], hangs: Option<Vec3>) -> BridleClamp {
        let [a, b] = sides.map(|(_, aim)| (aim - corner).normalize());
        let between = a.angle_between(b).max(1e-3);
        let inward = (a + b).normalize_or(a.any_orthonormal_vector());
        let radius = SADDLE_RADIUS.f32();
        let saddle = corner + inward * (radius / (between / 2.0).sin());
        let to_tangent = radius / (between / 2.0).tan();
        let normal = a.cross(b).normalize_or(inward.any_orthonormal_vector());
        let toward_link = hangs
            .map(|pivot| (pivot - saddle).reject_from(normal).normalize_or(-inward))
            .unwrap_or(-inward);
        let eye = saddle + toward_link * EYE_OFFSET.f32();
        let slide = hangs.map_or(Vec3::ZERO, |pivot| {
            let reach = pivot - eye;
            reach - reach.normalize_or_zero() * SHACKLE_LENGTH.f32()
        });
        let corner = corner + slide;
        BridleClamp {
            corner,
            saddle: saddle + slide,
            toward_link,
            normal,
            eye: eye + slide,
            bend: 180.0 - between.to_degrees(),
            sides: [
                (sides[0].0, sides[0].1, corner + a * to_tangent),
                (sides[1].0, sides[1].1, corner + b * to_tangent),
            ],
        }
    }

    /// Where `cable` leaves the saddle, if it is one of this clamp's sides.
    pub fn tangent(&self, cable: IntervalKey) -> Option<Vec3> {
        self.sides.iter().find(|(key, ..)| *key == cable).map(|(_, _, tangent)| *tangent)
    }
}
