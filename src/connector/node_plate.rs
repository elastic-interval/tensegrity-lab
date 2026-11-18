//! A node plate: the steel plate at a bridle node, with a hole for each of
//! the cables meeting there, usually three. Each cable is a whole cable
//! with a jaw terminal at both ends, and the node end's terminal pins
//! through its hole; or the plate hangs on a fork rod from the strut end it
//! bridles. Three forces holding a point are coplanar, so the plate lies
//! square to its cables, each hole on its cable's line.
//!
//! Every three-cable plate is the same part: an equilateral triangle
//! `PLATE_SIDE` on a side, a hole in each corner. Where the cables' lines
//! meet depends on their directions, so it is the triangle's centre only
//! when they are 120° apart.
//!
//! A plate taking more (`Bridle::bridle_across`) is a tube plate: its hanger
//! pins through a hole as on any plate, and each cable's fork pins through a
//! cross-tube welded on a boss, as on a ring. The plate lies in a plane
//! holding the hanger's line, the one its cables come closest to; each boss
//! points along its cable's line in that plane and its tube lies across it,
//! so the fork turns on its pin to follow the cable out of the plane. The
//! pivots are all at one distance from the centre, as close as
//! `PLATE_HOLE_SPACING` allows. How far the hanger leaves the plane is its
//! `tilt`, nil by construction.
//!
//! The simulation knows the plate only as the joint where the three cables'
//! lines meet.

use crate::connector::fork::{HANGER_LENGTH, MAX_PLATE_TILT, PLATE_HOLE_SPACING, PLATE_SIDE};
use crate::fabric::IntervalKey;
use crate::units::Unit;
use glam::{Mat3, Vec3};

#[derive(Clone, Debug)]
pub struct NodePlate {
    /// Where the cables' lines meet: the simulated joint, except on a hinge
    /// cable's plate (`ConnectorSystem::split_plate_center`).
    pub center: Vec3,
    /// Square to the plate, along its pins.
    pub normal: Vec3,
    /// Each cable with its hole, where its terminal's or fork's pin passes:
    /// on a tube plate, the cross-tube's centre for each cable but the
    /// hanger.
    pub holes: Vec<(IntervalKey, Vec3)>,
    /// Each hole's pin axis, in the order of `holes`: square to the plate
    /// through a hole, along the tube through a cross-tube.
    pub pins: Vec<Vec3>,
    /// The cables but the hanger pin through cross-tubes.
    pub tubed: bool,
    /// How far the plate is moved to hang its hanger's hole a hanger's
    /// length from the pivot, which a link of the right length makes nil.
    pub slide: Vec3,
    /// No equilateral plate has a corner on each cable's line, or a plate
    /// with more holes leaves a cable more than `MAX_PLATE_TILT` out of it.
    pub jammed: bool,
    /// The most any cable through a hole leaves the plate's plane, in
    /// degrees: nil with three cables, and for a tube plate's hanger.
    pub tilt: f32,
}

impl NodePlate {
    /// The plate for the cables meeting at `center`, each aimed at its far
    /// end; one may hang the plate on a fork rod from the strut end's pivot
    /// it aims at.
    pub fn new(center: Vec3, cables: &[(IntervalKey, Vec3, bool)]) -> NodePlate {
        match cables {
            &[a, b, c] => Self::triangle(center, [a, b, c]),
            _ => Self::polygon(center, cables),
        }
    }

    fn triangle(center: Vec3, cables: [(IntervalKey, Vec3, bool); 3]) -> NodePlate {
        let [a, b, c] = cables.map(|(_, aim, _)| (aim - center).normalize());
        let normal = (a.cross(b) + b.cross(c) + c.cross(a)).normalize_or(a.any_orthonormal_vector());
        let toward = cables.map(|(_, aim, _)| (aim - center).reject_from(normal).normalize_or(a));
        let side = PLATE_SIDE.f32();
        let solved = equilateral_radii(toward, side);
        let radii = solved.unwrap_or([side / 3f32.sqrt(); 3]);
        let mut holes = [0, 1, 2].map(|i| center + toward[i] * radii[i]);
        let mut slide = Vec3::ZERO;
        if let Some(i) = cables.iter().position(|&(_, _, hanger)| hanger) {
            let gap = holes[i] - cables[i].1;
            slide = gap.normalize() * HANGER_LENGTH.f32() - gap;
            holes = holes.map(|hole| hole + slide);
        }
        let holes = [0, 1, 2].map(|i| (cables[i].0, holes[i])).to_vec();
        NodePlate { center, normal, holes, pins: vec![normal; 3], tubed: false, slide, jammed: solved.is_none(), tilt: 0.0 }
    }

