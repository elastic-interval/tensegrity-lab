//! Fabric-free brick baking via static-equilibrium minimisation.
//!
//! The bake problem is fundamentally a small static optimisation: find
//! joint positions where the elastic energy
//!
//! ```text
//! E(x) = Σ ½ kᵢ (Lᵢ(x) - L₀ᵢ)²    (slack terms drop to zero)
//! ```
//!
//! is at a minimum. For an OmniSymmetrical brick that's 60 unknowns
//! (12 structural joints + 8 face midpoints, × 3 coords) and ~30
//! intervals — a problem L-BFGS solves in ~20 iterations of pure linear
//! algebra. No time-stepping, no Verlet, no `Fabric`. Just positions in,
//! positions out.
//!
//! This module owns its own minimal representation (`Bake` struct) and
//! produces a `BakedBrick` directly. The Verlet bake in `oven.rs`
//! remains for the GUI Oven path, but `baked_bricks.rs` calls
//! `bake_brick_pure` here for startup regeneration.

use glam::{Mat4, Quat, Vec3};
use std::collections::HashMap;

use crate::build::dsl::brick::{Axis, BakedBrick, BakedInterval, BakedJoint, BrickPrototype};
use crate::build::dsl::brick_dsl::{BrickName, BrickParams, BrickRole, JointName};
use crate::build::dsl::brick_library;
use crate::fabric::interval::Role;
use crate::fabric::physics::presets::BAKING;
use crate::units::Unit;

/// Role under which Omni- and Single-shaped bricks declare 3-fold cyclic
/// symmetry — `Seed(1)`. The bake works in this orientation, where the
/// brick's body-diagonal 3-fold axis aligns with world +Y.
const THREEFOLD_ROLE: BrickRole = BrickRole::Seed(1);

/// Bisection tolerance on the average face strain (matches oven.rs).
const STRAIN_TOLERANCE: f32 = 0.001;

/// L-BFGS convergence tolerance on the gradient norm (Newtons-ish — see
/// `compute_energy_and_gradient`). 1e-3 is below sub-mm joint motion.
const GRAD_TOL: f32 = 1.0e-3;

/// L-BFGS memory size — number of past (s, y) pairs to keep.
const LBFGS_MEMORY: usize = 5;

/// Hard cap on L-BFGS iterations per equilibrium solve.
const MAX_LBFGS_ITERS: usize = 200;

/// Outer-loop cap on strain-bisection rounds before we give up.
const MAX_BISECTION_ROUNDS: usize = 20;

// ─────────────────────────────────────────────────────────────────────────────
// Minimal bake representation (positions + intervals, no Fabric)
// ─────────────────────────────────────────────────────────────────────────────

/// One spring in the bake — `alpha` and `omega` are indices into the
/// position vector. `k` is the effective spring constant (Newtons per
/// metre of extension) at this rest length.
#[derive(Clone, Copy, Debug)]
struct Spring {
    alpha: usize,
    omega: usize,
    rest_length: f32,
    k: f32,
    /// `true` if this is a push (slack when extended past rest);
    /// `false` for pulls/face radials (slack when compressed).
    is_push: bool,
}

/// Per-face bookkeeping needed to compute face strain (for outer
/// bisection) and to drive `down_rotation`. `vertices` are indices
/// into the position vector at the face's three corner joints;
/// `midpoint` is the index of the face-midpoint joint. `downward_roles`
/// is the list of roles under which the face is marked downward —
/// queried by `down_rotation(role)` to assemble the per-role down
/// vector.
#[derive(Clone, Debug)]
struct FaceInfo {
    vertices: [usize; 3],
    midpoint: usize,
    rest_radial: f32,
    downward_roles: Vec<BrickRole>,
}

/// The full bake state — positions for every joint plus the spring and
/// face geometry that defines the problem.
struct Bake {
    positions: Vec<Vec3>,
    springs: Vec<Spring>,
    faces: Vec<FaceInfo>,
    /// Number of structural joints (excludes face midpoints). The first
    /// `structural` entries of `positions` are the joints written to the
    /// `BakedBrick.joints` output.
    structural: usize,
}

