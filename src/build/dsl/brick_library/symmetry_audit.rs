//! Diagnostic probe for baked-brick symmetry. Mirrors the `Oven`'s bake
//! loop (`src/build/oven.rs`) without its UI dependencies (no `Radio`, no
//! `CrucibleContext`) and logs a *symmetry deviation* at every stage of the
//! bake, so we can pinpoint which step in the loop introduces the
//! 1e-5-scale asymmetry visible in the current baked literals.
//!
//! This module is `#[cfg(test)]`-gated — pure diagnosis, no production
//! code touched. Once we know where the asymmetry creeps in, a follow-up
//! commit will introduce the fix in `oven.rs` itself.
//!
//! Run with:
//!
//! ```text
//! cargo test --release --lib symmetry_audit -- --nocapture
//! ```

#![cfg(test)]

use std::time::Duration;

use glam::{Quat, Vec3};

use crate::build::dsl::brick::BrickPrototype;
use crate::build::dsl::brick_dsl::{BrickName, BrickRole};
use crate::build::dsl::brick_library;
use crate::fabric::physics::presets::BAKING;
use crate::fabric::{Fabric, JointKey};
use std::collections::HashSet;

/// The orientation role whose `down_rotation` aligns the brick's 3-fold
/// body-diagonal axis with world +Y. `Seed(1)` is the convention across
/// the Omni- and Single-shaped bricks. This is *not* the role the Oven
/// uses to physically reorient (`max_seed()` = `Seed(4)` for Omni); for
/// the diagnostic we want the 3-fold-aligned frame.
const THREEFOLD_ROLE: BrickRole = BrickRole::Seed(1);

// Mirrors the constants in oven.rs.
const BAKED_DURATION: Duration = Duration::from_secs(2);
const REORIENT_DURATION: Duration = Duration::from_millis(500);

/// One stage of the bake loop, with the symmetry-deviation measurement
/// and max joint speed at that stage. Max speed lets us see when the
/// system stops moving — the natural early-stop criterion.
#[derive(Debug)]
struct Stage {
    label: &'static str,
    description: &'static str,
    deviation_m: f32,
    max_speed_m_per_s: f32,
}

/// Run the bake loop for one brick (single pass — no strain-bisection
/// restarts — using the brick's cached scale) and return the per-stage
/// deviation trace. The trace mirrors the suspected probe points in
/// `oven.rs`:
///
///   A. After `prototype.to_fabric()` (initial setup).
///   B. End of the first physics burst, immediately before reorientation.
///   C. Immediately after the reorientation rotation.
///   D. Immediately after `zero_velocities()`.
///   E. End of the second physics burst (final converged state).
fn trace_bake(brick_name: BrickName) -> Vec<Stage> {
    let proto = brick_library::get_prototype(brick_name);
    let scale = brick_library::get_scale(brick_name);
    let scaled = scale_prototype(&proto, scale);
    let mut fabric = scaled.to_fabric(brick_name.face_scaling());

    let mut stages: Vec<Stage> = Vec::new();
    let record = |fabric: &Fabric, proto: &BrickPrototype, stages: &mut Vec<Stage>,
                  label, description| {
        let dev = symmetry_deviation(fabric, proto);
        // Max joint speed = how much the system is still moving. Once
        // this is sub-mm/s, the brick has effectively stopped settling.
        let max_speed = fabric.joints.values()
            .map(|j| j.velocity.length())
            .fold(0.0_f32, f32::max);
        stages.push(Stage {
            label,
            description,
            deviation_m: dev,
            max_speed_m_per_s: max_speed,
        });
    };

    record(&fabric, &proto, &mut stages,
        "A", "after to_fabric (initial geometry)");

    // First physics burst: until age >= REORIENT_DURATION.
    while fabric.age.as_duration() < REORIENT_DURATION {
        fabric.iterate(&BAKING);
    }
    record(&fabric, &proto, &mut stages,
        "B", "after first physics burst (before reorientation)");

    // Reorientation — mirror oven.rs:192-205, broken into sub-steps so we
    // can probe at each one.
    let centroid = fabric.centroid();
    fabric.apply_translation(-centroid);
    let rotation = fabric.down_rotation(proto.max_seed());
    fabric.apply_matrix4(rotation);
    let translation = fabric.centralize_translation(Some(0.0));
    fabric.apply_translation(translation);
    record(&fabric, &proto, &mut stages,
        "C", "after reorientation rotation (before zero_velocities)");

    fabric.zero_velocities();
    record(&fabric, &proto, &mut stages,
        "D", "after zero_velocities");

    // Second physics burst, sampled per-iteration so we can see the rate
    // of asymmetry growth. Linear in N → one asymmetric op per step.
    // √N → random-walk float drift (no single culprit). Sudden jumps →
    // discrete event.
    let mut sample_points: Vec<usize> = vec![1, 10, 100, 1000, 10000];
    let remaining = BAKED_DURATION.saturating_sub(fabric.age.as_duration());
    let total_iterations = (remaining.as_secs_f32() * crate::Age::iterations_per_second())
        .round() as usize;
    sample_points.retain(|&n| n < total_iterations);
    sample_points.push(total_iterations);

    let labels: &[&str] = &["D+1", "D+10", "D+100", "D+1k", "D+10k", "E"];
    let mut iterations_done = 0;
    for (idx, &target) in sample_points.iter().enumerate() {
        while iterations_done < target {
            fabric.iterate(&BAKING);
            iterations_done += 1;
        }
        let label = labels.get(idx).copied().unwrap_or("?");
        let desc: String = if idx + 1 == sample_points.len() {
            format!("after second physics burst, {iterations_done} iters (final converged)")
        } else {
            format!("after {iterations_done} physics iterations")
        };
        let dev = symmetry_deviation(&fabric, &proto);
        let max_speed = fabric.joints.values()
            .map(|j| j.velocity.length())
            .fold(0.0_f32, f32::max);
        stages.push(Stage {
            label,
            description: Box::leak(desc.into_boxed_str()),
            deviation_m: dev,
            max_speed_m_per_s: max_speed,
        });
    }

    // Stage F: apply the Oven's orbit-averaging symmetrise step. Deviation
    // should drop to the canonicalisation noise floor (~1e-7 m).
    crate::build::oven::symmetrize_brick_3fold(&mut fabric, brick_name);
    record(&fabric, &proto, &mut stages,
        "F", "after symmetrize_brick_3fold (snapped to manifold)");

    stages
}

