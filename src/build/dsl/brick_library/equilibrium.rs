//! Brick baking as a static-equilibrium minimisation.
//!
//! Minimises `E(x) = Σ ½ kᵢ (Lᵢ - L₀ᵢ)²` (slack drops to zero) via L-BFGS
//! on a `Bake` struct that holds positions and springs directly — no
//! `Fabric`, no time-stepping. Output is a `BakedBrick`.

use glam::{Mat4, Quat, Vec3};
use std::collections::HashMap;

use crate::build::dsl::brick::{
    Axis, BakedBrick, BakedInterval, BakedJoint, BrickPrototype, BrickSymmetry,
};
use crate::build::dsl::brick_dsl::{BrickName, BrickParams, BrickRole, JointName};
use crate::build::dsl::brick_library;
use crate::fabric::interval::Role;
use crate::fabric::physics::presets::BAKING;
use crate::units::Unit;

/// Role under which Omni and Single declare their 3-fold cyclic symmetry.
const THREEFOLD_ROLE: BrickRole = BrickRole::Seed(1);

/// Tolerance for `verify_symmetry`. Pure solver lands at ~1e-7 m.
const SYMMETRY_VERIFICATION_TOLERANCE: f32 = 1.0e-5;

const STRAIN_TOLERANCE: f32 = 0.001;
const GRAD_TOL: f32 = 1.0e-3;
const LBFGS_MEMORY: usize = 5;
const MAX_LBFGS_ITERS: usize = 200;
const MAX_BISECTION_ROUNDS: usize = 20;

/// Designed pretension band for the shrink-wrap pass: after form-finding,
/// every member's rest length is re-derived from its settled length so pulls
/// end this fraction stretched and pushes this fraction compressed, then the
/// brick re-settles; repeated to a fixed point. This is what makes a brick a
/// solid tensegrity regardless of what the prototype's numbers said.
const SHRINK_WRAP_PRETENSION: f32 = 0.05;
const SHRINK_WRAP_ROUNDS: usize = 8;
const SHRINK_WRAP_REST_TOL: f32 = 1.0e-4;

/// A member within this strain of zero is slack — not a tensegrity.
const SLACK_EPSILON: f32 = 0.005;

#[derive(Clone, Copy, Debug)]
struct Spring {
    alpha: usize,
    omega: usize,
    rest_length: f32,
    k: f32,
    /// Push: slack when stretched. Pull/radial: slack when compressed.
    is_push: bool,
}

#[derive(Clone, Debug)]
struct FaceInfo {
    vertices: [usize; 3],
    midpoint: usize,
    rest_radial: f32,
    downward_roles: Vec<BrickRole>,
}

struct Bake {
    positions: Vec<Vec3>,
    springs: Vec<Spring>,
    faces: Vec<FaceInfo>,
    /// First `structural` positions become BakedJoints; the rest are face midpoints.
    structural: usize,
}

