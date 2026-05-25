use crate::build::dsl::brick::{BakedBrick, BakedInterval, BakedJoint, BrickPrototype};
use crate::build::dsl::brick_dsl::BrickParams;
use crate::build::dsl::brick_dsl::{BrickName, BrickRole};
use crate::build::dsl::brick_library;
use crate::crucible_context::CrucibleContext;
use crate::fabric::interval::Role;
use crate::fabric::physics::presets::BAKING;
use crate::fabric::{Fabric, IntervalKey, JointKey};
use crate::{Radio, StateChange};
use glam::{Quat, Vec3};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use strum::IntoEnumIterator;

/// Role under which Omni- and Single-shaped bricks declare 3-fold cyclic
/// symmetry — `down_rotation(THREEFOLD_ROLE)` aligns the brick's
/// body-diagonal 3-fold axis with world +Y. See `symmetrize_brick_3fold`.
const THREEFOLD_ROLE: BrickRole = BrickRole::Seed(1);

/// Safety cap on bake duration. Reached only if the system never settles
/// (which shouldn't happen with the BAKING preset's strong damping).
/// In practice the convergence check below kicks in well before this.
const BAKED_DURATION: Duration = Duration::from_secs(2);

/// Reorient the brick at this time so user can see it
const REORIENT_DURATION: Duration = Duration::from_millis(500);

/// Stop the second physics burst once max joint speed drops below this
/// (m/s). 1 cm/s — well below any meaningful joint motion at our scale,
/// reached in ~1 s fabric time on both Omni and Single bricks (~half the
/// safety cap). The post-bake `symmetrize_brick_3fold` projection cleans
/// up the residual sub-mm motion anyway.
const CONVERGENCE_SPEED_M_PER_S: f32 = 1.0e-2;

/// Run physics for at least this long after reorientation before checking
/// the convergence criterion — otherwise we'd catch the freshly-zeroed
/// velocities and stop immediately.
const MIN_PHYSICS_AFTER_REORIENT: Duration = Duration::from_millis(150);

/// Tolerance for face strain convergence
const STRAIN_TOLERANCE: f32 = 0.001;

struct TuningState {
    scale: f32,
    low_scale: Option<f32>,  // Scale that gave strain < target
    high_scale: Option<f32>, // Scale that gave strain > target
    iteration: usize,
}

impl TuningState {
    fn new(initial_scale: f32) -> Self {
        Self {
            scale: initial_scale,
            low_scale: None,
            high_scale: None,
            iteration: 0,
        }
    }
}

pub struct Oven {
    brick_names: Vec<BrickName>,
    current_index: usize,
    radio: Radio,
    baked_fabrics: Vec<Option<Fabric>>,
    reoriented: bool,
    tuning: TuningState,
}

impl Oven {
    pub fn new(radio: Radio) -> Self {
        // Only include non-derived bricks
        let brick_names: Vec<BrickName> = BrickName::iter()
            .filter(|name| !brick_library::is_derived(*name))
            .collect();
        let baked_fabrics = vec![None; brick_names.len()];
        let initial_scale = brick_library::get_scale(brick_names[0]);

        Self {
            brick_names,
            current_index: 0,
            radio,
            baked_fabrics,
            reoriented: false,
            tuning: TuningState::new(initial_scale),
        }
    }

    pub fn current_brick_name(&self) -> BrickName {
        self.brick_names[self.current_index]
    }

    /// Check if the current brick is already baked
    fn current_is_baked(&self) -> bool {
        self.baked_fabrics[self.current_index].is_some()
    }

    /// Check if all bricks are baked
    fn all_baked(&self) -> bool {
        self.baked_fabrics.iter().all(|f| f.is_some())
    }

    /// Find the next unbaked brick index, if any
    fn next_unbaked_index(&self) -> Option<usize> {
        for i in 0..self.brick_names.len() {
            let index = (self.current_index + 1 + i) % self.brick_names.len();
            if self.baked_fabrics[index].is_none() {
                return Some(index);
            }
        }
        None
    }

    pub fn create_fresh_fabric(&self) -> Fabric {
        let brick_name = self.current_brick_name();
        let prototype = brick_library::get_prototype(brick_name);
        let scaled = scale_prototype(&prototype, self.tuning.scale);
        scaled.to_fabric(brick_name.face_scaling())
    }

