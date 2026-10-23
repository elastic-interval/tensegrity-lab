//! The hardware at a cable end: the jaw terminal (clevis) that pivots on
//! the connector's cross-tube pin — a bought Louis Ruys 37260006 stainless
//! jaw terminal for 6 mm wire, held by a plain headed pin with a split ring.
//!
//! Also provides the capsule model of a whole assembled connector end
//! (jaw, pin, barrel), used to mark pairs that would physically collide —
//! real construction issues, surfaced once per slot-assignment rebuild and
//! highlighted by the renderer.

use crate::connector::attachment::segment_segment_distance;
use crate::units::{Degrees, Meters, Unit};
use glam::Vec3;

// Cross-tube of the connector part (see docs/connectors.md).
pub const TUBE_RADIUS: f32 = 0.009; // D_tube / 2, metres — 18x2.5 E235+N precision tube
pub const TUBE_LENGTH: f32 = 0.0135; // L_tube, metres — wider than the 6 mm cables from every angle

/// A jaw terminal in its own frame: pin centre at the origin, the barrel
/// running out along the cable, the pin across the jaw gap. Lengths along
/// the cable are measured from the pin centre.
pub struct ClevisDimensions {
    pub jaw_gap: Meters,
    pub pin_radius: Meters,
    pub pin_to_jaw_base: Meters,
    pub overall_length: Meters,
    pub barrel_length: Meters,
    pub barrel_radius: Meters,
    pub barrel_taper: Meters,
    pub barrel_tip_radius: Meters,
    pub neck_length: Meters,
    pub plate_thickness: Meters,
    pub plate_width: Meters,
    pub pin_to_tip: Meters,
    pub pin_head_radius: Meters,
    pub pin_head_height: Meters,
    pub pin_overhang: Meters,
    pub split_ring_radius: Meters,
    pub split_ring_wire: Meters,
}

// Louis Ruys 37260006 stainless jaw terminal, 6 mm, AISI-316 (catalogue p.96).
// "scaled" values are measured off the catalogue drawing.
pub const CLEVIS: ClevisDimensions = ClevisDimensions {
    jaw_gap: Meters(0.014),            // A, datasheet (and measured)
    pin_radius: Meters(0.00485),       // D = 9.7, datasheet
    pin_to_jaw_base: Meters(0.0254),   // lg 20.5 + D/2, datasheet
    overall_length: Meters(0.106),     // L, pin centre to barrel end, datasheet
    barrel_length: Meters(0.063),      // L1, datasheet
    barrel_radius: Meters(0.006),      // scaled
    barrel_taper: Meters(0.008),       // scaled, conical end
    barrel_tip_radius: Meters(0.0035), // ESTIMATE, just over the 6 mm wire
    neck_length: Meters(0.007),        // scaled
    plate_thickness: Meters(0.0038),   // scaled
    plate_width: Meters(0.020),        // ESTIMATE, not in datasheet
    pin_to_tip: Meters(0.0125),        // scaled, rounded tip radius
    pin_head_radius: Meters(0.008),    // scaled
    pin_head_height: Meters(0.0038),   // scaled
    pin_overhang: Meters(0.007),       // scaled, far side, cross-hole for the split ring
    split_ring_radius: Meters(0.011),  // scaled
    split_ring_wire: Meters(0.001),    // ESTIMATE
};

impl ClevisDimensions {
    /// Half the jaw's width across the pin axis, outer face to outer face.
    pub fn jaw_half_thickness(&self) -> f32 {
        self.jaw_gap.f32() / 2.0 + self.plate_thickness.f32()
    }

    /// Where the solid jaw block ends and the neck down to the barrel begins.
    pub fn jaw_block_end(&self) -> f32 {
        self.overall_length.f32() - self.barrel_length.f32() - self.neck_length.f32()
    }
}

