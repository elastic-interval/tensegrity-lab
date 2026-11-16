//! What a settled brick is judged by: the rules it must pass to be a brick
//! at all, and the candidate fitness measures (docs/brick-evolution.md).

use crate::build::dsl::brick::BakedBrick;
use crate::build::dsl::brick_library::equilibrium::Bake;
use crate::connector::attachment::segment_segment_distance;
use glam::Vec3;

/// How far the face radials' mean strain may miss the baker's target:
/// further off, the bisection found no scale that anchors the brick.
const FACE_STRAIN_TOLERANCE: f32 = 0.005;

/// No two struts closer than this, in face radial lengths.
const CLEARANCE: f32 = 0.1;

/// A member within this strain of zero is slack.
pub const SLACK_STRAIN: f32 = 0.005;

/// A non-rigid stiffness mode below this fraction of the stiffest is a mechanism.
pub(crate) const MECHANISM_FRACTION: f64 = 1.0e-6;

const RIGID_MODES: usize = 6;

#[derive(Clone, Debug)]
pub struct Measures {
    /// Mean strain of the face radials.
    pub face_strain: f32,
    pub slack: usize,
    pub mechanisms: usize,
    /// Lowest non-rigid stiffness mode per member.
    pub stiffness: f32,
    /// Closest approach of two struts, in face radial lengths.
    pub clearance: f32,
    /// Largest spread (deviation / |mean|) of strain within one role.
    pub strain_spread: f32,
    /// Largest spread (max − min) / mean of one face's three radial lengths.
    pub face_irregularity: f32,
    /// Spread (deviation / mean) of the joints' distance from their centre.
    pub sphere_spread: f32,
}

impl Measures {
    pub fn viable(&self) -> bool {
        (self.face_strain - BakedBrick::TARGET_FACE_STRAIN).abs() < FACE_STRAIN_TOLERANCE
            && self.slack == 0
            && self.mechanisms == 0
            && self.clearance > CLEARANCE
    }
}

pub(crate) fn measure(bake: &Bake) -> Measures {
    let structural = bake.structural;
    let is_radial = |alpha: usize, omega: usize| alpha >= structural || omega >= structural;
    let length = |alpha: usize, omega: usize| (bake.positions[omega] - bake.positions[alpha]).length();

    let mut slack = 0;
    let (mut push, mut pull, mut radial) = (vec![], vec![], vec![]);
    for s in &bake.springs {
        let strain = (length(s.alpha, s.omega) - s.rest_length) / s.rest_length;
        let engaged = if s.is_push { strain < -SLACK_STRAIN } else { strain > SLACK_STRAIN };
        if !engaged {
            slack += 1;
        }
        match (s.is_push, is_radial(s.alpha, s.omega)) {
            (true, _) => push.push(strain),
            (false, true) => radial.push(strain),
            (false, false) => pull.push(strain),
        }
    }
    let face_strain = mean(&radial);
    let strain_spread = [push, pull, radial]
        .iter()
        .filter(|v| !v.is_empty())
        .map(|v| spread(v))
        .fold(0.0, f32::max);

    let radial_length = mean(
        &bake
            .faces
            .iter()
            .flat_map(|f| f.vertices.map(|v| length(f.midpoint, v)))
            .collect::<Vec<_>>(),
    );
    let face_irregularity = bake
        .faces
        .iter()
        .map(|f| {
            let r = f.vertices.map(|v| length(f.midpoint, v));
            let (lo, hi) = (r.iter().cloned().fold(f32::MAX, f32::min), r.iter().cloned().fold(0.0, f32::max));
            (hi - lo) / mean(&r)
        })
        .fold(0.0, f32::max);

    let struts: Vec<(Vec3, Vec3)> = bake
        .springs
        .iter()
        .filter(|s| s.is_push)
        .map(|s| (bake.positions[s.alpha], bake.positions[s.omega]))
        .collect();
    let mut closest = f32::MAX;
    for i in 0..struts.len() {
        for j in (i + 1)..struts.len() {
            closest = closest.min(segment_segment_distance(struts[i].0, struts[i].1, struts[j].0, struts[j].1));
        }
    }

    let joints = &bake.positions[..structural];
    let centre = joints.iter().copied().sum::<Vec3>() / joints.len() as f32;
    let sphere_spread = spread(&joints.iter().map(|p| (*p - centre).length()).collect::<Vec<_>>());

    let mut modes = stiffness_modes(bake);
    modes.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let stiffest = modes.last().copied().unwrap_or(0.0);
    let soft = modes.iter().filter(|&&m| m < stiffest * MECHANISM_FRACTION).count();
    let lowest_non_rigid = modes.get(RIGID_MODES).copied().unwrap_or(0.0);

    Measures {
        face_strain,
        slack,
        mechanisms: soft.saturating_sub(RIGID_MODES),
        stiffness: (lowest_non_rigid / bake.springs.len() as f64) as f32,
        clearance: closest / radial_length,
        strain_spread,
        face_irregularity,
        sphere_spread,
    }
}

fn mean(v: &[f32]) -> f32 {
    v.iter().sum::<f32>() / v.len() as f32
}

fn spread(v: &[f32]) -> f32 {
    let m = mean(v);
    let variance = v.iter().map(|x| (x - m) * (x - m)).sum::<f32>() / v.len() as f32;
    variance.sqrt() / m.abs()
}