    /// Get the fabric for the current brick - either baked or fresh
    fn current_fabric(&self) -> Fabric {
        if let Some(fabric) = &self.baked_fabrics[self.current_index] {
            fabric.clone()
        } else {
            self.create_fresh_fabric()
        }
    }

    pub fn next_brick(&mut self) -> Fabric {
        self.current_index = (self.current_index + 1) % self.brick_names.len();
        self.reoriented = false;
        self.tuning = TuningState::new(brick_library::get_scale(self.current_brick_name()));
        self.send_name_and_label();
        self.current_fabric()
    }

    /// Send fabric name and stage label for current brick
    fn send_name_and_label(&self) {
        StateChange::SetFabricName(format!("{}", self.current_brick_name())).send(&self.radio);
        self.send_stage_label();
    }

    /// Send the appropriate stage label based on baked state
    pub fn send_stage_label(&self) {
        let label = if self.current_is_baked() {
            if self.all_baked() {
                "All Baked"
            } else {
                "Baked"
            }
        } else {
            "Baking"
        };
        StateChange::SetStageLabel(label.to_string()).send(&self.radio);
    }

    pub fn copy_physics_into(&self, context: &mut CrucibleContext) {
        *context.physics = BAKING;
    }

    fn compute_new_scale(&mut self, current_strain: f32) -> f32 {
        next_scale(&mut self.tuning, current_strain)
    }

    pub fn iterate(&mut self, context: &mut CrucibleContext) -> Option<Fabric> {
        if self.current_is_baked() {
            return None;
        }

        for _ in 0..60 {
            context.fabric.iterate(context.physics);
        }

        if !self.reoriented && context.fabric.age.as_duration() >= REORIENT_DURATION {
            // First move centroid to origin so rotation is around the centroid
            let centroid = context.fabric.centroid();
            context.fabric.apply_translation(-centroid);
            // Now rotate around the (centered) origin
            let prototype = brick_library::get_prototype(self.current_brick_name());
            let rotation = context.fabric.down_rotation(prototype.max_seed());
            context.fabric.apply_matrix4(rotation);
            // Finally centralize with bottom at y=0
            let translation = context.fabric.centralize_translation(Some(0.0));
            context.fabric.apply_translation(translation);
            context.fabric.zero_velocities();
            self.reoriented = true;
        }

        // Stop early if the system has effectively settled — max joint
        // speed below threshold AND we've had enough time post-reorient
        // for the system to actually pick up speed first. Falls back on
        // the BAKED_DURATION safety cap.
        let age = context.fabric.age.as_duration();
        let post_reorient = age.saturating_sub(REORIENT_DURATION);
        let settled = self.reoriented
            && post_reorient >= MIN_PHYSICS_AFTER_REORIENT
            && context.fabric.stats.max_speed < CONVERGENCE_SPEED_M_PER_S;
        let timed_out = age >= BAKED_DURATION;

        if settled || timed_out {
            let current_strain = average_face_strain(&context.fabric);
            let error = (current_strain - BakedBrick::TARGET_FACE_STRAIN).abs();

            if error > STRAIN_TOLERANCE {
                let new_scale = self.compute_new_scale(current_strain);
                println!(
                    "Tuning {}: strain={:.4}, scale {:.4} -> {:.4}",
                    self.current_brick_name(),
                    current_strain,
                    self.tuning.scale,
                    new_scale
                );

                self.tuning.scale = new_scale;
                self.tuning.iteration += 1;
                self.reoriented = false;

                return Some(self.create_fresh_fabric());
            }

            let final_scale = self.tuning.scale;
            if self.tuning.iteration > 0 {
                println!(
                    "Tuned {} in {} iterations: scale={:.4}, strain={:.4}",
                    self.current_brick_name(),
                    self.tuning.iteration,
                    final_scale,
                    current_strain
                );
            }

            let brick_name = self.current_brick_name();
            // Snap onto the brick's 3-fold-symmetric manifold before
            // snapshotting. Drops residual asymmetry from physics drift
            // (~4 μm for Omni, ~25 μm for Single) to canonicalisation
            // noise (~1e-7 m), so the baked literals are truly symmetric.
            // No-op for bricks without 3-fold cyclic symmetry declared.
            symmetrize_brick_3fold(&mut context.fabric, brick_name);
            let code = self.generate_baked_code(&context.fabric, final_scale);
            self.baked_fabrics[self.current_index] = Some(context.fabric.clone());
            self.export_brick(brick_name, &code);

            if let Some(next_index) = self.next_unbaked_index() {
                self.current_index = next_index;
                self.reoriented = false;
                self.tuning =
                    TuningState::new(brick_library::get_scale(self.brick_names[next_index]));
                self.send_name_and_label();
                // Jump camera to view new brick at good distance
                StateChange::JumpToFabric.send(&self.radio);
                return Some(self.create_fresh_fabric());
            } else {
                self.send_stage_label();
            }
        }

        None
    }