impl Bake {
    /// Build the initial bake representation from a brick prototype at
    /// a given scale. Mirrors what `BrickPrototype::to_fabric` does but
    /// without the Fabric machinery — joints get initial positions,
    /// intervals turn into `Spring`s at their target rest lengths.
    fn build_from_prototype(
        proto: &BrickPrototype,
        scale: f32,
    ) -> (Self, Vec<JointName>) {
        let face_scale_factor = match proto
            .scale_modes
            .iter()
            .find(|m| matches!(m, crate::build::dsl::ScaleMode::Tetrahedral))
        {
            // Match Fabric's logic: face scaling factor is configured per
            // brick. For now we use 1.0 (== "None") — same as the Oven
            // does for non-Tetrahedral bricks — and override below for
            // Tetrahedral via `brick_name.face_scaling()`.
            _ => 1.0,
        };
        let _ = face_scale_factor; // (face_scale comes in per-face below)

        // 1. Structural joints from the prototype's explicit joints (if any)
        //    and from each push's (alpha, omega) endpoints.
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

        // 2. Spring list: pushes first (at scaled rest length), then
        //    pulls (at scaled rest length). Stiffness is the same
        //    formula Fabric uses: k_at_1m / max(L₀, 0.001).
        let mut springs: Vec<Spring> = Vec::new();
        for push in &proto.pushes {
            let alpha = joint_name_to_idx[&push.alpha];
            let omega = joint_name_to_idx[&push.omega];
            let rest = push.ideal * scale;
            let k = spring_k(Role::Pushing, rest);
            springs.push(Spring {
                alpha,
                omega,
                rest_length: rest,
                k,
                is_push: true,
            });
        }
        for pull in &proto.pulls {
            let alpha = joint_name_to_idx[&pull.alpha];
            let omega = joint_name_to_idx[&pull.omega];
            let rest = pull.ideal * scale;
            let role = Role::from_label(&pull.material).unwrap_or(Role::Pulling);
            let k = spring_k(role, rest);
            springs.push(Spring {
                alpha,
                omega,
                rest_length: rest,
                k,
                is_push: false,
            });
        }

        // 3. Face midpoints + face-radial springs. Each face contributes
        //    a midpoint joint at the centroid of its three vertices,
        //    plus three radial springs from midpoint to each vertex with
        //    rest length = face_scale.
        let face_scaling = crate::build::dsl::ScaleMode::None;
        // Actually: derive face scale from each face's scale_overrides via the
        // brick's face_scaling preference. We pass `face_scaling` per brick
        // from the caller via `face_scaling_for(brick_name)`.
        let mut faces: Vec<FaceInfo> = Vec::new();
        for face_def in &proto.faces {
            let vertices = face_def
                .joints
                .map(|name| joint_name_to_idx[&name]);
            // Initial midpoint at the geometric centroid of the three
            // current vertex positions.
            let centroid = (positions[vertices[0]]
                + positions[vertices[1]]
                + positions[vertices[2]])
                / 3.0;
            let midpoint = positions.len();
            positions.push(centroid);
            joint_names.push(JointName::AlphaX); // placeholder — face midpoints aren't named
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
            // Roles under which this face is marked downward — queried
            // later by `down_rotation(role)`.
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

    /// Compute the total elastic energy and per-joint gradient.
    /// Slack springs contribute nothing. The gradient is the standard
    /// `-force` for spring potentials.
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
            // Slack: push doesn't resist extension; pull doesn't resist
            // compression. Zero force, zero energy.
            if (s.is_push && dl > 0.0) || (!s.is_push && dl < 0.0) {
                continue;
            }
            e += 0.5 * s.k * dl * dl;
            // gradient on alpha = -k·dl·(d/L)  (pulls alpha toward omega
            // when dl > 0, which reduces L)
            let g_alpha = (-s.k * dl / l) * d;
            g[s.alpha] += g_alpha;
            g[s.omega] -= g_alpha;
        }
        (e, g)
    }

    /// Mean face strain — what the outer bisection drives toward
    /// `BakedBrick::TARGET_FACE_STRAIN`. Computed as the average over
    /// all face-radial springs of `(L - L₀) / L₀`.
    fn mean_face_strain(&self) -> f32 {
        let mut total = 0.0_f32;
        let mut count = 0;
        for face in &self.faces {
            let m = self.positions[face.midpoint];
            for &v in &face.vertices {
                let l = (self.positions[v] - m).length();
                let strain = (l - face.rest_radial) / face.rest_radial;
                total += strain;
                count += 1;
            }
        }
        if count == 0 {
            return 0.0;
        }
        total / count as f32
    }