    /// A tube plate laid out about where it hangs: the hanger slides it, so
    /// it is laid out again about its new place until the slide is spent,
    /// each boss then pointing along its cable.
    fn polygon(center: Vec3, cables: &[(IntervalKey, Vec3, bool)]) -> NodePlate {
        let mut at = center;
        let mut plate = Self::laid_out(at, cables);
        for _ in 0..8 {
            if plate.slide.length() < 1e-5 {
                break;
            }
            at += plate.slide;
            plate = Self::laid_out(at, cables);
        }
        NodePlate { slide: at - center + plate.slide, ..plate }
    }

    fn laid_out(center: Vec3, cables: &[(IntervalKey, Vec3, bool)]) -> NodePlate {
        let directions: Vec<Vec3> = cables.iter().map(|&(_, aim, _)| (aim - center).normalize()).collect();
        let hanger = cables.iter().position(|&(_, _, hanger)| hanger);
        let normal = match hanger {
            Some(h) => plane_holding(directions[h], &directions),
            None => least_plane_normal(&directions),
        };
        let tilt = hanger.map_or(0.0, |h| directions[h].dot(normal).abs().asin().to_degrees());
        let toward: Vec<Vec3> = directions.iter().map(|d| d.reject_from(normal).normalize_or(*d)).collect();
        let u = toward[0];
        let w = normal.cross(u);
        let mut angles: Vec<f32> = toward.iter().map(|t| t.dot(w).atan2(t.dot(u))).collect();
        angles.sort_by(f32::total_cmp);
        let gap = (0..angles.len())
            .map(|i| {
                let next = if i + 1 < angles.len() { angles[i + 1] } else { angles[0] + std::f32::consts::TAU };
                next - angles[i]
            })
            .fold(f32::MAX, f32::min);
        let triangle = PLATE_SIDE.f32() / 3f32.sqrt();
        let radius = (PLATE_HOLE_SPACING.f32() / (2.0 * (gap / 2.0).sin())).max(triangle);
        let mut holes: Vec<Vec3> = toward.iter().map(|t| center + *t * radius).collect();
        let mut slide = Vec3::ZERO;
        if let Some(i) = cables.iter().position(|&(_, _, hanger)| hanger) {
            let hole_gap = holes[i] - cables[i].1;
            slide = hole_gap.normalize() * HANGER_LENGTH.f32() - hole_gap;
            holes = holes.iter().map(|hole| *hole + slide).collect();
        }
        let jammed = tilt > MAX_PLATE_TILT || radius > 3.0 * triangle;
        // Each tube square to its cable as it leaves the tube, where the
        // plate hangs, not as it would leave the centre.
        let pins = (0..cables.len())
            .map(|i| {
                if Some(i) == hanger {
                    return normal;
                }
                let leaving = (cables[i].1 - holes[i]).reject_from(normal).normalize_or(toward[i]);
                normal.cross(leaving).normalize()
            })
            .collect();
        let holes = cables.iter().zip(holes).map(|(&(key, _, _), hole)| (key, hole)).collect();
        NodePlate { center, normal, holes, pins, tubed: true, slide, jammed, tilt }
    }

    /// The pin axis `cable`'s terminal or fork turns on, if it is one of
    /// this plate's.
    pub fn pin(&self, cable: IntervalKey) -> Option<Vec3> {
        self.holes.iter().position(|(key, _)| *key == cable).map(|i| self.pins[i])
    }

    /// The holes in order round the plate, for its outline.
    pub fn outline(&self) -> Vec<Vec3> {
        let middle = self.holes.iter().map(|(_, hole)| *hole).sum::<Vec3>() / self.holes.len() as f32;
        let u = (self.holes[0].1 - middle).normalize();
        let w = self.normal.cross(u);
        let mut around: Vec<(f32, Vec3)> = self
            .holes
            .iter()
            .map(|&(_, hole)| ((hole - middle).dot(w).atan2((hole - middle).dot(u)), hole))
            .collect();
        around.sort_by(|a, b| a.0.total_cmp(&b.0));
        around.into_iter().map(|(_, hole)| hole).collect()
    }