impl Bake {
    /// Build the initial bake from a prototype: explicit joints first,
    /// then push endpoints at `±vector·ideal/2`, then a midpoint per face.
    /// Springs cover pushes, pulls, and face radials.
    fn build_from_prototype(
        proto: &BrickPrototype,
        scale: f32,
        face_scaling: crate::build::dsl::ScaleMode,
    ) -> (Self, Vec<JointName>) {
        let mut joint_name_to_idx: HashMap<JointName, usize> = HashMap::new();
        let mut positions: Vec<Vec3> = Vec::new();
        let mut joint_names: Vec<JointName> = Vec::new();

        for name in &proto.joints {
            joint_name_to_idx.insert(*name, positions.len());
            positions.push(Vec3::ZERO);
            joint_names.push(*name);
        }
        for push in &proto.pushes {
            let vector = axis_vec(push.axis);
            let ideal = push.ideal * scale;
            let alpha_pos = -vector * ideal / 2.0;
            let omega_pos = vector * ideal / 2.0;
            joint_name_to_idx.insert(push.alpha, positions.len());
            positions.push(alpha_pos);
            joint_names.push(push.alpha);
            joint_name_to_idx.insert(push.omega, positions.len());
            positions.push(omega_pos);
            joint_names.push(push.omega);
        }
        let structural = positions.len();

        let mut springs: Vec<Spring> = Vec::new();
        for push in &proto.pushes {
            let alpha = joint_name_to_idx[&push.alpha];
            let omega = joint_name_to_idx[&push.omega];
            let rest = push.ideal * scale;
            let k = spring_k(Role::Pushing, rest);
            springs.push(Spring { alpha, omega, rest_length: rest, k, is_push: true });
        }
        for pull in &proto.pulls {
            let alpha = joint_name_to_idx[&pull.alpha];
            let omega = joint_name_to_idx[&pull.omega];
            let rest = pull.ideal * scale;
            let role = Role::from_label(&pull.material).unwrap_or(Role::Pulling);
            let k = spring_k(role, rest);
            springs.push(Spring { alpha, omega, rest_length: rest, k, is_push: false });
        }

        let mut faces: Vec<FaceInfo> = Vec::new();
        for face_def in &proto.faces {
            let vertices = face_def.joints.map(|name| joint_name_to_idx[&name]);
            let centroid = (positions[vertices[0]]
                + positions[vertices[1]]
                + positions[vertices[2]])
                / 3.0;
            let midpoint = positions.len();
            positions.push(centroid);
            joint_names.push(JointName::AlphaX);
            let face_scale = face_def.scale_for(face_scaling);
            let k = spring_k(Role::FaceRadial, face_scale.max(0.001));
            for &v in &vertices {
                springs.push(Spring {
                    alpha: midpoint,
                    omega: v,
                    rest_length: face_scale,
                    k,
                    is_push: false,
                });
            }
            let downward_roles: Vec<BrickRole> = face_def
                .aliases
                .iter()
                .filter(|alias| {
                    matches!(
                        alias.face_name,
                        crate::build::dsl::brick_dsl::FaceName::Downwards(_)
                    )
                })
                .map(|alias| alias.brick_role)
                .collect();
            faces.push(FaceInfo {
                vertices,
                midpoint,
                rest_radial: face_scale,
                downward_roles,
            });
        }

        (
            Bake {
                positions,
                springs,
                faces,
                structural,
            },
            joint_names,
        )
    }

    /// Total elastic energy and per-joint gradient (= −force).
    fn energy_and_gradient(&self) -> (f32, Vec<Vec3>) {
        let n = self.positions.len();
        let mut e = 0.0_f32;
        let mut g = vec![Vec3::ZERO; n];
        for s in &self.springs {
            let d = self.positions[s.omega] - self.positions[s.alpha];
            let l = d.length();
            if l < 1.0e-9 {
                continue;
            }
            let dl = l - s.rest_length;
            if (s.is_push && dl > 0.0) || (!s.is_push && dl < 0.0) {
                continue;
            }
            e += 0.5 * s.k * dl * dl;
            let g_alpha = (-s.k * dl / l) * d;
            g[s.alpha] += g_alpha;
            g[s.omega] -= g_alpha;
        }
        (e, g)
    }

    /// Mean face strain — the outer bisection's target.
    fn mean_face_strain(&self) -> f32 {
        let mut total = 0.0_f32;
        let mut count = 0;
        for face in &self.faces {
            let m = self.positions[face.midpoint];
            for &v in &face.vertices {
                let l = (self.positions[v] - m).length();
                total += (l - face.rest_radial) / face.rest_radial;
                count += 1;
            }
        }
        if count == 0 { 0.0 } else { total / count as f32 }
    }

    fn apply_matrix(&mut self, m: Mat4) {
        for p in &mut self.positions {
            *p = m.transform_point3(*p);
        }
    }

    fn translate(&mut self, t: Vec3) {
        for p in &mut self.positions {
            *p += t;
        }
    }

    fn centroid(&self) -> Vec3 {
        self.positions.iter().copied().sum::<Vec3>() / self.positions.len() as f32
    }