/// Print a per-stage deviation trace to stderr (visible under
/// `cargo test -- --nocapture`).
fn print_trace(brick_name: BrickName, stages: &[Stage]) {
    eprintln!("\nBake-trace symmetry deviation for {brick_name}");
    eprintln!("(deviation = max nearest-neighbour distance after applying the brick's");
    eprintln!(" 120° cyclic-axis rotation; 0 = perfectly symmetric)\n");
    eprintln!("    Stage  Deviation        MaxSpeed         Description");
    for stage in stages {
        eprintln!("    {:>5}  {:>11.2e} m   {:>9.2e} m/s   {}",
            stage.label, stage.deviation_m, stage.max_speed_m_per_s, stage.description);
    }

    // Highlight the biggest jump between consecutive stages — the most
    // likely suspect for where asymmetry enters.
    let biggest_jump = stages.windows(2)
        .enumerate()
        .map(|(i, w)| (i, w[1].deviation_m - w[0].deviation_m))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    if let Some((i, jump)) = biggest_jump {
        if jump > 1e-7 {
            eprintln!("\n  ⚠ Biggest jump: {} → {}  (+{:.2e} m)",
                stages[i].label, stages[i + 1].label, jump);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Symmetry deviation
// ─────────────────────────────────────────────────────────────────────────────

/// Measure how far the structural joints of `fabric` deviate from being
/// invariant under the brick's natural 120° rotation. Result is the worst
/// per-joint nearest-neighbour distance after applying the rotation; 0
/// means perfect symmetry.
///
/// We canonicalise via `down_rotation(THREEFOLD_ROLE)` first — that's the
/// rotation that aligns the brick's 3-fold body-diagonal axis with world
/// +Y, computed fresh from current face normals so it works at any bake
/// stage regardless of whatever rotation the Oven has applied.
fn symmetry_deviation(fabric: &Fabric, _proto: &BrickPrototype) -> f32 {
    let canonicalise = fabric.down_rotation(THREEFOLD_ROLE);
    let face_middles: HashSet<JointKey> = fabric
        .faces
        .values()
        .map(|f| f.middle_joint(fabric))
        .collect();
    let structural: Vec<Vec3> = fabric
        .joints
        .iter()
        .filter(|(k, _)| !face_middles.contains(k))
        .map(|(_, j)| j.location)
        .collect();
    // Centre the structural joints so the rotation acts about the brick
    // centroid, not about world origin (the Oven's `centralize_translation`
    // shifts the brick so its bottom sits at y=0 — non-origin).
    let centre: Vec3 = structural.iter().copied().sum::<Vec3>() / structural.len() as f32;
    let joints: Vec<Vec3> = structural
        .iter()
        .map(|p| canonicalise.transform_point3(*p - centre))
        .collect();

    let rotation = Quat::from_axis_angle(Vec3::Y, std::f32::consts::TAU / 3.0);
    let mut max_dev = 0.0_f32;
    for p in &joints {
        let target = rotation * *p;
        let nearest = joints
            .iter()
            .map(|q| (target - *q).length())
            .fold(f32::INFINITY, f32::min);
        if nearest > max_dev {
            max_dev = nearest;
        }
    }
    max_dev
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers shared with oven.rs (duplicated here to keep the audit isolated)
// ─────────────────────────────────────────────────────────────────────────────

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

// ─────────────────────────────────────────────────────────────────────────────
// Tests (run with --nocapture)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn trace_bake_omni_symmetrical() {
    let stages = trace_bake(BrickName::OmniSymmetrical);
    print_trace(BrickName::OmniSymmetrical, &stages);
}

#[test]
fn trace_bake_single_twist_left() {
    let stages = trace_bake(BrickName::SingleTwistLeft);
    print_trace(BrickName::SingleTwistLeft, &stages);
}

/// Bake using a convergence-based stop instead of fixed duration. Run
/// physics post-reorientation until `max_speed` drops below threshold
/// (with a min-time floor to let the system get going first, and a
/// max-time safety cap). Report how many fabric-time milliseconds were
/// needed for each brick — should be much less than the current 2000 ms.
#[test]
fn trace_bake_with_convergence_stop() {
    use std::time::Instant;

    // Tuneable: stop the second physics burst when the fastest joint is
    // below this speed (m/s). 1e-2 ≈ 1 cm/s — well below any practical
    // joint motion at the scale of metre-sized struts, but loose enough
    // that bricks reach it well before the existing 2 s safety duration.
    const STOP_SPEED: f32 = 1.0e-2;
    // Floor: physics for at least this long after reorientation, so the
    // system actually picks up speed before we start measuring.
    const MIN_PHYSICS_AFTER_REORIENT: Duration = Duration::from_millis(150);
    // Cap: never bake longer than this (matches current Oven).
    const SAFETY_CAP: Duration = Duration::from_secs(2);

    fn bake(brick_name: BrickName) -> (Duration, usize, f32, f32) {
        let proto = brick_library::get_prototype(brick_name);
        let scale = brick_library::get_scale(brick_name);
        let scaled = scale_prototype(&proto, scale);
        let mut fabric = scaled.to_fabric(brick_name.face_scaling());

        let wall_start = Instant::now();
        let mut iters = 0;

        while fabric.age.as_duration() < REORIENT_DURATION {
            fabric.iterate(&BAKING);
            iters += 1;
        }
        let centroid = fabric.centroid();
        fabric.apply_translation(-centroid);
        let rotation = fabric.down_rotation(proto.max_seed());
        fabric.apply_matrix4(rotation);
        let translation = fabric.centralize_translation(Some(0.0));
        fabric.apply_translation(translation);
        fabric.zero_velocities();

        let reorient_done = fabric.age.as_duration();
        loop {
            fabric.iterate(&BAKING);
            iters += 1;
            let elapsed = fabric.age.as_duration();
            let elapsed_post_reorient = elapsed.saturating_sub(reorient_done);
            if elapsed >= SAFETY_CAP {
                break;
            }
            if elapsed_post_reorient >= MIN_PHYSICS_AFTER_REORIENT
                && fabric.stats.max_speed < STOP_SPEED
            {
                break;
            }
        }

        let wall = wall_start.elapsed();
        let fabric_time = fabric.age.as_duration().as_secs_f32() * 1000.0;
        let final_speed = fabric.stats.max_speed;
        let _ = iters;

        crate::build::oven::symmetrize_brick_3fold(&mut fabric, brick_name);
        let final_dev = symmetry_deviation(&fabric, &proto);
        eprintln!(
            "  {:<22} fabric={:>5.0}ms  wall={:>5.1}ms  iters={:>6}  \
             final_speed={:.2e} m/s  final_dev={:.2e} m (post-symmetrize)",
            format!("{}", brick_name),
            fabric_time,
            wall.as_secs_f64() * 1000.0,
            iters,
            final_speed,
            final_dev,
        );
        (wall, iters, final_speed, final_dev)
    }

    eprintln!("\nConvergence-based bake (stop when max_speed < {:.0e} m/s, cap {} s)",
        STOP_SPEED, SAFETY_CAP.as_secs());
    for brick_name in [BrickName::OmniSymmetrical, BrickName::SingleTwistLeft] {
        bake(brick_name);
    }
    eprintln!("\nFor reference, current fixed-duration bake = 2000 ms fabric time per brick.");
}

// ─────────────────────────────────────────────────────────────────────────────
// OpenClaw-level asymmetry probe
// ─────────────────────────────────────────────────────────────────────────────
// Stand-alone diagnostic: measure how far each off-axis joint in OpenClaw
// (at end-of-Building, BEFORE apply_threefold_symmetry runs) is from its
// rotational image under 120°-about-Y. Aim is to compare the magnitude of
// OpenClaw-level asymmetry against the brick-level asymmetry diagnosed
// above, to see whether the brick asymmetry alone seeds what the paste
// step corrects, or whether OpenClaw's own build-time physics adds more.

#[test]
fn trace_openclaw_joint_asymmetry_pre_paste() {
    use crate::build::dsl::fabric_library::{self, FabricName};
    use crate::build::dsl::fabric_plan_executor::{ExecutorStage, FabricPlanExecutor};
    use std::collections::BTreeMap;

    let plan = fabric_library::get_fabric_plan(FabricName::OpenClaw);
    let mut executor = FabricPlanExecutor::new(plan);
    while *executor.stage() == ExecutorStage::Building {
        let _ = executor.iterate();
    }
    let fabric = &executor.fabric;

    // Group joints by (brick, position) ignoring leg letter. Each group
    // should have exactly 3 members (one per leg) at rotational-image
    // positions.
    let rot = Quat::from_axis_angle(Vec3::Y, std::f32::consts::TAU / 3.0);
    let mut groups: BTreeMap<String, Vec<(char, Vec3)>> = BTreeMap::new();
    for (key, joint) in fabric.joints.iter() {
        let label = fabric.joint_label(key);
        let bytes = label.as_bytes();
        if bytes.len() < 2 || !matches!(bytes[0], b'A' | b'B' | b'C') {
            continue;
        }
        let leg = bytes[0] as char;
        let suffix = std::str::from_utf8(&bytes[1..]).unwrap_or("").to_string();
        groups.entry(suffix).or_default().push((leg, joint.location));
    }

    // For each (brick, position) triple, find the rotational mismatch.
    let mut per_orbit: Vec<(String, f32)> = Vec::new();
    for (suffix, members) in &groups {
        if members.len() != 3 {
            continue;
        }
        let (_, p_a) = members.iter().find(|(l, _)| *l == 'A').copied().unwrap();
        let (_, p_b) = members.iter().find(|(l, _)| *l == 'B').copied().unwrap();
        let (_, p_c) = members.iter().find(|(l, _)| *l == 'C').copied().unwrap();
        let centre = (p_a + p_b + p_c) / 3.0;
        // Rotate (A - centre) by 120° → should equal (B - centre).
        // Then (B - centre) rotated → C. Report worst residual.
        let from_a = rot * (p_a - centre);
        let from_b = rot * (p_b - centre);
        let d_ab = (from_a - (p_b - centre)).length();
        let d_bc = (from_b - (p_c - centre)).length();
        let worst = d_ab.max(d_bc);
        per_orbit.push((suffix.clone(), worst));
    }

    per_orbit.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let worst = per_orbit.first().map(|(_, d)| *d).unwrap_or(0.0);
    let mean = per_orbit.iter().map(|(_, d)| *d).sum::<f32>()
        / per_orbit.len().max(1) as f32;
    let median = per_orbit.get(per_orbit.len() / 2).map(|(_, d)| *d).unwrap_or(0.0);

    eprintln!("\nOpenClaw joint-position asymmetry (end of Building, pre-paste)");
    eprintln!("  Triples measured: {}", per_orbit.len());
    eprintln!("  Worst residual : {:.2e} m ({})", worst,
        per_orbit.first().map(|(s, _)| s.as_str()).unwrap_or(""));
    eprintln!("  Mean residual  : {:.2e} m", mean);
    eprintln!("  Median residual: {:.2e} m", median);
    eprintln!("\n  Top 5 worst orbits:");
    for (suffix, d) in per_orbit.iter().take(5) {
        eprintln!("    A{0}/B{0}/C{0}  →  {1:.2e} m", suffix, d);
    }
    eprintln!("\nCompare to brick-bake final asymmetry: ~4e-6 m (OmniSymmetrical)");
    eprintln!("and ~25e-6 m (SingleTwistLeft).");
}