/// Only end pairs whose pivots are this close can possibly collide: nothing
/// of a terminal reaches further from its pin than the barrel end.
pub const OVERLAP_PREFILTER: f32 = 2.0 * CLEVIS.overall_length.0;

/// From a strut's connector pivot to the centre of a bridle clamp: the
/// shackle on the cross-tube plus the clamp's eye. ESTIMATE.
pub const BRIDLE_LINK: Meters = Meters(0.090);
/// Bridle clamp, a cheek block round a fixed grooved saddle. ESTIMATES
/// until it is drawn. The cable's centreline wraps the saddle at this
/// radius: about eight wire diameters, gentle enough for 6 mm wire.
pub const SADDLE_RADIUS: Meters = Meters(0.025);
/// The 6 mm wire's radius.
pub const WIRE_RADIUS: Meters = Meters(0.003);
/// Cheek plates: round the saddle, laser cut from stainless plate.
pub const CHEEK_RADIUS: Meters = Meters(0.034);
pub const CHEEK_THICKNESS: Meters = Meters(0.006);
/// Between the cheeks: the wire and its clearance.
pub const CHEEK_GAP: Meters = Meters(0.008);
/// From the saddle's centre to the shackle pin through the cheeks' eye,
/// clear of the wire on the saddle.
pub const EYE_OFFSET: Meters = Meters(0.044);
/// Metal round the eye.
pub const EYE_RADIUS: Meters = Meters(0.012);
/// The sharpest bend the clamp's saddle lets the cable take. ESTIMATE.
pub const MAX_SADDLE_BEND: Degrees = Degrees(110.0);

/// Half the width of a bridle link's shackle, whose ears straddle the
/// connector's cross-tube. ESTIMATE.
pub const SHACKLE_RADIUS: Meters = Meters(0.012);

/// Half the width of a ring's boss, the arm out to the cross-tube.
pub const BOSS_ARM_RADIUS: Meters = Meters(0.006);

/// `(segment start, segment end, radius)`.
pub type Capsule = (Vec3, Vec3, f32);

/// One assembled cable end for collision testing: its body (fork, shackle
/// or clamp) and, where it hangs on a ring, that ring's boss arm.
#[derive(Clone, Copy, Debug)]
pub struct Assembly {
    pub body: [Capsule; 3],
    pub arm: Option<Capsule>,
}

impl Assembly {
    /// Add the boss arm from the ring's rim, `ring_radius` out from its
    /// centre, to the pivot.
    pub fn with_arm(self, ring_center: Vec3, pivot: Vec3, ring_radius: f32) -> Self {
        let radial = (pivot - ring_center).normalize();
        let rim = ring_center + radial * ring_radius;
        Assembly {
            arm: Some(shrunk(rim, pivot, BOSS_ARM_RADIUS.f32())),
            ..self
        }
    }
}

/// A bridle link for collision testing: its shackle from the connector
/// pivot to the clamp.
pub fn shackle_capsules(pivot: Vec3, clamp: Vec3) -> Assembly {
    Assembly {
        body: [(pivot, clamp, SHACKLE_RADIUS.f32()); 3],
        arm: None,
    }
}

/// A bridle clamp's cheek block for collision testing: the round cheeks
/// across and along the link direction, the latter reaching to the eye.
pub fn clamp_capsules(saddle: Vec3, toward_link: Vec3, normal: Vec3) -> Assembly {
    let across = toward_link.cross(normal);
    let thick = CHEEK_GAP.f32() / 2.0 + CHEEK_THICKNESS.f32();
    let reach = CHEEK_RADIUS.f32() - thick;
    Assembly {
        body: [
            (saddle - across * reach, saddle + across * reach, thick),
            (saddle - toward_link * reach, saddle + toward_link * EYE_OFFSET.f32(), thick),
            (saddle + toward_link * EYE_OFFSET.f32(), saddle + toward_link * EYE_OFFSET.f32(), EYE_RADIUS.f32()),
        ],
        arm: None,
    }
}