    /// The hole `cable`'s terminal or fork pins through, if it is one of
    /// this plate's.
    pub fn hole(&self, cable: IntervalKey) -> Option<Vec3> {
        self.holes.iter().find(|(key, _)| *key == cable).map(|(_, hole)| *hole)
    }
}

/// The normal of the plane holding `line` that the `directions` come
/// closest to: of the planes turning about the line, the one with least
/// spread out of it.
fn plane_holding(line: Vec3, directions: &[Vec3]) -> Vec3 {
    let e1 = line.any_orthonormal_vector();
    let e2 = line.cross(e1).normalize();
    let (mut a, mut b, mut c) = (0.0, 0.0, 0.0);
    for d in directions {
        let (p, q) = (d.dot(e1), d.dot(e2));
        a += p * p;
        b += p * q;
        c += q * q;
    }
    let theta = 0.5 * (2.0 * b).atan2(a - c) + std::f32::consts::FRAC_PI_2;
    (e1 * theta.cos() + e2 * theta.sin()).normalize()
}

/// The normal of the plane the `directions` come closest to: the axis of
/// least spread, by power iteration on its complement.
fn least_plane_normal(directions: &[Vec3]) -> Vec3 {
    let spread = directions.iter().fold(Mat3::ZERO, |m, d| {
        m + Mat3::from_cols(*d * d.x, *d * d.y, *d * d.z)
    });
    let complement = Mat3::IDENTITY * (spread.x_axis.x + spread.y_axis.y + spread.z_axis.z) - spread;
    let start = directions.windows(2).map(|pair| pair[0].cross(pair[1])).sum::<Vec3>();
    let mut normal = start.normalize_or(Vec3::Y);
    for _ in 0..100 {
        normal = (complement * normal).normalize_or(normal);
    }
    normal
}

/// How far along each of three lines through a point in a plane the corners
/// of an equilateral triangle with this `side` lie, each line's direction
/// being its cable's, by Newton's method from a range of starts. Corners lie
/// ahead of the point if they can. Otherwise the corner of the cable pulling
/// most against the other two may lie behind it: two cables spreading to far
/// struts meet beyond the plate, on the third cable's side. The other two
/// never do, or they would cross. `None` if no such triangle is found.
fn equilateral_radii(toward: [Vec3; 3], side: f32) -> Option<[f32; 3]> {
    const SIZES: [f32; 5] = [0.3, 0.6, 1.5, 4.0, 10.0];
    let opposing = (0..3)
        .min_by(|&i, &j| {
            let against = |k: usize| toward[k].dot(toward[(k + 1) % 3] + toward[(k + 2) % 3]);
            against(i).total_cmp(&against(j))
        })
        .unwrap_or(0);
    let behind = {
        let mut signs = Vec3::ONE;
        signs[opposing] = -1.0;
        signs
    };
    let starts = SIZES.iter().flat_map(|size| SIZES.iter().map(move |opposite| (*size, *opposite)));
    [Vec3::ONE, behind].iter().find_map(|signs| {
        starts.clone().find_map(|(size, opposite)| {
            let mut start = *signs * side * size;
            start[opposing] = signs[opposing] * side * opposite;
            let r = newton_radii(toward, side, start)?;
            let right_way = (0..3).all(|i| r[i] * signs[i] > 0.0);
            let corners = [0, 1, 2].map(|i| toward[i] * r[i]);
            let middle = (corners[0] + corners[1] + corners[2]) / 3.0;
            let outward = (0..3).all(|i| toward[i].dot(corners[i] - middle) > 0.0);
            (right_way && outward).then_some(r)
        })
    })
}

fn newton_radii(toward: [Vec3; 3], side: f32, start: Vec3) -> Option<[f32; 3]> {
    let pairs = [(0, 1), (1, 2), (2, 0)];
    let cos = pairs.map(|(i, j)| toward[i].dot(toward[j]));
    let mut r = start;
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
    converged.then(|| r.to_array())
}