    /// Rotation aligning the average of `role`'s downward face normals
    /// with world `-Y`. Identity if no face is marked downward under `role`.
    fn down_rotation(&self, role: BrickRole) -> Mat4 {
        let normals: Vec<Vec3> = self
            .faces
            .iter()
            .filter(|f| f.downward_roles.contains(&role))
            .map(|f| {
                let p0 = self.positions[f.vertices[0]];
                let p1 = self.positions[f.vertices[1]];
                let p2 = self.positions[f.vertices[2]];
                let n = (p1 - p0).cross(p2 - p0).normalize();
                let face_centre = (p0 + p1 + p2) / 3.0;
                if n.dot(face_centre - self.centroid()) < 0.0 { -n } else { n }
            })
            .collect();
        if normals.is_empty() {
            return Mat4::IDENTITY;
        }
        let down: Vec3 = normals.into_iter().sum::<Vec3>().normalize();
        Mat4::from_quat(Quat::from_rotation_arc(down, -Vec3::Y))
    }
}

fn axis_vec(axis: Axis) -> Vec3 {
    match axis {
        Axis::X => Vec3::X,
        Axis::Y => Vec3::Y,
        Axis::Z => Vec3::Z,
    }
}

/// Spring constant: `k_at_1m / max(L₀, 0.001)` — same formula as Fabric.
fn spring_k(role: Role, rest_length: f32) -> f32 {
    let k_at_1m = role.material().spring_constant_at_1m().f32();
    k_at_1m / rest_length.max(0.001) * BAKING.rigidity_multiplier()
}

fn dot_v(a: &[Vec3], b: &[Vec3]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x.dot(*y)).sum()
}

fn norm_v(a: &[Vec3]) -> f32 {
    a.iter().map(|v| v.length_squared()).sum::<f32>().sqrt()
}

/// L-BFGS with backtracking line search. Returns `(iterations, final ‖∇‖)`.
fn lbfgs(bake: &mut Bake) -> (usize, f32) {
    let (mut e, mut g) = bake.energy_and_gradient();
    let n = bake.positions.len();

    let mut s_hist: Vec<Vec<Vec3>> = Vec::with_capacity(LBFGS_MEMORY);
    let mut y_hist: Vec<Vec<Vec3>> = Vec::with_capacity(LBFGS_MEMORY);
    let mut rho_hist: Vec<f32> = Vec::with_capacity(LBFGS_MEMORY);

    for iter in 0..MAX_LBFGS_ITERS {
        let g_norm = norm_v(&g);
        if g_norm < GRAD_TOL {
            return (iter, g_norm);
        }

        // Two-loop recursion → search direction d = -H⁻¹·g.
        let mut q = g.clone();
        let mut alphas: Vec<f32> = Vec::with_capacity(s_hist.len());
        for i in (0..s_hist.len()).rev() {
            let alpha = rho_hist[i] * dot_v(&s_hist[i], &q);
            for k in 0..n {
                q[k] -= alpha * y_hist[i][k];
            }
            alphas.push(alpha);
        }
        alphas.reverse();
        let gamma = match (s_hist.last(), y_hist.last()) {
            (Some(s), Some(y)) => {
                let yy = dot_v(y, y);
                if yy > 0.0 { dot_v(s, y) / yy } else { 1.0 }
            }
            _ => 1.0,
        };
        let mut r: Vec<Vec3> = q.iter().map(|v| *v * gamma).collect();
        for i in 0..s_hist.len() {
            let beta = rho_hist[i] * dot_v(&y_hist[i], &r);
            for k in 0..n {
                r[k] += (alphas[i] - beta) * s_hist[i][k];
            }
        }
        let direction: Vec<Vec3> = r.iter().map(|v| -*v).collect();

        let dphi0 = dot_v(&g, &direction);
        if dphi0 >= 0.0 {
            // Not a descent direction; reset history and try again.
            s_hist.clear();
            y_hist.clear();
            rho_hist.clear();
            continue;
        }
        let mut step = 1.0_f32;
        let mut new_positions = bake.positions.clone();
        let (new_e, new_g) = loop {
            for k in 0..n {
                new_positions[k] = bake.positions[k] + step * direction[k];
            }
            let saved = std::mem::replace(&mut bake.positions, new_positions.clone());
            let (e2, g2) = bake.energy_and_gradient();
            bake.positions = saved;
            if e2 <= e + 1.0e-4 * step * dphi0 || step < 1.0e-10 {
                break (e2, g2);
            }
            step *= 0.5;
        };

        let s_k: Vec<Vec3> = (0..n)
            .map(|i| new_positions[i] - bake.positions[i])
            .collect();
        let y_k: Vec<Vec3> = (0..n).map(|i| new_g[i] - g[i]).collect();
        let sy = dot_v(&s_k, &y_k);
        if sy.abs() > 1.0e-12 {
            if s_hist.len() >= LBFGS_MEMORY {
                s_hist.remove(0);
                y_hist.remove(0);
                rho_hist.remove(0);
            }
            s_hist.push(s_k);
            y_hist.push(y_k);
            rho_hist.push(1.0 / sy);
        }
        bake.positions = new_positions;
        e = new_e;
        g = new_g;
    }
    (MAX_LBFGS_ITERS, norm_v(&g))
}

