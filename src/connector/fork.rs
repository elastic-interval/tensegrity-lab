//! The hardware at a cable end: the jaw terminal (clevis) that pivots on
//! the connector's cross-tube pin — a bought Louis Ruys 37260006 stainless
//! jaw terminal for 6 mm wire, held by a plain headed pin with a split ring.
//!
//! Also provides the capsule model of a whole assembled connector end
//! (jaw, pin, barrel), used to mark pairs that would physically collide —
//! real construction issues, surfaced once per slot-assignment rebuild and
//! highlighted by the renderer.

use crate::connector::attachment::segment_segment_distance;
use crate::units::{Meters, Unit};
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

/// A bridle's node plate, the triangular steel plate its three cables'
/// forks pin to (see `connector::node_plate`). ESTIMATES until it is drawn.
/// Every plate is equilateral, this far hole to hole.
pub const PLATE_SIDE: Meters = Meters(0.060);
/// Steel round each hole: the radius of the plate's rounded corners.
pub const PLATE_CORNER_RADIUS: Meters = Meters(0.016);
/// Thick enough to bear the pins, thin enough for the forks' 14 mm jaw gap.
pub const PLATE_THICKNESS: Meters = Meters(0.012);

/// A bridle's jaw–jaw swivel: a jaw like the jaw terminal's at each end,
/// one pinned through the strut end's cross-tube and one through its node
/// plate's hole, joined by a bearing so either turns freely about the
/// link. ESTIMATES: confirm against a catalogue swivel for 6 mm.
/// From a pin to the end of its jaw's block.
pub const SWIVEL_JAW_LENGTH: Meters = Meters(0.031);
/// The bearing between the two jaw blocks.
pub const SWIVEL_BEARING_LENGTH: Meters = Meters(0.020);
pub const SWIVEL_BEARING_RADIUS: Meters = Meters(0.009);
/// Pin to pin.
pub const SWIVEL_LENGTH: Meters = Meters(2.0 * SWIVEL_JAW_LENGTH.0 + SWIVEL_BEARING_LENGTH.0);
/// Where a bridle node whose plate hangs on a swivel starts, from its strut
/// end, before `Fabric::hang_bridles` fits its link to the swivel. Where
/// nodes start tips Headless Hug's borderline collisions either way.
pub const BRIDLE_LINK: Meters = Meters(0.100);
/// Half the width of a bridle swivel's jaws, for collision testing.
/// ESTIMATE.
pub const SWIVEL_RADIUS: Meters = Meters(0.012);
/// A swivel link, where a cable from a node plate would be too short to
/// make: a swivel stretched by a solid round bar, cut to length. ESTIMATE.
pub const LINK_ROD_RADIUS: Meters = Meters(0.005);

/// Half the width of a ring's boss, the arm out to the cross-tube.
pub const BOSS_ARM_RADIUS: Meters = Meters(0.006);

/// `(segment start, segment end, radius)`.
pub type Capsule = (Vec3, Vec3, f32);

/// One assembled cable end for collision testing: its body (fork, swivel,
/// clamp or node plate) and, where it hangs on a ring, that ring's boss arm.
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

/// A bridle swivel for collision testing: from the connector pivot to the
/// plate's hole.
pub fn swivel_capsules(pivot: Vec3, hole: Vec3) -> Assembly {
    Assembly {
        body: [(pivot, hole, SWIVEL_RADIUS.f32()); 3],
        arm: None,
    }
}

/// A swivel link for collision testing: a jaw at each pin, the bearing and
/// bar between them.
pub fn link_capsules(pivot: Vec3, far: Vec3) -> Assembly {
    let along = (far - pivot).normalize_or_zero() * SWIVEL_JAW_LENGTH.f32();
    Assembly {
        body: [
            (pivot, pivot + along, SWIVEL_RADIUS.f32()),
            (pivot + along, far - along, SWIVEL_BEARING_RADIUS.f32()),
            (far - along, far, SWIVEL_RADIUS.f32()),
        ],
        arm: None,
    }
}

/// A node plate for collision testing: its three edges, hole to hole,
/// as thick as its rounded corners.
pub fn plate_capsules(holes: [Vec3; 3]) -> Assembly {
    let corner = PLATE_CORNER_RADIUS.f32();
    Assembly {
        body: [
            (holes[0], holes[1], corner),
            (holes[1], holes[2], corner),
            (holes[2], holes[0], corner),
        ],
        arm: None,
    }
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
