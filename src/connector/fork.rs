//! The hardware at a cable end: the fork terminal (clevis) that pivots on
//! the connector's cross-tube pin. Dimensions are estimates from the site
//! photo and catalog swage forks for 6 mm wire at the 8–9 kN load class;
//! exact values await Peter's terminal drawings.
//!
//! Also provides the capsule model of a whole assembled connector end
//! (tube + jaws + body + shank), used to mark pairs that would physically
//! collide — real construction issues, surfaced once per slot-assignment
//! rebuild and highlighted by the renderer.

use crate::connector::attachment::segment_segment_distance;
use glam::Vec3;

// Cross-tube of the connector part (see docs/connectors.md).
pub const TUBE_RADIUS: f32 = 0.010; // D_tube / 2, metres
pub const TUBE_LENGTH: f32 = 0.010; // L_tube, metres — wider than the 6 mm cables from every angle

// Fork terminal.
pub const JAW_THICKNESS: f32 = 0.0045;
pub const JAW_CLEARANCE: f32 = 0.001; // air between each jaw and the tube end — the pivot's slack
pub const JAW_NOSE_RADIUS: f32 = 0.0095; // jaw outline radius around the pin
pub const JAW_REACH_UNITS: f32 = 2.2; // jaw length behind the pin, in nose radii
pub const PIN_RADIUS: f32 = 0.005; // the 10 mm clevis pin
pub const PIN_PROTRUSION: f32 = 0.0025; // pin visible past each jaw face
pub const SHANK_RADIUS: f32 = 0.006; // swage shank crimped onto the 6 mm cable
pub const SHANK_LENGTH: f32 = 0.045;
// The shank starts embedded in the jaw plates so fork and shank read as one
// body instead of just touching.
pub const SHANK_START: f32 = JAW_NOSE_RADIUS * JAW_REACH_UNITS - 0.006;

/// Only end pairs whose pivots are this close can possibly collide.
pub const OVERLAP_PREFILTER: f32 = 0.16;

/// Where the rendered cable should terminate: buried inside the swage shank,
/// so the cable visibly ends at its terminal rather than running on into the
/// connector's cross-tube.
pub fn cable_termination(pivot: Vec3, pull_other_end: Vec3) -> Vec3 {
    let dir = (pull_other_end - pivot).normalize();
    pivot + dir * (SHANK_START + SHANK_LENGTH * 0.5)
}

/// Capsule approximation of one assembled connector end for collision
/// testing: the jaw/pin region across the tangent, the nose-to-body span
/// along the cable, and the swage shank. `(segment start, segment end,
/// radius)`.
///
/// A capsule's end caps extend a full radius beyond its segment, so each
/// segment below is the part's real extent shrunk by the radius at both
/// ends — otherwise every assembly carries phantom metal past its faces
/// and neighbours flag as culprits without touching.
pub fn capsules(pivot: Vec3, tangent: Vec3, cable_dir: Vec3) -> [(Vec3, Vec3, f32); 3] {
    let jaw_half = TUBE_LENGTH / 2.0 + JAW_CLEARANCE + JAW_THICKNESS;
    [
        shrunk(
            pivot - tangent * jaw_half,
            pivot + tangent * jaw_half,
            JAW_NOSE_RADIUS,
        ),
        shrunk(
            pivot - cable_dir * JAW_NOSE_RADIUS,
            pivot + cable_dir * (JAW_NOSE_RADIUS * JAW_REACH_UNITS),
            JAW_NOSE_RADIUS,
        ),
        shrunk(
            pivot + cable_dir * SHANK_START,
            pivot + cable_dir * (SHANK_START + SHANK_LENGTH),
            SHANK_RADIUS,
        ),
    ]
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

pub fn assemblies_collide(a: &[(Vec3, Vec3, f32); 3], b: &[(Vec3, Vec3, f32); 3]) -> bool {
    for (a0, a1, ra) in a {
        for (b0, b1, rb) in b {
            if segment_segment_distance(*a0, *a1, *b0, *b1) < ra + rb {
                return true;
            }
        }
    }
    false
}
