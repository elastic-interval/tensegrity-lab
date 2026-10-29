//! A node plate: the triangular steel plate at a bridle node, with a hole
//! for each of the three cables meeting there. Each cable is a whole cable
//! with a jaw terminal at both ends, and the node end's terminal pins
//! through its hole; or the plate hangs on a swivel from the strut end it
//! bridles. Three forces holding a point are coplanar, so the plate lies
//! square to its cables, each hole on its cable's line.
//!
//! Every plate is the same part: an equilateral triangle `PLATE_SIDE` on a
//! side, a hole in each corner. Where the cables' lines meet depends on
//! their directions, so it is the triangle's centre only when they are 120°
//! apart.
//!
//! The simulation knows the plate only as the joint where the three cables'
//! lines meet.

use crate::connector::fork::{PLATE_SIDE, SWIVEL_LENGTH};
use crate::fabric::IntervalKey;
use crate::units::Unit;
use glam::{Mat3, Vec3};

#[derive(Clone, Debug)]
pub struct NodePlate {
    /// Where the cables' lines meet: the simulated joint.
    pub center: Vec3,
    /// Square to the plate, along its pins.
    pub normal: Vec3,
    /// Each cable with its hole, where its terminal's or swivel's pin passes.
    pub holes: [(IntervalKey, Vec3); 3],
    /// How far the plate is moved to hang its swivel's hole a swivel's
    /// length from the pivot, which a link of the right length makes nil.
    pub slide: Vec3,
    /// No equilateral plate has a corner on each cable's line.
    pub jammed: bool,
}

impl NodePlate {
    /// The plate for three cables meeting at `center`, each aimed at its far
    /// end; one may hang the plate on a swivel from the strut end's pivot
    /// it aims at.
    pub fn new(center: Vec3, cables: [(IntervalKey, Vec3, bool); 3]) -> NodePlate {
        let [a, b, c] = cables.map(|(_, aim, _)| (aim - center).normalize());
        let normal = (a.cross(b) + b.cross(c) + c.cross(a)).normalize_or(a.any_orthonormal_vector());
        let toward = cables.map(|(_, aim, _)| (aim - center).reject_from(normal).normalize_or(a));
        let side = PLATE_SIDE.f32();
        let solved = equilateral_radii(toward, side);
        let radii = solved.unwrap_or([side / 3f32.sqrt(); 3]);
        let mut holes = [0, 1, 2].map(|i| center + toward[i] * radii[i]);
        let mut slide = Vec3::ZERO;
        if let Some(i) = cables.iter().position(|&(_, _, swivel)| swivel) {
            let gap = holes[i] - cables[i].1;
            slide = gap.normalize() * SWIVEL_LENGTH.f32() - gap;
            holes = holes.map(|hole| hole + slide);
        }
        let holes = [0, 1, 2].map(|i| (cables[i].0, holes[i]));
        NodePlate { center, normal, holes, slide, jammed: solved.is_none() }
    }

    /// The hole `cable`'s terminal or swivel pins through, if it is one of
    /// this plate's.
    pub fn hole(&self, cable: IntervalKey) -> Option<Vec3> {
        self.holes.iter().find(|(key, _)| *key == cable).map(|(_, hole)| *hole)
    }
}

/// How far out along each of three directions in a plane the corners of an
/// equilateral triangle with this `side` lie, by Newton's method from the
/// triangle centred on their meeting point. `None` if there is no such
/// triangle with every corner ahead.
fn equilateral_radii(toward: [Vec3; 3], side: f32) -> Option<[f32; 3]> {
    let pairs = [(0, 1), (1, 2), (2, 0)];
    let cos = pairs.map(|(i, j)| toward[i].dot(toward[j]));
    let mut r = Vec3::splat(side / 3f32.sqrt());
    for _ in 0..50 {
        let mut residual = Vec3::ZERO;
        let mut rows = [Vec3::ZERO; 3];
        for (k, &(i, j)) in pairs.iter().enumerate() {
            residual[k] = r[i] * r[i] + r[j] * r[j] - 2.0 * r[i] * r[j] * cos[k] - side * side;
            rows[k][i] = 2.0 * (r[i] - r[j] * cos[k]);
            rows[k][j] = 2.0 * (r[j] - r[i] * cos[k]);
        }
        let jacobian = Mat3::from_cols(rows[0], rows[1], rows[2]).transpose();
        if jacobian.determinant().abs() < 1e-12 {
            return None;
        }
        r -= jacobian.inverse() * residual;
        if residual.abs().max_element() < 1e-10 {
            break;
        }
    }
    let converged = pairs.iter().all(|&(i, j)| {
        ((toward[i] * r[i]).distance(toward[j] * r[j]) - side).abs() < 1e-5
    });
    (converged && r.min_element() > 0.0).then(|| r.to_array())
}