/// Shortest side of a cable bent through a bridle clamp, from the far
/// pivot to where it leaves the saddle: a terminal and half the free wire
/// a cable needs.
pub fn min_clamp_side() -> f32 {
    CLEVIS.overall_length.f32() + MIN_FREE_WIRE.f32() / 2.0
}

/// Bare wire a cable needs between its two barrels to be made at all.
/// ESTIMATE — confirm the rigger's minimum for swaging 6 mm wire.
pub const MIN_FREE_WIRE: Meters = Meters(0.050);

/// Shortest pivot-to-pivot span buildable as a cable: a terminal at each
/// end plus the minimum free wire. Anything shorter needs a solid link.
pub fn min_cable_length() -> f32 {
    2.0 * CLEVIS.overall_length.f32() + MIN_FREE_WIRE.f32()
}

/// Where the rendered cable should terminate: buried inside the barrel, so
/// the cable visibly ends at its terminal rather than running on into the
/// connector's cross-tube.
pub fn cable_termination(pivot: Vec3, pull_other_end: Vec3) -> Vec3 {
    let dir = (pull_other_end - pivot).normalize();
    pivot + dir * (CLEVIS.overall_length.f32() - CLEVIS.barrel_length.f32() / 2.0)
}

/// Capsule approximation of one assembled connector end for collision
/// testing: the jaw from tip to neck, the pin from split-ring end to head,
/// and the barrel; add its ring's boss arm with `Assembly::with_arm`.
///
/// A capsule's end caps extend a full radius beyond its segment, so each
/// segment below is the part's real extent shrunk by the radius at both
/// ends — otherwise every assembly carries phantom metal past its faces
/// and neighbours flag as culprits without touching.
pub fn capsules(pivot: Vec3, tangent: Vec3, cable_dir: Vec3) -> Assembly {
    let c = &CLEVIS;
    let jaw_half = c.jaw_half_thickness();
    let body = [
        shrunk(
            pivot - cable_dir * c.pin_to_tip.f32(),
            pivot + cable_dir * c.jaw_block_end(),
            jaw_half.max(c.plate_width.f32() / 2.0),
        ),
        shrunk(
            pivot - tangent * (jaw_half + c.pin_overhang.f32()),
            pivot + tangent * (jaw_half + c.pin_head_height.f32()),
            c.pin_radius.f32(),
        ),
        shrunk(
            pivot + cable_dir * c.jaw_block_end(),
            pivot + cable_dir * c.overall_length.f32(),
            c.barrel_radius.f32(),
        ),
    ];
    Assembly { body, arm: None }
}

/// Pull both segment ends inward by the radius, so the capsule's overall
/// reach equals the part's real extent. Degenerates to a sphere at the
/// midpoint when the part is shorter than two radii.
fn shrunk(a: Vec3, b: Vec3, radius: f32) -> (Vec3, Vec3, f32) {
    let d = b - a;
    let len = d.length();
    if len <= 2.0 * radius {
        let mid = (a + b) * 0.5;
        (mid, mid, radius)
    } else {
        let unit = d / len;
        (a + unit * radius, b - unit * radius, radius)
    }
}

/// Whether two assemblies touch: body against body, and each one's boss
/// arm against the other's body. Two arms never meet — rings on one stack
/// lie in parallel planes, a washer apart.
pub fn assemblies_collide(a: &Assembly, b: &Assembly) -> bool {
    let touch = |(a0, a1, ra): &Capsule, (b0, b1, rb): &Capsule| {
        segment_segment_distance(*a0, *a1, *b0, *b1) < ra + rb
    };
    let body_hits = |arm: &Option<Capsule>, body: &[Capsule; 3]| {
        arm.iter().any(|arm| body.iter().any(|part| touch(arm, part)))
    };
    a.body.iter().any(|pa| b.body.iter().any(|pb| touch(pa, pb)))
        || body_hits(&a.arm, &b.body)
        || body_hits(&b.arm, &a.body)
}