struct ScaleBisector {
    scale: f32,
    low: Option<f32>,
    high: Option<f32>,
}

impl ScaleBisector {
    fn new(initial: f32) -> Self {
        Self { scale: initial, low: None, high: None }
    }
    fn next_scale(&mut self, strain: f32) -> f32 {
        let target = BakedBrick::TARGET_FACE_STRAIN;
        if strain < target { self.low = Some(self.scale); }
        else { self.high = Some(self.scale); }
        if let (Some(lo), Some(hi)) = (self.low, self.high) {
            return (lo + hi) / 2.0;
        }
        let ratio = (target / strain.max(0.001)).clamp(0.5, 2.0);
        let damped = 1.0 + 0.5 * (ratio - 1.0);
        (self.scale * damped).clamp(0.1, 10.0)
    }
}

/// Bake a brick by direct equilibrium minimisation. Output is a fully-
/// formed `BakedBrick` ready for runtime use.
pub fn bake_brick_pure(
    brick_name: BrickName,
    initial_scale: f32,
    params: BrickParams,
) -> BakedBrick {
    let proto = brick_library::get_prototype(brick_name);
    let mut bisector = ScaleBisector::new(initial_scale);

    let final_bake = loop {
        let (mut bake, names) =
            Bake::build_from_prototype(&proto, bisector.scale, brick_name.face_scaling());
        lbfgs(&mut bake);
        let strain = bake.mean_face_strain();
        if (strain - BakedBrick::TARGET_FACE_STRAIN).abs() <= STRAIN_TOLERANCE {
            break (bake, names, bisector.scale);
        }
        let next = bisector.next_scale(strain);
        if (next - bisector.scale).abs() < 1.0e-9
            || (bisector.low.is_some() && bisector.high.is_some()
                && (bisector.high.unwrap() - bisector.low.unwrap()).abs() < 1.0e-6)
        {
            break (bake, names, bisector.scale);
        }
        bisector.scale = next;
        let _ = MAX_BISECTION_ROUNDS;
    };
    let (mut bake, joint_names, scale) = final_bake;

    // Form is found; now set the forces: shrink-wrap every member onto the
    // settled geometry at the designed pretension band.
    shrink_wrap(&mut bake);

    // Reorient on the brick's max_seed role (matches the Oven's
    // visual-orientation choice).
    let centroid = bake.centroid();
    bake.translate(-centroid);
    let reorient = bake.down_rotation(proto.max_seed());
    bake.apply_matrix(reorient);
    let centroid = bake.centroid();
    bake.translate(-centroid);

    if let Some(symmetry) = proto.symmetry(THREEFOLD_ROLE) {
        symmetrize(&mut bake, &symmetry, reorient);
        verify_symmetry(&bake, &symmetry, reorient, brick_name);
    }

    // Symmetrize moved positions slightly; pin every member to exactly the
    // pretension band on the final (symmetric) geometry so stored strains
    // are identical across rotational triples.
    let _ = assign_banded_rests(&mut bake);

    // A brick must be a solid tensegrity: every push in compression, every
    // pull in tension. Panics with a per-member report otherwise.
    validate_pretension(&bake, &joint_names, brick_name);

    let joints: Vec<BakedJoint> = bake.positions[..bake.structural]
        .iter()
        .map(|p| BakedJoint { location: *p })
        .collect();
    let mut intervals: Vec<BakedInterval> = Vec::new();
    for s in &bake.springs {
        // Face radials never enter the baked output.
        if s.alpha >= bake.structural || s.omega >= bake.structural {
            continue;
        }
        let l = (bake.positions[s.omega] - bake.positions[s.alpha]).length();
        let strain = if l > 0.0 { (l - s.rest_length) / s.rest_length } else { 0.0 };
        let material_name = if s.is_push { "push" } else { "pull" }.to_string();
        intervals.push(BakedInterval {
            alpha_index: s.alpha,
            omega_index: s.omega,
            strain,
            material_name,
        });
    }

    BakedBrick {
        params,
        scale,
        joints,
        intervals,
        faces: proto.derive_baked_faces(brick_name.face_scaling()),
    }
}