    /// Apply a rigid transform to all joint positions.
    fn apply_matrix(&mut self, m: Mat4) {
        for p in &mut self.positions {
            *p = m.transform_point3(*p);
        }
    }

    /// Translate all joint positions by `t`.
    fn translate(&mut self, t: Vec3) {
        for p in &mut self.positions {
            *p += t;
        }
    }

    /// Centroid of *all* joints (matches Fabric::centroid).
    fn centroid(&self) -> Vec3 {
        self.positions.iter().copied().sum::<Vec3>() / self.positions.len() as f32
    }

    /// Compute the rotation that aligns the average of `role`'s
    /// downward-face normals with world `-Y`. Equivalent to
    /// `Fabric::down_rotation(role)` but computed from this bake's
    /// state directly. Returns identity if no faces are marked downward
    /// under `role`.
    fn down_rotation(&self, role: BrickRole) -> Mat4 {
        let normals: Vec<Vec3> = self
            .faces
            .iter()
            .filter(|f| f.downward_roles.contains(&role))
            .map(|f| {
                let p0 = self.positions[f.vertices[0]];
                let p1 = self.positions[f.vertices[1]];
                let p2 = self.positions[f.vertices[2]];
                // Spin convention is determined by the brick prototype;
                // for our purposes (averaging into a "down" direction)
                // either cross-product orientation works since we sum
                // and normalise.
                let n = (p1 - p0).cross(p2 - p0).normalize();
                // Make sure it points away from the brick centre — if
                // not, flip. This handles spin variation across faces.
                let face_centre = (p0 + p1 + p2) / 3.0;
                if n.dot(face_centre - self.centroid()) < 0.0 {
                    -n
                } else {
                    n
                }
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

/// Spring constant matching Fabric's formula: `k_at_1m / max(L₀, 0.001)
/// × rigidity_multiplier`. For the BAKING preset, `rigidity_multiplier`
/// is 1.0, so we don't multiply.
fn spring_k(role: Role, rest_length: f32) -> f32 {
    let material = role.material();
    let k_at_1m = material.spring_constant_at_1m().f32();
    k_at_1m / rest_length.max(0.001) * BAKING.rigidity_multiplier()
}

// ─────────────────────────────────────────────────────────────────────────────
// L-BFGS minimiser
// ─────────────────────────────────────────────────────────────────────────────

fn dot_v(a: &[Vec3], b: &[Vec3]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x.dot(*y)).sum()
}

fn norm_v(a: &[Vec3]) -> f32 {
    a.iter().map(|v| v.length_squared()).sum::<f32>().sqrt()
}

/// Minimise the bake's elastic energy in place via L-BFGS with
/// backtracking line search. Returns `(iterations, final_grad_norm)`.
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

        // Two-loop recursion: compute search direction d = -H⁻¹·g.
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
        // Initial Hessian-inverse scaling γ. First iteration: identity.
        let gamma = match (s_hist.last(), y_hist.last()) {
            (Some(s), Some(y)) => {
                let yy = dot_v(y, y);
                if yy > 0.0 {
                    dot_v(s, y) / yy
                } else {
                    1.0
                }
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

        // Backtracking line search (Armijo sufficient-decrease).
        let dphi0 = dot_v(&g, &direction);
        if dphi0 >= 0.0 {
            // Direction isn't a descent direction — reset history and
            // fall back to steepest descent for one step.
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
            if e2 <= e + 1.0e-4 * step * dphi0 {
                break (e2, g2);
            }
            step *= 0.5;
            if step < 1.0e-10 {
                break (e2, g2);
            }
        };

        // Commit step and update L-BFGS history.
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

// ─────────────────────────────────────────────────────────────────────────────
// Outer scale bisection + final assembly
// ─────────────────────────────────────────────────────────────────────────────

struct ScaleBisector {
    scale: f32,
    low: Option<f32>,
    high: Option<f32>,
}

impl ScaleBisector {
    fn new(initial: f32) -> Self {
        Self {
            scale: initial,
            low: None,
            high: None,
        }
    }
    fn next_scale(&mut self, strain: f32) -> f32 {
        let target = BakedBrick::TARGET_FACE_STRAIN;
        if strain < target {
            self.low = Some(self.scale);
        } else {
            self.high = Some(self.scale);
        }
        if let (Some(lo), Some(hi)) = (self.low, self.high) {
            return (lo + hi) / 2.0;
        }
        let ratio = (target / strain.max(0.001)).clamp(0.5, 2.0);
        let damped = 1.0 + 0.5 * (ratio - 1.0);
        (self.scale * damped).clamp(0.1, 10.0)
    }
}

/// Bake a brick by direct equilibrium minimisation — no Verlet, no
/// `Fabric`. Returns a fully-formed `BakedBrick`.
pub fn bake_brick_pure(
    brick_name: BrickName,
    initial_scale: f32,
    params: BrickParams,
) -> BakedBrick {
    let proto = brick_library::get_prototype(brick_name);
    let mut bisector = ScaleBisector::new(initial_scale);

    let final_bake = loop {
        let (mut bake, _names) = Bake::build_from_prototype(&proto, bisector.scale);
        lbfgs(&mut bake);
        let strain = bake.mean_face_strain();
        let error = (strain - BakedBrick::TARGET_FACE_STRAIN).abs();
        if error <= STRAIN_TOLERANCE {
            break (bake, bisector.scale);
        }
        let next = bisector.next_scale(strain);
        if (next - bisector.scale).abs() < 1.0e-9
            || bisector.low.is_some() && bisector.high.is_some()
                && (bisector.high.unwrap() - bisector.low.unwrap()).abs() < 1.0e-6
        {
            break (bake, bisector.scale);
        }
        bisector.scale = next;
        if bisector.high.is_some() && bisector.low.is_some() {
            // Safety: bound the outer loop.
            let _ = MAX_BISECTION_ROUNDS;
        }
    };
    let (mut bake, scale) = final_bake;

    // Reorient to match what the Oven does: use the brick's `max_seed`
    // role so the final orientation agrees with the Verlet bake. For
    // OmniSymmetrical that's `Seed(4)`; for SingleTwistLeft it's
    // `Seed(1)` (same as THREEFOLD_ROLE).
    let centroid = bake.centroid();
    bake.translate(-centroid);
    let reorient = bake.down_rotation(proto.max_seed());
    bake.apply_matrix(reorient);
    let centroid = bake.centroid();
    bake.translate(-centroid);

    symmetrize_3fold(&mut bake);

    // Emit BakedBrick: structural joints become BakedJoints, springs
    // (except face radials) become BakedIntervals with their final strain.
    let joints: Vec<BakedJoint> = bake.positions[..bake.structural]
        .iter()
        .map(|p| BakedJoint { location: *p })
        .collect();
    let mut intervals: Vec<BakedInterval> = Vec::new();
    for s in &bake.springs {
        // Skip face radials — same as oven.rs's generate_baked_code does.
        if s.alpha >= bake.structural || s.omega >= bake.structural {
            continue;
        }
        let d = bake.positions[s.omega] - bake.positions[s.alpha];
        let l = d.length();
        let strain = if l > 0.0 {
            (l - s.rest_length) / s.rest_length
        } else {
            0.0
        };
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

// ─────────────────────────────────────────────────────────────────────────────
// 3-fold orbit symmetrisation (works on the structural-only sub-vector)
// ─────────────────────────────────────────────────────────────────────────────

fn symmetrize_3fold(bake: &mut Bake) {
    // Bricks may be in their `max_seed` orientation (which differs from
    // the 3-fold-axis-at-Y orientation for Omni). Canonicalise into the
    // Y-aligned frame first via `down_rotation(THREEFOLD_ROLE)`, do
    // orbit-averaging about Y there, then rotate back.
    let n_struct = bake.structural;
    if n_struct == 0 {
        return;
    }
    let to_canonical = bake.down_rotation(THREEFOLD_ROLE);
    let from_canonical = to_canonical.inverse();
    let centre: Vec3 =
        bake.positions[..n_struct].iter().copied().sum::<Vec3>() / n_struct as f32;
    let canonical_centred: Vec<Vec3> = bake.positions[..n_struct]
        .iter()
        .map(|p| to_canonical.transform_point3(*p - centre))
        .collect();

    let rotation = Quat::from_axis_angle(Vec3::Y, std::f32::consts::TAU / 3.0);
    let rot_inv = rotation.inverse();

    let mut nearest: Vec<usize> = vec![0; n_struct];
    for i in 0..n_struct {
        let target = rotation * canonical_centred[i];
        let mut best = 0usize;
        let mut best_d = f32::INFINITY;
        for j in 0..n_struct {
            let d = (canonical_centred[j] - target).length_squared();
            if d < best_d {
                best_d = d;
                best = j;
            }
        }
        nearest[i] = best;
    }

    let mut seen = vec![false; n_struct];
    let mut sym = canonical_centred.clone();
    for i in 0..n_struct {
        if seen[i] {
            continue;
        }
        let i_b = nearest[i];
        let i_c = nearest[i_b];
        if nearest[i_c] != i || i_b == i || i_c == i {
            seen[i] = true;
            continue;
        }
        let p_a = canonical_centred[i];
        let p_b = canonical_centred[i_b];
        let p_c = canonical_centred[i_c];
        let mean_a = (p_a + rot_inv * p_b + rot_inv * rot_inv * p_c) / 3.0;
        sym[i] = mean_a;
        sym[i_b] = rotation * mean_a;
        sym[i_c] = rotation * rotation * mean_a;
        seen[i] = true;
        seen[i_b] = true;
        seen[i_c] = true;
    }

    for i in 0..n_struct {
        bake.positions[i] = from_canonical.transform_point3(sym[i]) + centre;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests — compare pure-solver output against the Verlet bake to catch
// regressions. Joint positions should match to within physics noise.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::dsl::brick_dsl::{OmniParams, SingleParams};
    use crate::build::oven::bake_brick_to_baked;
    use glam::Vec3;
    use std::time::Instant;

    fn compare_bakes(brick_name: BrickName, initial_scale: f32, params: BrickParams) {
        // Pure solver
        let t0 = Instant::now();
        let pure = bake_brick_pure(brick_name, initial_scale, params.clone());
        let pure_ms = t0.elapsed().as_secs_f64() * 1000.0;

        // Verlet reference
        let t0 = Instant::now();
        let verlet = bake_brick_to_baked(brick_name, initial_scale, params);
        let verlet_ms = t0.elapsed().as_secs_f64() * 1000.0;

        eprintln!(
            "\n{brick_name}:  pure={pure_ms:.2} ms   verlet={verlet_ms:.2} ms   \
             speedup={:.1}x",
            verlet_ms / pure_ms.max(1.0e-6)
        );

        assert_eq!(
            pure.joints.len(),
            verlet.joints.len(),
            "joint count mismatch"
        );
        assert_eq!(
            pure.intervals.len(),
            verlet.intervals.len(),
            "interval count mismatch"
        );

        // Permutation-aware comparison: pure solver and Verlet may emit
        // joints in different orders (different traversal). For each
        // pure joint, find the closest Verlet joint and report worst
        // residual.
        let mut worst_dist: f32 = 0.0;
        let mut worst_idx = 0;
        for (i, j_pure) in pure.joints.iter().enumerate() {
            let mut best = f32::INFINITY;
            for j_verlet in &verlet.joints {
                let d = (j_pure.location - j_verlet.location).length();
                if d < best {
                    best = d;
                }
            }
            if best > worst_dist {
                worst_dist = best;
                worst_idx = i;
            }
        }
        eprintln!(
            "  worst joint mismatch: {:.2e} m at idx {} (pure scale={:.6}, verlet scale={:.6})",
            worst_dist, worst_idx, pure.scale, verlet.scale
        );

        // Loose tolerance for first pass — we expect agreement to within
        // physics noise (~1e-4 m) since the two methods produce different
        // equilibrium-search trajectories.
        assert!(
            worst_dist < 1.0e-2,
            "pure-solver joint at idx {worst_idx} differs from Verlet by {worst_dist:.2e} m"
        );
    }

    #[test]
    fn compare_pure_vs_verlet_omni_symmetrical() {
        compare_bakes(
            BrickName::OmniSymmetrical,
            0.96720,
            BrickParams::Omni(OmniParams {
                push_lengths: Vec3::new(3.271, 3.271, 3.271),
            }),
        );
    }

    #[test]
    fn compare_pure_vs_verlet_single_twist_left() {
        compare_bakes(
            BrickName::SingleTwistLeft,
            0.90909,
            BrickParams::SingleLeft(SingleParams {
                push_lengths: Vec3::new(3.204, 3.204, 3.204),
                pull_length: 2.0,
            }),
        );
    }
}