/// Eigenvalues of the tangent stiffness matrix at the bake's positions:
/// material stiffness along each engaged member plus the geometric
/// stiffness of its force across it (positive in tension, negative in
/// compression). Slack members contribute nothing, as in the baker.
pub(crate) fn stiffness_modes(bake: &Bake) -> Vec<f64> {
    let n = 3 * bake.positions.len();
    let mut k = vec![0.0f64; n * n];
    for s in &bake.springs {
        let d = bake.positions[s.omega] - bake.positions[s.alpha];
        let l = d.length();
        let stretch = l - s.rest_length;
        if l < 1.0e-9 || (s.is_push && stretch > 0.0) || (!s.is_push && stretch < 0.0) {
            continue;
        }
        let u = (d / l).as_dvec3().to_array();
        let force = (s.k * stretch) as f64;
        let (axial, lateral) = (s.k as f64, force / l as f64);
        for i in 0..3 {
            for j in 0..3 {
                let uu = u[i] * u[j];
                let identity = if i == j { 1.0 } else { 0.0 };
                let block = axial * uu + lateral * (identity - uu);
                for (a, b, sign) in [(s.alpha, s.alpha, 1.0), (s.omega, s.omega, 1.0), (s.alpha, s.omega, -1.0), (s.omega, s.alpha, -1.0)] {
                    k[(3 * a + i) * n + 3 * b + j] += sign * block;
                }
            }
        }
    }
    let rigid = rigid_motions(&bake.positions);
    jacobi_eigenvalues(without(k, &rigid, n), n)
}

/// The six rigid motions of the positions (three slides, three turns about
/// their centre), orthonormal, as flat vectors of length 3 × positions.
fn rigid_motions(positions: &[Vec3]) -> Vec<Vec<f64>> {
    let centre = positions.iter().copied().sum::<Vec3>() / positions.len() as f32;
    let mut motions: Vec<Vec<f64>> = vec![];
    for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
        motions.push(positions.iter().flat_map(|_| axis.as_dvec3().to_array()).collect());
        motions.push(positions.iter().flat_map(|p| axis.cross(*p - centre).as_dvec3().to_array()).collect());
    }
    let mut basis: Vec<Vec<f64>> = vec![];
    for mut v in motions {
        for b in &basis {
            let dot: f64 = v.iter().zip(b).map(|(x, y)| x * y).sum();
            v.iter_mut().zip(b).for_each(|(x, y)| *x -= dot * y);
        }
        let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        v.iter_mut().for_each(|x| *x /= norm);
        basis.push(v);
    }
    basis
}

/// P K P with P projecting out the rigid motions, so they become exactly
/// soft and every other mode is what the members resist.
fn without(k: Vec<f64>, rigid: &[Vec<f64>], n: usize) -> Vec<f64> {
    let mut p = vec![0.0f64; n * n];
    for i in 0..n {
        p[i * n + i] = 1.0;
    }
    for r in rigid {
        for i in 0..n {
            for j in 0..n {
                p[i * n + j] -= r[i] * r[j];
            }
        }
    }
    let multiply = |a: &[f64], b: &[f64]| -> Vec<f64> {
        let mut c = vec![0.0f64; n * n];
        for i in 0..n {
            for l in 0..n {
                let ail = a[i * n + l];
                if ail != 0.0 {
                    for j in 0..n {
                        c[i * n + j] += ail * b[l * n + j];
                    }
                }
            }
        }
        c
    };
    let pk = multiply(&p, &k);
    let pkp = multiply(&pk, &p);
    // Symmetrize away rounding so Jacobi sees an exactly symmetric matrix.
    let mut sym = pkp.clone();
    for i in 0..n {
        for j in 0..n {
            sym[i * n + j] = 0.5 * (pkp[i * n + j] + pkp[j * n + i]);
        }
    }
    sym
}

/// Cyclic Jacobi rotations on a symmetric matrix, row-major.
fn jacobi_eigenvalues(mut a: Vec<f64>, n: usize) -> Vec<f64> {
    for _ in 0..100 {
        let off: f64 = (0..n).flat_map(|p| (0..n).filter(move |&q| q != p).map(move |q| (p, q))).map(|(p, q)| a[p * n + q] * a[p * n + q]).sum();
        let diagonal: f64 = (0..n).map(|p| a[p * n + p] * a[p * n + p]).sum();
        if off <= 1.0e-24 * diagonal {
            break;
        }
        for p in 0..n {
            for q in (p + 1)..n {
                let apq = a[p * n + q];
                if apq.abs() < 1.0e-300 {
                    continue;
                }
                let theta = (a[q * n + q] - a[p * n + p]) / (2.0 * apq);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let t = if theta == 0.0 { 1.0 } else { t };
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                for r in 0..n {
                    let (arp, arq) = (a[r * n + p], a[r * n + q]);
                    a[r * n + p] = c * arp - s * arq;
                    a[r * n + q] = s * arp + c * arq;
                }
                for r in 0..n {
                    let (apr, aqr) = (a[p * n + r], a[q * n + r]);
                    a[p * n + r] = c * apr - s * aqr;
                    a[q * n + r] = s * apr + c * aqr;
                }
            }
        }
    }
    (0..n).map(|p| a[p * n + p]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jacobi_finds_known_eigenvalues() {
        // Eigenvalues 1, 2 and 4, rotated off the diagonal.
        let a = vec![2.0, 1.0, 0.0, 1.0, 2.0, 0.0, 0.0, 0.0, 4.0];
        let mut values = jacobi_eigenvalues(a, 3);
        values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        for (found, expected) in values.iter().zip([1.0, 3.0, 4.0]) {
            assert!((found - expected).abs() < 1.0e-9, "{values:?}");
        }
    }
}