/// Re-derive each member's rest length from its settled length at the
/// designed pretension band, re-settle, and repeat to a fixed point. Face
/// radials keep their absolute rests — they anchor the brick's size and the
/// standard face geometry for attachment.
/// Set every member's rest length so its current length sits exactly at the
/// designed pretension band. Returns the largest relative rest change.
fn assign_banded_rests(bake: &mut Bake) -> f32 {
    let mut max_rel_change = 0.0f32;
    for i in 0..bake.springs.len() {
        let s = bake.springs[i];
        if s.alpha >= bake.structural || s.omega >= bake.structural {
            continue;
        }
        let length = (bake.positions[s.omega] - bake.positions[s.alpha]).length();
        if length <= 0.0 {
            continue;
        }
        let new_rest = if s.is_push {
            length / (1.0 - SHRINK_WRAP_PRETENSION)
        } else {
            length / (1.0 + SHRINK_WRAP_PRETENSION)
        };
        let rel_change = ((new_rest - s.rest_length) / s.rest_length).abs();
        max_rel_change = max_rel_change.max(rel_change);
        bake.springs[i].rest_length = new_rest;
    }
    max_rel_change
}

fn shrink_wrap(bake: &mut Bake) {
    for _ in 0..SHRINK_WRAP_ROUNDS {
        let max_rel_change = assign_banded_rests(bake);
        lbfgs(bake);
        if max_rel_change < SHRINK_WRAP_REST_TOL {
            break;
        }
    }
}

/// Enforce the definition of a solid tensegrity on the finished bake:
/// every push strictly in compression, every pull strictly in tension.
/// Panics with a per-member report naming the offenders otherwise.
fn validate_pretension(bake: &Bake, joint_names: &[JointName], brick_name: BrickName) {
    let mut violations: Vec<String> = Vec::new();
    let mut push_strains: Vec<f32> = Vec::new();
    let mut pull_strains: Vec<f32> = Vec::new();

    for s in &bake.springs {
        if s.alpha >= bake.structural || s.omega >= bake.structural {
            continue;
        }
        let length = (bake.positions[s.omega] - bake.positions[s.alpha]).length();
        let strain = (length - s.rest_length) / s.rest_length;
        if s.is_push {
            push_strains.push(strain);
        } else {
            pull_strains.push(strain);
        }
        let slack_or_wrong_signed = if s.is_push {
            strain > -SLACK_EPSILON
        } else {
            strain < SLACK_EPSILON
        };
        if slack_or_wrong_signed {
            violations.push(format!(
                "{:?}-{:?} {} strain {:+.4}",
                joint_names[s.alpha],
                joint_names[s.omega],
                if s.is_push { "push" } else { "pull" },
                strain,
            ));
        }
    }

    if !violations.is_empty() {
        let span = |v: &[f32]| -> String {
            let min = v.iter().copied().fold(f32::MAX, f32::min);
            let max = v.iter().copied().fold(f32::MIN, f32::max);
            format!("{:+.4}..{:+.4}", min, max)
        };
        panic!(
            "{brick_name:?} is not a solid tensegrity — {} slack or wrong-signed member(s):\n  {}\n\
             (push strains {}, pull strains {})",
            violations.len(),
            violations.join("\n  "),
            span(&push_strains),
            span(&pull_strains),
        );
    }
}