    fn generate_baked_code(&self, fabric: &Fabric, scale: f32) -> String {
        let mut oriented = fabric.clone();
        // The fabric is already rotated during the baking display phase.
        // Just move centroid to origin - baked bricks must have centroid at origin.
        let centroid = oriented.centroid();
        oriented.apply_translation(-centroid);

        // Get face center joints to exclude them
        let face_joints: Vec<JointKey> = oriented
            .faces
            .values()
            .map(|face| face.middle_joint(&oriented))
            .collect();

        // Build mapping from fabric joint key to baked joint index
        let mut fabric_to_baked: HashMap<JointKey, usize> = HashMap::new();
        let mut baked_index = 0;
        for (key, _joint) in oriented.joints.iter() {
            if !face_joints.contains(&key) {
                fabric_to_baked.insert(key, baked_index);
                baked_index += 1;
            }
        }

        // Build joints using helper function format. {:.7} preserves
        // f32's ~7 significant digits — together with the
        // `symmetrize_brick_3fold` projection above, this lets the baked
        // literals carry symmetric positions to ε.
        let joints_str: Vec<String> = oriented
            .joints
            .iter()
            .filter(|(key, _)| !face_joints.contains(key))
            .map(|(_, joint)| {
                let loc = joint.location;
                format!(
                    "            joint({:.7}, {:.7}, {:.7}),",
                    loc.x, loc.y, loc.z
                )
            })
            .collect();

        // Build pushes and pulls using helper function format
        let mut pushes: Vec<String> = Vec::new();
        let mut pulls: Vec<String> = Vec::new();

        for interval in oriented.interval_values() {
            if interval.role == Role::FaceRadial {
                continue;
            }
            let alpha = fabric_to_baked.get(&interval.alpha_key);
            let omega = fabric_to_baked.get(&interval.omega_key);
            if let (Some(&a), Some(&o)) = (alpha, omega) {
                if interval.role == Role::Pushing {
                    pushes.push(format!(
                        "            push({}, {}, {:.7}),",
                        a, o, interval.strain
                    ));
                } else {
                    pulls.push(format!(
                        "            pull({}, {}, {:.7}),",
                        a, o, interval.strain
                    ));
                }
            }
        }

        // Combine intervals
        let mut intervals: Vec<String> = pushes;
        intervals.extend(pulls);

        format!(
            "        scale: {:.7},
        joints: vec![
{}
        ],
        intervals: vec![
{}
        ],",
            scale,
            joints_str.join("\n"),
            intervals.join("\n"),
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn export_brick(&self, brick_name: BrickName, baked_code: &str) {
        crate::build::brick_exporter::export(brick_name, baked_code);
    }

    #[cfg(target_arch = "wasm32")]
    fn export_brick(&self, _brick_name: BrickName, _baked_code: &str) {}
}

// ─────────────────────────────────────────────────────────────────────────────
// Bake helpers (free functions, shared between Oven::iterate and the
// headless `bake_brick` startup-regen path)
// ─────────────────────────────────────────────────────────────────────────────

/// Multiply every push and pull rest length by `scale`. Used to find the
/// scale at which the brick's average face strain matches
/// `BakedBrick::TARGET_FACE_STRAIN`.
fn scale_prototype(proto: &BrickPrototype, scale: f32) -> BrickPrototype {
    let mut scaled = proto.clone();
    for push in &mut scaled.pushes {
        push.ideal *= scale;
    }
    for pull in &mut scaled.pulls {
        pull.ideal *= scale;
    }
    scaled
}

/// Mean face strain across all faces — the bisection target.
fn average_face_strain(fabric: &Fabric) -> f32 {
    let strain_sum: f32 = fabric.faces.values().map(|face| face.strain(fabric)).sum();
    strain_sum / fabric.faces.len() as f32
}

/// Pick the next scale given the current strain. Bisects once both
/// bounds are known; otherwise damps proportionally to find the missing
/// bound. Identical logic to what Oven::compute_new_scale used.
fn next_scale(tuning: &mut TuningState, current_strain: f32) -> f32 {
    let target = BakedBrick::TARGET_FACE_STRAIN;
    if current_strain < target {
        tuning.low_scale = Some(tuning.scale);
    } else {
        tuning.high_scale = Some(tuning.scale);
    }
    if let (Some(low), Some(high)) = (tuning.low_scale, tuning.high_scale) {
        return (low + high) / 2.0;
    }
    let ratio = (target / current_strain.max(0.001)).clamp(0.5, 2.0);
    let damped = 1.0 + 0.5 * (ratio - 1.0);
    (tuning.scale * damped).clamp(0.1, 10.0)
}

/// Run a single bake pass (first physics burst → reorientation →
/// second physics burst with convergence-stop). Reused by both the live
/// UI bake and the headless startup-regen bake.
fn run_bake_pass(fabric: &mut Fabric, proto: &BrickPrototype) {
    // First physics burst — let the brick approach its rough shape from
    // the prototype's slack intervals.
    while fabric.age.as_duration() < REORIENT_DURATION {
        fabric.iterate(&BAKING);
    }
    // Reorientation: centre, align "down" to world -Y, then sit the
    // bottom at y=0. Matches the Oven's interactive path so the headless
    // bake produces the same geometry.
    let centroid = fabric.centroid();
    fabric.apply_translation(-centroid);
    let rotation = fabric.down_rotation(proto.max_seed());
    fabric.apply_matrix4(rotation);
    let translation = fabric.centralize_translation(Some(0.0));
    fabric.apply_translation(translation);
    fabric.zero_velocities();

    // Second physics burst — settle to equilibrium. Stops as soon as
    // motion has died down, with a hard cap at BAKED_DURATION.
    loop {
        for _ in 0..60 {
            fabric.iterate(&BAKING);
        }
        let age = fabric.age.as_duration();
        let post_reorient = age.saturating_sub(REORIENT_DURATION);
        let settled = post_reorient >= MIN_PHYSICS_AFTER_REORIENT
            && fabric.stats.max_speed < CONVERGENCE_SPEED_M_PER_S;
        let timed_out = age >= BAKED_DURATION;
        if settled || timed_out {
            return;
        }
    }
}

/// Bake one brick headlessly — no `Radio`, no UI hooks. Repeats the
/// physics + reorientation pass with adjusted scales until the average
/// face strain matches the target tolerance, then projects onto the
/// 3-fold-symmetric manifold. Pure function of `brick_name` plus the
/// initial scale estimate (`initial_scale`).
///
/// Called from the brick-library's startup regeneration path. The live
/// Oven still uses its UI-driven `iterate()` loop, but shares all the
/// helpers above so the two paths produce identical output.
/// Convenience: bake a brick and convert the resulting fabric directly
/// to the in-memory `BakedBrick` form used by `attach_brick`. Mirrors
/// `generate_baked_code` but returns a struct rather than emitting
/// source text. Used by `baked_bricks.rs` for startup regeneration.
pub fn bake_brick_to_baked(
    brick_name: BrickName,
    initial_scale: f32,
    params: BrickParams,
) -> BakedBrick {
    let (mut fabric, scale) = bake_brick(brick_name, initial_scale);
    // Re-centre on origin — baked bricks must have centroid at origin
    // (asserted in `brick_library::get_brick`). `bake_brick` leaves the
    // fabric with its bottom at y=0 from `centralize_translation`.
    let centroid = fabric.centroid();
    fabric.apply_translation(-centroid);
    let face_middles: HashMap<JointKey, ()> = fabric
        .faces
        .values()
        .map(|f| (f.middle_joint(&fabric), ()))
        .collect();

    let mut key_to_index: HashMap<JointKey, usize> = HashMap::new();
    let mut joints: Vec<BakedJoint> = Vec::new();
    for (key, joint) in fabric.joints.iter() {
        if face_middles.contains_key(&key) {
            continue;
        }
        key_to_index.insert(key, joints.len());
        joints.push(BakedJoint { location: joint.location });
    }

    let mut intervals: Vec<BakedInterval> = Vec::new();
    for interval in fabric.interval_values() {
        if interval.role == Role::FaceRadial {
            continue;
        }
        let (Some(&a), Some(&o)) = (
            key_to_index.get(&interval.alpha_key),
            key_to_index.get(&interval.omega_key),
        ) else {
            continue;
        };
        let material_name = if interval.role == Role::Pushing {
            "push".to_string()
        } else {
            "pull".to_string()
        };
        intervals.push(BakedInterval {
            alpha_index: a,
            omega_index: o,
            strain: interval.strain,
            material_name,
        });
    }

    let proto = brick_library::get_prototype(brick_name);
    let faces = proto.derive_baked_faces(brick_name.face_scaling());
    BakedBrick { params, scale, joints, intervals, faces }
}

pub fn bake_brick(brick_name: BrickName, initial_scale: f32) -> (Fabric, f32) {
    let proto = brick_library::get_prototype(brick_name);
    let mut tuning = TuningState::new(initial_scale);
    let face_scaling = brick_name.face_scaling();

    loop {
        let scaled = scale_prototype(&proto, tuning.scale);
        let mut fabric = scaled.to_fabric(face_scaling);
        run_bake_pass(&mut fabric, &proto);

        let strain = average_face_strain(&fabric);
        let error = (strain - BakedBrick::TARGET_FACE_STRAIN).abs();
        if error <= STRAIN_TOLERANCE {
            symmetrize_brick_3fold(&mut fabric, brick_name);
            return (fabric, tuning.scale);
        }
        tuning.scale = next_scale(&mut tuning, strain);
        tuning.iteration += 1;
        // Safety: if bisection isn't converging in a reasonable number
        // of rounds, give up and return what we have (symmetrised).
        if tuning.iteration > 20 {
            symmetrize_brick_3fold(&mut fabric, brick_name);
            return (fabric, tuning.scale);
        }
    }
}

/// Snap a brick's structural joint positions (and the matching push/pull
/// strains) onto the 3-fold-symmetric manifold. Orbit-averages each group
/// of three rotational siblings, so the resulting baked literals are
/// symmetric to within f32 ε (~1e-7 m) rather than the ε·√N physics-drift
/// floor (~4–25 μm).
///
/// No-op for bricks whose prototype doesn't declare a 3-fold cyclic axis
/// at `THREEFOLD_ROLE` (e.g. `TorqueSymmetrical`, which is only 2-fold
/// symmetric).
pub fn symmetrize_brick_3fold(fabric: &mut Fabric, brick_name: BrickName) {
    let proto = brick_library::get_prototype(brick_name);
    // Only act on bricks that declare 3-fold cyclic symmetry under
    // THREEFOLD_ROLE. `cyclic_axes_for` returns the axis order; len()==3
    // means a 3-fold rotation about the body diagonal.
    let Some(axes) = proto.cyclic_axes_for(THREEFOLD_ROLE) else {
        return;
    };
    if axes.len() != 3 {
        return;
    }

    // 1. Canonicalise: rotate brick so its 3-fold axis aligns with world
    //    +Y, computed fresh from current face normals so this works
    //    regardless of any rotation the bake has already applied.
    let to_canonical = fabric.down_rotation(THREEFOLD_ROLE);
    let from_canonical = to_canonical.inverse();

    // 2. Identify structural joints (push endpoints; face midpoints are
    //    not written to baked literals so we don't touch them).
    let face_middles: HashSet<JointKey> = fabric
        .faces
        .values()
        .map(|f| f.middle_joint(fabric))
        .collect();
    let structural: Vec<JointKey> = fabric
        .joints
        .keys()
        .filter(|k| !face_middles.contains(k))
        .collect();
    if structural.is_empty() {
        return;
    }

    // 3. Bring positions into canonical frame, centred on their centroid
    //    so the 120°-Y rotation acts about the brick centre.
    let positions: HashMap<JointKey, Vec3> = structural
        .iter()
        .map(|&k| (k, to_canonical.transform_point3(fabric.joints[k].location)))
        .collect();
    let centre: Vec3 = positions.values().copied().sum::<Vec3>()
        / positions.len() as f32;
    let centred: HashMap<JointKey, Vec3> = positions
        .iter()
        .map(|(k, p)| (*k, *p - centre))
        .collect();

    // 4. For each joint find the joint nearest to its 120°-Y image.
    //    A clean 3-cycle (k → k_b → k_c → k) is a 3-orbit; we average
    //    the three positions in a common frame, then write back the
    //    symmetric replacements. Joints that don't form clean cycles
    //    (axis singletons, malformed) keep their canonical positions.
    let rotation = Quat::from_axis_angle(Vec3::Y, std::f32::consts::TAU / 3.0);
    let rot_inv = rotation.inverse();

    let mut nearest: HashMap<JointKey, JointKey> = HashMap::new();
    for (&k, &p) in &centred {
        let target = rotation * p;
        let best = centred
            .iter()
            .map(|(k2, p2)| (*k2, (target - *p2).length_squared()))
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .expect("centred non-empty")
            .0;
        nearest.insert(k, best);
    }

    let mut sym_positions: HashMap<JointKey, Vec3> = HashMap::new();
    let mut joint_orbit_id: HashMap<JointKey, usize> = HashMap::new();
    let mut seen: HashSet<JointKey> = HashSet::new();
    let mut next_orbit_id: usize = 0;
    for &k in centred.keys() {
        if seen.contains(&k) {
            continue;
        }
        let k_b = nearest[&k];
        let k_c = nearest[&k_b];
        // Must form a closed 3-cycle. If not (e.g. axis singleton), leave
        // the joint where physics put it.
        if nearest[&k_c] != k || k_b == k || k_c == k {
            seen.insert(k);
            sym_positions.insert(k, centred[&k]);
            continue;
        }
        // Orbit mean expressed in A's frame: average of p_a, R^-1·p_b,
        // R^-2·p_c. Then rotate to get B's and C's symmetric positions.
        let p_a = centred[&k];
        let p_b = centred[&k_b];
        let p_c = centred[&k_c];
        let mean_a = (p_a + rot_inv * p_b + rot_inv * rot_inv * p_c) / 3.0;
        sym_positions.insert(k, mean_a);
        sym_positions.insert(k_b, rotation * mean_a);
        sym_positions.insert(k_c, rotation * rotation * mean_a);
        joint_orbit_id.insert(k, next_orbit_id);
        joint_orbit_id.insert(k_b, next_orbit_id);
        joint_orbit_id.insert(k_c, next_orbit_id);
        next_orbit_id += 1;
        seen.insert(k);
        seen.insert(k_b);
        seen.insert(k_c);
    }

    // 5. Write symmetrised positions back to fabric, undoing centring +
    //    canonicalisation.
    for (k, p) in &sym_positions {
        let world_p = from_canonical.transform_point3(*p + centre);
        fabric.joints[*k].location = world_p;
    }

    // 6. Average strain within each interval orbit. Two intervals are in
    //    the same orbit when their endpoints sit in the same pair of
    //    joint orbits (using the orbit ids we just assigned).
    let mut interval_orbits: HashMap<(usize, usize), Vec<IntervalKey>> = HashMap::new();
    for (key, interval) in fabric.intervals.iter() {
        if interval.role == Role::FaceRadial {
            continue;
        }
        let (Some(&a_orbit), Some(&o_orbit)) = (
            joint_orbit_id.get(&interval.alpha_key),
            joint_orbit_id.get(&interval.omega_key),
        ) else {
            continue;
        };
        // Canonicalise the orbit pair so (A, B) and (B, A) merge.
        let key_pair = if a_orbit <= o_orbit {
            (a_orbit, o_orbit)
        } else {
            (o_orbit, a_orbit)
        };
        interval_orbits.entry(key_pair).or_default().push(key);
    }
    for (_, members) in interval_orbits {
        if members.len() < 2 {
            continue;
        }
        let mean_strain: f32 = members
            .iter()
            .map(|k| fabric.intervals[*k].strain)
            .sum::<f32>()
            / members.len() as f32;
        for k in members {
            fabric.intervals[k].strain = mean_strain;
        }
    }
}