/// The declared symmetry's rotation axis transformed into world space
/// via the rigid `reorient` the bake just applied.
fn world_symmetry_quat(symmetry: &BrickSymmetry, reorient: Mat4) -> Quat {
    let axis_world = reorient.transform_vector3(symmetry.axis).normalize();
    Quat::from_axis_angle(axis_world, symmetry.angle())
}

/// Orbit-average structural joints onto the symmetric manifold.
/// Currently handles 3-fold cyclic only.
fn symmetrize(bake: &mut Bake, symmetry: &BrickSymmetry, reorient: Mat4) {
    if symmetry.order != 3 {
        return;
    }
    let n_struct = bake.structural;
    if n_struct == 0 {
        return;
    }
    let centre: Vec3 =
        bake.positions[..n_struct].iter().copied().sum::<Vec3>() / n_struct as f32;
    let centred: Vec<Vec3> = bake.positions[..n_struct]
        .iter()
        .map(|p| *p - centre)
        .collect();

    let rotation = world_symmetry_quat(symmetry, reorient);
    let rot_inv = rotation.inverse();

    let mut nearest: Vec<usize> = vec![0; n_struct];
    for i in 0..n_struct {
        let target = rotation * centred[i];
        let mut best = 0usize;
        let mut best_d = f32::INFINITY;
        for j in 0..n_struct {
            let d = (centred[j] - target).length_squared();
            if d < best_d {
                best_d = d;
                best = j;
            }
        }
        nearest[i] = best;
    }

    let mut seen = vec![false; n_struct];
    let mut sym = centred.clone();
    for i in 0..n_struct {
        if seen[i] { continue; }
        let i_b = nearest[i];
        let i_c = nearest[i_b];
        if nearest[i_c] != i || i_b == i || i_c == i {
            seen[i] = true;
            continue;
        }
        let mean_a = (centred[i] + rot_inv * centred[i_b] + rot_inv * rot_inv * centred[i_c]) / 3.0;
        sym[i] = mean_a;
        sym[i_b] = rotation * mean_a;
        sym[i_c] = rotation * rotation * mean_a;
        seen[i] = true;
        seen[i_b] = true;
        seen[i_c] = true;
    }

    for i in 0..n_struct {
        bake.positions[i] = sym[i] + centre;
    }
}

/// Loud-fail if the baked joints aren't invariant under the declared
/// symmetry within tolerance.
fn verify_symmetry(
    bake: &Bake,
    symmetry: &BrickSymmetry,
    reorient: Mat4,
    brick_name: BrickName,
) {
    let n_struct = bake.structural;
    if n_struct == 0 {
        return;
    }
    let centre: Vec3 =
        bake.positions[..n_struct].iter().copied().sum::<Vec3>() / n_struct as f32;
    let centred: Vec<Vec3> = bake.positions[..n_struct]
        .iter()
        .map(|p| *p - centre)
        .collect();
    let rotation = world_symmetry_quat(symmetry, reorient);
    let mut worst: f32 = 0.0;
    for p in &centred {
        let target = rotation * *p;
        let mut nearest = f32::INFINITY;
        for q in &centred {
            let d = (target - *q).length();
            if d < nearest { nearest = d; }
        }
        if nearest > worst { worst = nearest; }
    }
    assert!(
        worst < SYMMETRY_VERIFICATION_TOLERANCE,
        "{brick_name}: baked brick not invariant under declared symmetry — \
         residual {worst:.2e} m > tolerance {SYMMETRY_VERIFICATION_TOLERANCE:.0e}"
    );
}


#[cfg(test)]
mod pretension_tests {
    use crate::build::dsl::brick_dsl::BrickName;
    use crate::build::dsl::brick_library;
    use strum::IntoEnumIterator;

    /// Every baked brick must be a solid tensegrity. `validate_pretension`
    /// panics inside `bake_brick_pure` when any member ends slack or
    /// wrong-signed, so simply baking each brick is the whole assertion.
    #[test]
    fn all_bricks_are_solid_tensegrities() {
        for brick_name in BrickName::iter() {
            let _ = brick_library::get_scale(brick_name);
        }
    }
}
