//! Open Claw — symmetry enforcement, CSV export, and the test that drives both.
//!
//! Open Claw is the only fabric whose construction targets a physical build,
//! so the engineering CSV and its 3-fold-symmetry guarantees live here rather
//! than in the generic fabric layer. The rest of the codebase remains
//! oblivious to leg-letters, seed-joint shapes, and CSV output.
//!
//! What this module provides:
//!   - Rotation helpers for the joint-label scheme established by
//!     [`crate::build::dsl::labelling::OmniSeedLabeller`] — A→B→C leg
//!     cycling for path joints, the analogous cycle for seed joints.
//!   - [`apply_threefold_symmetry`] — copies one representative's slot
//!     assignments to its rotational partners after the generic per-push
//!     algorithm has run, so floating-point ε doesn't flip cables between
//!     adjacent slots within a triple.
//!   - A private `write_csv` that emits the slack-moment CSV.
//!   - [`test_open_claw_threefold_symmetry`] — exports the CSV (headless)
//!     and asserts every rotational triple agrees on length, slot, and bend.
//!   - [`test_open_claw_cable_triples`] — companion test focused on cable
//!     lengths alone (kept here because it's the same symmetry property
//!     viewed from a different angle).

use glam::{Mat3, Vec3};
use std::collections::BTreeMap;
use std::f32::consts::FRAC_PI_2;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use crate::fabric::attachment::{TabBend, PullConnection, ATTACHMENT_POINTS};
use crate::fabric::interval::Role;
use crate::fabric::{Fabric, FabricDimensions, IntervalEnd, IntervalKey, JointKey};
use crate::units::{Unit, MM_PER_METER};

// ─────────────────────────────────────────────────────────────────────────────
// Rotation helpers — Open Claw label scheme
// ─────────────────────────────────────────────────────────────────────────────

/// Rotate a joint label by 120° about the central axis (A→B→C).
///
/// Three label shapes participate:
///   - Path joints (start with A/B/C):   `AX4YZ1` → `BX4YZ1`
///   - Seed joints (`[BT][AO][ABC]`):    `BAA`    → `BAB`
///   - Everything else (apex `YZ0`, off-axis paths starting with D/E/…): unchanged.
fn rotate_label_once(label: &str) -> String {
    let bytes = label.as_bytes();
    let n = bytes.len();
    if n == 3
        && matches!(bytes[0], b'B' | b'T')
        && matches!(bytes[1], b'A' | b'O')
        && matches!(bytes[2], b'A' | b'B' | b'C')
    {
        let next = match bytes[2] {
            b'A' => 'B',
            b'B' => 'C',
            b'C' => 'A',
            _ => unreachable!(),
        };
        return format!("{}{}{}", bytes[0] as char, bytes[1] as char, next);
    }
    if n > 0 && matches!(bytes[0], b'A' | b'B' | b'C') {
        let next = match bytes[0] {
            b'A' => 'B',
            b'B' => 'C',
            b'C' => 'A',
            _ => unreachable!(),
        };
        return format!("{}{}", next, &label[1..]);
    }
    label.to_string()
}

/// Canonical key for an unordered pair of joint labels (a push or cable):
/// the lex-min over (rotate^k(a), rotate^k(b)) for k∈{0,1,2} and both end orderings.
/// Two pairs share this key iff one is a rotational image of the other.
fn canonical_push_key(a: &str, b: &str) -> (String, String) {
    let mut a_rot = a.to_string();
    let mut b_rot = b.to_string();
    let mut best = std::cmp::min(
        (a_rot.clone(), b_rot.clone()),
        (b_rot.clone(), a_rot.clone()),
    );
    for _ in 0..2 {
        a_rot = rotate_label_once(&a_rot);
        b_rot = rotate_label_once(&b_rot);
        let pair = (a_rot.clone(), b_rot.clone());
        let swapped = (b_rot.clone(), a_rot.clone());
        if pair < best {
            best = pair;
        }
        if swapped < best {
            best = swapped;
        }
    }
    best
}

// ─────────────────────────────────────────────────────────────────────────────
// Threefold-symmetry enforcement
// ─────────────────────────────────────────────────────────────────────────────

/// Force slot assignments to be perfectly threefold-symmetric across leg-letter
/// rotations of joint labels.
///
/// Call after `update_all_attachment_connections`. Groups push intervals by
/// rotation-canonical (alpha_label, omega_label); the first member of each
/// group keeps the slot assignment the generic algorithm produced, and the
/// other members copy it via label rotation. Cables are matched by rotating
/// each rep cable's far-end joint label by the rotation count `k` that maps
/// rep → member, then locating the corresponding cable in the member.
///
/// Using the rotation count `k` (rather than a canonical form of the far-end
/// label) matters: at one push end, multiple cables can share a canonical
/// far-end — e.g., seed joint BAA has cables to both BAB and BAC, both of
/// which canonicalise to "BAA".
pub fn apply_threefold_symmetry(fabric: &mut Fabric) {
    let push_keys: Vec<IntervalKey> = fabric
        .intervals
        .iter()
        .filter_map(|(key, interval)| interval.has_role(Role::Pushing).then_some(key))
        .collect();

    let mut groups: BTreeMap<(String, String), Vec<IntervalKey>> = BTreeMap::new();
    for &key in &push_keys {
        let iv = &fabric.intervals[key];
        let a = fabric.joint_label(iv.alpha_key);
        let b = fabric.joint_label(iv.omega_key);
        groups
            .entry(canonical_push_key(&a, &b))
            .or_default()
            .push(key);
    }

    for (_canon, members) in groups {
        let Some(&rep) = members.first() else { continue };
        for &member in &members[1..] {
            copy_symmetric_attachments(fabric, rep, member);
        }
    }
}

/// Copy `rep`'s slot assignment to `member` via label rotation. Silently
/// no-ops if the labels don't form a clean rotation match (which shouldn't
/// happen for an OpenClaw fabric).
fn copy_symmetric_attachments(
    fabric: &mut Fabric,
    rep_key: IntervalKey,
    member_key: IntervalKey,
) {
    let (rep_a_key, rep_b_key, mem_a_key, mem_b_key) = {
        let Some(rep) = fabric.intervals.get(rep_key) else { return };
        let Some(member) = fabric.intervals.get(member_key) else { return };
        (rep.alpha_key, rep.omega_key, member.alpha_key, member.omega_key)
    };

    let rep_a_label = fabric.joint_label(rep_a_key);
    let rep_b_label = fabric.joint_label(rep_b_key);
    let mem_a_label = fabric.joint_label(mem_a_key);
    let mem_b_label = fabric.joint_label(mem_b_key);

    // Find k ∈ {0,1,2} and swap so that rotate^k(rep_a, rep_b) matches
    // (mem_a, mem_b) directly or with ends swapped.
    let mut a = rep_a_label.clone();
    let mut b = rep_b_label.clone();
    let mut k_swap: Option<(usize, bool)> = None;
    for k in 0..3 {
        if a == mem_a_label && b == mem_b_label {
            k_swap = Some((k, false));
            break;
        }
        if a == mem_b_label && b == mem_a_label {
            k_swap = Some((k, true));
            break;
        }
        a = rotate_label_once(&a);
        b = rotate_label_once(&b);
    }
    let Some((k, swap)) = k_swap else { return };

    let label_to_key: BTreeMap<String, JointKey> = fabric
        .joints
        .iter()
        .map(|(jk, _)| (fabric.joint_label(jk), jk))
        .collect();

    let rep_alpha_src = read_end_assignments(fabric, rep_key, IntervalEnd::Alpha, rep_a_key);
    let rep_omega_src = read_end_assignments(fabric, rep_key, IntervalEnd::Omega, rep_b_key);

    let (alpha_src, omega_src) = if swap {
        (rep_omega_src, rep_alpha_src)
    } else {
        (rep_alpha_src, rep_omega_src)
    };

    let Some(alpha_pulls) = translate_assignments(fabric, &alpha_src, k, mem_a_key, &label_to_key)
    else { return };
    let Some(omega_pulls) = translate_assignments(fabric, &omega_src, k, mem_b_key, &label_to_key)
    else { return };

    let Some(push) = fabric.intervals.get_mut(member_key) else { return };
    let Some(connections) = &mut push.connections else { return };
    connections.alpha = [None; ATTACHMENT_POINTS];
    connections.omega = [None; ATTACHMENT_POINTS];
    for (pull_key, slot) in alpha_pulls {
        if slot < ATTACHMENT_POINTS {
            connections.alpha[slot] = Some(PullConnection {
                pull_interval_key: pull_key,
                attachment_index: slot,
            });
        }
    }
    for (pull_key, slot) in omega_pulls {
        if slot < ATTACHMENT_POINTS {
            connections.omega[slot] = Some(PullConnection {
                pull_interval_key: pull_key,
                attachment_index: slot,
            });
        }
    }
}

fn read_end_assignments(
    fabric: &Fabric,
    push_key: IntervalKey,
    end: IntervalEnd,
    near_joint_key: JointKey,
) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let Some(push) = fabric.intervals.get(push_key) else { return out };
    let Some(connections) = push.connections(end) else { return out };
    for (slot, opt) in connections.iter().enumerate() {
        let Some(pc) = opt else { continue };
        let Some(pull) = fabric.intervals.get(pc.pull_interval_key) else { continue };
        let other_key = if pull.alpha_key == near_joint_key {
            pull.omega_key
        } else {
            pull.alpha_key
        };
        out.push((slot, fabric.joint_label(other_key)));
    }
    out
}

fn translate_assignments(
    fabric: &Fabric,
    src: &[(usize, String)],
    k: usize,
    member_near_key: JointKey,
    label_to_key: &BTreeMap<String, JointKey>,
) -> Option<Vec<(IntervalKey, usize)>> {
    let mut out = Vec::with_capacity(src.len());
    for (slot, src_other_label) in src {
        let mut rotated = src_other_label.clone();
        for _ in 0..k {
            rotated = rotate_label_once(&rotated);
        }
        let &target_other_key = label_to_key.get(&rotated)?;
        let cable_key = fabric.intervals.iter().find_map(|(ik, iv)| {
            if !iv.role.is_pull_like() {
                return None;
            }
            let connects = (iv.alpha_key == member_near_key
                && iv.omega_key == target_other_key)
                || (iv.alpha_key == target_other_key && iv.omega_key == member_near_key);
            connects.then_some(ik)
        })?;
        out.push((cable_key, *slot));
    }
    Some(out)
}

// ─────────────────────────────────────────────────────────────────────────────
// CSV export (engineering format — slack moment)
// ─────────────────────────────────────────────────────────────────────────────

/// Rotation from simulation space (Y-up) to CSV space (Z-up, RFEM/Rhino).
///
/// Simulation uses Y as the vertical axis; the engineer's FEA tools expect
/// Z-up. A +90° rotation about X sends sim +Y → csv +Z (vertical extent lands
/// in the CSV's Z column). Sim +Z → csv −Y keeps the transform a pure
/// right-handed rotation.
fn sim_to_csv() -> Mat3 {
    Mat3::from_rotation_x(FRAC_PI_2)
}

/// Write the slack-moment engineering CSV. Caller is responsible for having
/// run `update_all_attachment_connections`, `apply_threefold_symmetry`, and
/// `recompute_bend_magnitudes` in that order.
fn write_csv(fabric: &Fabric, filename: &str) -> io::Result<()> {
    let path = Path::new(filename);
    let mut file = File::create(path)?;

    let dimensions = &fabric.dimensions;
    let to_csv = sim_to_csv();
    let height_mm = fabric
        .joints
        .values()
        .fold(0.0f32, |h, joint| h.max(joint.location.y))
        * MM_PER_METER;

    let joints_csv: Vec<(String, Vec3)> = fabric
        .joints
        .iter()
        .map(|(k, j)| (fabric.joint_label(k), to_csv * (j.location * MM_PER_METER)))
        .collect();
    let mut sorted_by_z = joints_csv.clone();
    sorted_by_z.sort_by(|a, b| a.1.z.partial_cmp(&b.1.z).unwrap_or(std::cmp::Ordering::Equal));
    let lowest_three: Vec<&(String, Vec3)> = sorted_by_z.iter().take(3).collect();
    let highest_one: &(String, Vec3) = sorted_by_z.last().expect("fabric has at least one joint");

    let now = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();

    let phase_str = "slack";
    writeln!(
        file,
        "# {}, Phase: {}, Height: {:.1}mm, Created: {}",
        fabric.name, phase_str, height_mm, now
    )?;
    write_dimensions_comments(&mut file, &fabric.dimensions)?;
    writeln!(
        file,
        "# Orientation check (CSV coords, mm, Z-up): ground plane at Z=0, apex at Z={:.1}",
        highest_one.1.z
    )?;
    for (i, (path, pos)) in lowest_three.iter().enumerate() {
        writeln!(
            file,
            "# Lowest[{}]: joint={} X={:.1} Y={:.1} Z={:.1}",
            i + 1, path, pos.x, pos.y, pos.z
        )?;
    }
    writeln!(
        file,
        "# Highest:   joint={} X={:.1} Y={:.1} Z={:.1}",
        highest_one.0, highest_one.1.x, highest_one.1.y, highest_one.1.z
    )?;
    let bend_summary = build_bend_summary(fabric);
    file.write_all(bend_summary.as_bytes())?;
    let clearance_summary = build_clearance_summary(fabric);
    file.write_all(clearance_summary.as_bytes())?;
    writeln!(file, "Index,Role,Length(m),Strain,AlphaX,AlphaY,AlphaZ,AlphaJoint,AlphaSlot,AlphaAngle,OmegaX,OmegaY,OmegaZ,OmegaJoint,OmegaSlot,OmegaAngle")?;

    // (pull_interval_key, end, slot) -> (pull_end_pos, tab_pos, joint_key, slot, tab_bend, ideal_deg)
    let mut pull_bend_info: BTreeMap<
        (IntervalKey, IntervalEnd, usize),
        (Vec3, Vec3, JointKey, usize, TabBend, f32),
    > = BTreeMap::new();

    for (_key, push_interval) in fabric.intervals.iter() {
        if !push_interval.has_role(Role::Pushing) {
            continue;
        }

        let alpha_pos = fabric.joints[push_interval.alpha_key].location;
        let omega_pos = fabric.joints[push_interval.omega_key].location;
        let push_dir = (omega_pos - alpha_pos).normalize();

        if let Some(connections) = push_interval.connections(IntervalEnd::Alpha) {
            for (slot_idx, conn_opt) in connections.iter().enumerate() {
                if let Some(connection) = conn_opt {
                    if let Some(pull_interval) =
                        fabric.intervals.get(connection.pull_interval_key)
                    {
                        let pull_other_end =
                            if pull_interval.alpha_key == push_interval.alpha_key {
                                fabric.joints[pull_interval.omega_key].location
                            } else {
                                fabric.joints[pull_interval.alpha_key].location
                            };

                        let (tab_pos, tab_bend, pull_end_pos, ideal_deg) =
                            dimensions.tab_geometry(
                                alpha_pos,
                                -push_dir,
                                slot_idx,
                                pull_other_end,
                            );

                        let pull_end = if pull_interval.alpha_key == push_interval.alpha_key {
                            IntervalEnd::Alpha
                        } else {
                            IntervalEnd::Omega
                        };
                        pull_bend_info.insert(
                            (connection.pull_interval_key, pull_end, slot_idx + 1),
                            (
                                pull_end_pos,
                                tab_pos,
                                push_interval.alpha_key,
                                slot_idx + 1,
                                tab_bend,
                                ideal_deg,
                            ),
                        );
                    }
                }
            }
        }

        if let Some(connections) = push_interval.connections(IntervalEnd::Omega) {
            for (slot_idx, conn_opt) in connections.iter().enumerate() {
                if let Some(connection) = conn_opt {
                    if let Some(pull_interval) =
                        fabric.intervals.get(connection.pull_interval_key)
                    {
                        let pull_other_end =
                            if pull_interval.alpha_key == push_interval.omega_key {
                                fabric.joints[pull_interval.omega_key].location
                            } else {
                                fabric.joints[pull_interval.alpha_key].location
                            };

                        let (tab_pos, tab_bend, pull_end_pos, ideal_deg) =
                            dimensions.tab_geometry(
                                omega_pos,
                                push_dir,
                                slot_idx,
                                pull_other_end,
                            );

                        let pull_end = if pull_interval.alpha_key == push_interval.omega_key {
                            IntervalEnd::Alpha
                        } else {
                            IntervalEnd::Omega
                        };
                        pull_bend_info.insert(
                            (connection.pull_interval_key, pull_end, slot_idx + 1),
                            (
                                pull_end_pos,
                                tab_pos,
                                push_interval.omega_key,
                                slot_idx + 1,
                                tab_bend,
                                ideal_deg,
                            ),
                        );
                    }
                }
            }
        }
    }

    let mut interval_infos: Vec<IntervalInfo> = fabric
        .intervals
        .iter()
        .filter_map(|(key, interval)| {
            if interval.has_role(Role::Support) {
                return None;
            }
            let alpha = fabric.joints[interval.alpha_key].location;
            let omega = fabric.joints[interval.omega_key].location;
            let length = (omega - alpha).length();
            Some(IntervalInfo {
                key,
                is_push: interval.has_role(Role::Pushing),
                length,
                strain: interval.strain,
            })
        })
        .collect();

    interval_infos.sort_by(|a, b| match (a.is_push, b.is_push) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a
            .length
            .partial_cmp(&b.length)
            .unwrap_or(std::cmp::Ordering::Equal),
    });

    // Group-mean rounding for displayed lengths. For every rotational triple
    // where neither endpoint is on the central apex axis, replace each
    // member's displayed length with the group mean — this collapses sub-mm
    // float drift that would otherwise straddle a millimetre boundary and
    // appear as a 1mm spread in the CSV. Intervals touching the apex joints
    // YZ0/YZ1 are skipped: the three rotational copies converging there
    // legitimately occupy three different slots and have three different
    // lengths by physical necessity.
    let displayed_length = build_group_mean_lengths(fabric, &interval_infos);

    let mut highest_slot_per_joint: BTreeMap<JointKey, usize> = BTreeMap::new();
    for ((_, _, slot), (_, _, joint_key, _, _, _)) in &pull_bend_info {
        let entry = highest_slot_per_joint.entry(*joint_key).or_insert(0);
        if *slot > *entry {
            *entry = *slot;
        }
    }

    let mut ring_centers: BTreeMap<(JointKey, usize), Vec3> = BTreeMap::new();

    for (_key, push_interval) in fabric.intervals.iter() {
        if !push_interval.has_role(Role::Pushing) {
            continue;
        }
        let alpha_pos = fabric.joints[push_interval.alpha_key].location;
        let omega_pos = fabric.joints[push_interval.omega_key].location;
        let push_dir = (omega_pos - alpha_pos).normalize();

        for slot_0 in 0..3 {
            let slot_1 = slot_0 + 1;
            let alpha_ring = dimensions.ring_center(alpha_pos, -push_dir, slot_0);
            ring_centers.insert((push_interval.alpha_key, slot_1), alpha_ring);
            let omega_ring = dimensions.ring_center(omega_pos, push_dir, slot_0);
            ring_centers.insert((push_interval.omega_key, slot_1), omega_ring);
        }
    }

    let mut current_index = 0usize;
    for info in interval_infos.iter() {
        current_index += 1;
        let interval = fabric.intervals.get(info.key).unwrap();
        let role_str = if info.is_push { "push" } else { "pull" };

        if info.is_push {
            let alpha_joint = &fabric.joints[interval.alpha_key];
            let omega_joint = &fabric.joints[interval.omega_key];
            let alpha = to_csv * (alpha_joint.location * MM_PER_METER);
            let omega = to_csv * (omega_joint.location * MM_PER_METER);
            let alpha_label = fabric.joint_label(interval.alpha_key);
            let omega_label = fabric.joint_label(interval.omega_key);
            let shown_length = displayed_length.get(&info.key).copied().unwrap_or(info.length);
            writeln!(
                file,
                "{},{},{:.3},{:.3e},{:.3},{:.3},{:.3},{},0,90,{:.3},{:.3},{:.3},{},0,90",
                current_index,
                role_str,
                shown_length,
                info.strain,
                alpha.x, alpha.y, alpha.z, alpha_label,
                omega.x, omega.y, omega.z, omega_label,
            )?;
        } else {
            let alpha_info = pull_bend_info
                .iter()
                .find(|((pull_id, end, _), _)| {
                    *pull_id == info.key && *end == IntervalEnd::Alpha
                })
                .map(|(_, data)| data);
            let omega_info = pull_bend_info
                .iter()
                .find(|((pull_id, end, _), _)| {
                    *pull_id == info.key && *end == IntervalEnd::Omega
                })
                .map(|(_, data)| data);

            let (alpha_pos, alpha_joint_label, alpha_slot, alpha_bend) =
                if let Some((pull_end_pos, _, joint_key, slot, bend, _)) = alpha_info {
                    (
                        to_csv * (*pull_end_pos * MM_PER_METER),
                        fabric.joint_label(*joint_key),
                        *slot,
                        Some(*bend),
                    )
                } else {
                    let joint = &fabric.joints[interval.alpha_key];
                    (
                        to_csv * (joint.location * MM_PER_METER),
                        fabric.joint_label(interval.alpha_key),
                        0,
                        None,
                    )
                };

            let (omega_pos, omega_joint_label, omega_slot, omega_bend) =
                if let Some((pull_end_pos, _, joint_key, slot, bend, _)) = omega_info {
                    (
                        to_csv * (*pull_end_pos * MM_PER_METER),
                        fabric.joint_label(*joint_key),
                        *slot,
                        Some(*bend),
                    )
                } else {
                    let joint = &fabric.joints[interval.omega_key];
                    (
                        to_csv * (joint.location * MM_PER_METER),
                        fabric.joint_label(interval.omega_key),
                        0,
                        None,
                    )
                };

            let geom_length = (omega_pos - alpha_pos).length() / MM_PER_METER;
            let shortened_length = displayed_length.get(&info.key).copied().unwrap_or(geom_length);
            let alpha_bend_str = alpha_bend.map_or(String::new(), |b| b.to_string());
            let omega_bend_str = omega_bend.map_or(String::new(), |b| b.to_string());

            writeln!(
                file,
                "{},{},{:.3},{:.3e},{:.3},{:.3},{:.3},{},{},{},{:.3},{:.3},{:.3},{},{},{}",
                current_index,
                role_str,
                shortened_length,
                info.strain,
                alpha_pos.x,
                alpha_pos.y,
                alpha_pos.z,
                alpha_joint_label,
                alpha_slot,
                alpha_bend_str,
                omega_pos.x,
                omega_pos.y,
                omega_pos.z,
                omega_joint_label,
                omega_slot,
                omega_bend_str,
            )?;
        }
    }

    // FEA intervals: push-fea (joint → highest ring center) and pull-fea
    // (highest ring center → highest ring center), sorted short-to-long
    // within each group.
    struct FeaIntervalInfo {
        is_push: bool,
        length: f32,
        strain: f32,
        alpha_pos: Vec3,
        omega_pos: Vec3,
        alpha_joint_label: String,
        omega_joint_label: String,
        alpha_slot: usize,
        omega_slot: usize,
    }

    let mut fea_infos: Vec<FeaIntervalInfo> = Vec::new();

    for info in interval_infos.iter().filter(|i| i.is_push) {
        let interval = fabric.intervals.get(info.key).unwrap();
        let alpha_joint = &fabric.joints[interval.alpha_key];
        let omega_joint = &fabric.joints[interval.omega_key];

        let alpha_highest = highest_slot_per_joint
            .get(&interval.alpha_key)
            .copied()
            .unwrap_or(0);
        let omega_highest = highest_slot_per_joint
            .get(&interval.omega_key)
            .copied()
            .unwrap_or(0);

        let alpha_fea = if alpha_highest > 0 {
            ring_centers
                .get(&(interval.alpha_key, alpha_highest))
                .copied()
                .unwrap_or(alpha_joint.location)
        } else {
            alpha_joint.location
        };
        let omega_fea = if omega_highest > 0 {
            ring_centers
                .get(&(interval.omega_key, omega_highest))
                .copied()
                .unwrap_or(omega_joint.location)
        } else {
            omega_joint.location
        };

        let fea_length = (omega_fea - alpha_fea).length();

        fea_infos.push(FeaIntervalInfo {
            is_push: true,
            length: fea_length,
            strain: info.strain,
            alpha_pos: alpha_fea,
            omega_pos: omega_fea,
            alpha_joint_label: fabric.joint_label(interval.alpha_key),
            omega_joint_label: fabric.joint_label(interval.omega_key),
            alpha_slot: alpha_highest,
            omega_slot: omega_highest,
        });
    }

    for info in interval_infos.iter().filter(|i| !i.is_push) {
        let interval = fabric.intervals.get(info.key).unwrap();

        let alpha_info = pull_bend_info
            .iter()
            .find(|((pull_id, end, _), _)| *pull_id == info.key && *end == IntervalEnd::Alpha)
            .map(|((_, _, _slot), (_, _, joint_key, _, _, _))| *joint_key);
        let omega_info = pull_bend_info
            .iter()
            .find(|((pull_id, end, _), _)| *pull_id == info.key && *end == IntervalEnd::Omega)
            .map(|((_, _, _slot), (_, _, joint_key, _, _, _))| *joint_key);

        let (alpha_fea, alpha_joint_label, alpha_slot) = if let Some(joint_key) = alpha_info {
            let highest_slot = highest_slot_per_joint.get(&joint_key).copied().unwrap_or(0);
            let ring = if highest_slot > 0 {
                ring_centers
                    .get(&(joint_key, highest_slot))
                    .copied()
                    .unwrap_or(fabric.joints[joint_key].location)
            } else {
                fabric.joints[joint_key].location
            };
            (ring, fabric.joint_label(joint_key), highest_slot)
        } else {
            let joint = &fabric.joints[interval.alpha_key];
            (joint.location, fabric.joint_label(interval.alpha_key), 0)
        };

        let (omega_fea, omega_joint_label, omega_slot) = if let Some(joint_key) = omega_info {
            let highest_slot = highest_slot_per_joint.get(&joint_key).copied().unwrap_or(0);
            let ring = if highest_slot > 0 {
                ring_centers
                    .get(&(joint_key, highest_slot))
                    .copied()
                    .unwrap_or(fabric.joints[joint_key].location)
            } else {
                fabric.joints[joint_key].location
            };
            (ring, fabric.joint_label(joint_key), highest_slot)
        } else {
            let joint = &fabric.joints[interval.omega_key];
            (joint.location, fabric.joint_label(interval.omega_key), 0)
        };

        let fea_length = (omega_fea - alpha_fea).length();

        fea_infos.push(FeaIntervalInfo {
            is_push: false,
            length: fea_length,
            strain: info.strain,
            alpha_pos: alpha_fea,
            omega_pos: omega_fea,
            alpha_joint_label,
            omega_joint_label,
            alpha_slot,
            omega_slot,
        });
    }

    fea_infos.sort_by(|a, b| match (a.is_push, b.is_push) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a
            .length
            .partial_cmp(&b.length)
            .unwrap_or(std::cmp::Ordering::Equal),
    });

    for fea in &fea_infos {
        current_index += 1;
        let role_str = if fea.is_push { "push-fea" } else { "pull-fea" };
        let alpha_mm = to_csv * (fea.alpha_pos * MM_PER_METER);
        let omega_mm = to_csv * (fea.omega_pos * MM_PER_METER);

        writeln!(
            file,
            "{},{},{:.3},{:.3e},{:.3},{:.3},{:.3},{},{},90,{:.3},{:.3},{:.3},{},{},90",
            current_index,
            role_str,
            fea.length,
            fea.strain,
            alpha_mm.x,
            alpha_mm.y,
            alpha_mm.z,
            fea.alpha_joint_label,
            fea.alpha_slot,
            omega_mm.x,
            omega_mm.y,
            omega_mm.z,
            fea.omega_joint_label,
            fea.omega_slot,
        )?;
    }

    // Connector link rows: joint → ring center → tab → pull-end.
    let mut push_end_connections: BTreeMap<JointKey, Vec<(usize, Vec3, Vec3)>> = BTreeMap::new();

    for (_, (pull_end_pos, tab_pos, joint_key, slot, _, _)) in &pull_bend_info {
        push_end_connections.entry(*joint_key).or_default().push((
            *slot,
            *pull_end_pos,
            *tab_pos,
        ));
    }

    let mut link_index = current_index;

    for (joint_key, mut connections) in push_end_connections {
        connections.sort_by_key(|(slot, _, _)| *slot);

        let joint = &fabric.joints[joint_key];
        let joint_pos = joint.location;

        let push_axis = fabric
            .intervals
            .values()
            .find(|i| {
                i.has_role(Role::Pushing)
                    && (i.alpha_key == joint_key || i.omega_key == joint_key)
            })
            .map(|push_interval| {
                let alpha_pos = fabric.joints[push_interval.alpha_key].location;
                let omega_pos = fabric.joints[push_interval.omega_key].location;
                let dir = (omega_pos - alpha_pos).normalize();
                if push_interval.alpha_key == joint_key {
                    -dir
                } else {
                    dir
                }
            })
            .unwrap_or(Vec3::Y);

        let mut prev_pos = joint_pos;
        let mut prev_slot = 0usize;

        let joint_label = fabric.joint_label(joint_key);
        for (slot, pull_end_pos, tab_pos) in &connections {
            let ring_center = dimensions.ring_center(joint_pos, push_axis, *slot - 1);

            link_index += 1;
            let prev_mm = to_csv * (prev_pos * MM_PER_METER);
            let ring_mm = to_csv * (ring_center * MM_PER_METER);
            let axial_length = (ring_center - prev_pos).length();
            writeln!(
                file,
                "{},axial,{:.3},0.000e0,{:.3},{:.3},{:.3},{},{},90,{:.3},{:.3},{:.3},{},{},90",
                link_index, axial_length,
                prev_mm.x, prev_mm.y, prev_mm.z, joint_label, prev_slot,
                ring_mm.x, ring_mm.y, ring_mm.z, joint_label, slot,
            )?;

            link_index += 1;
            let tab_mm = to_csv * (*tab_pos * MM_PER_METER);
            let radial_length = (*tab_pos - ring_center).length();
            writeln!(
                file,
                "{},radial,{:.3},0.000e0,{:.3},{:.3},{:.3},{},{},0.000,{:.3},{:.3},{:.3},{},{},0.000",
                link_index, radial_length,
                ring_mm.x, ring_mm.y, ring_mm.z, joint_label, slot,
                tab_mm.x, tab_mm.y, tab_mm.z, joint_label, slot,
            )?;

            link_index += 1;
            let pull_end_mm = to_csv * (*pull_end_pos * MM_PER_METER);
            let tab_link_length = (*pull_end_pos - *tab_pos).length();
            writeln!(
                file,
                "{},tab,{:.3},0.000e0,{:.3},{:.3},{:.3},{},{},0.000,{:.3},{:.3},{:.3},{},{},0.000",
                link_index, tab_link_length,
                tab_mm.x, tab_mm.y, tab_mm.z, joint_label, slot,
                pull_end_mm.x, pull_end_mm.y, pull_end_mm.z, joint_label, slot,
            )?;

            prev_pos = ring_center;
            prev_slot = *slot;
        }
    }

    println!("Exported {} to {}", fabric.name, filename);
    Ok(())
}

fn format_unsigned_angle(deg: f32) -> String {
    if (deg - deg.round()).abs() < 0.05 {
        format!("{}°", deg.round() as i32)
    } else {
        format!("{:.1}°", deg)
    }
}

fn format_signed_angle(deg: f32) -> String {
    if deg.abs() < 0.05 {
        "0°".to_string()
    } else if (deg - deg.round()).abs() < 0.05 {
        format!("{:+}°", deg.round() as i32)
    } else {
        format!("{:+.1}°", deg)
    }
}

fn build_bend_summary(fabric: &Fabric) -> String {
    use crate::fabric::bend_optimizer::snap_to_magnitudes;
    use std::fmt::Write;

    let mut s = String::new();
    let h = &fabric.dimensions.connector;
    let ideals = fabric.collect_ideal_bend_angles();

    writeln!(s, "# === Bend snap quality ===").ok();
    writeln!(s, "# Bend count (K):       {}", h.bend_count).ok();
    if h.bend_magnitudes_locked {
        writeln!(s, "# Magnitude source:     LOCKED to factory inventory (no per-export reoptimisation)").ok();
    } else {
        writeln!(s, "# Magnitude source:     k-center optimiser, recomputed per export").ok();
    }

    if h.bend_count == 0 {
        writeln!(s, "# Snapping disabled (K = 0); CSV uses continuous ideal angles.").ok();
        writeln!(s, "# Cable ends measured: {}", ideals.len()).ok();
        writeln!(s, "#").ok();
        return s;
    }
    if ideals.is_empty() {
        writeln!(s, "# Cable ends measured: 0  (no pull connections)").ok();
        writeln!(s, "#").ok();
        return s;
    }

    let mags = &h.bend_magnitudes;
    let mag_str: Vec<String> = mags.iter().map(|m| format_unsigned_angle(*m)).collect();
    writeln!(s, "# Optimal magnitudes:   [{}]", mag_str.join(", ")).ok();

    let mut signed: Vec<f32> = Vec::with_capacity(mags.len() * 2);
    for &m in mags.iter().rev() {
        if m != 0.0 {
            signed.push(-m);
        }
    }
    for &m in mags {
        signed.push(m);
    }
    let signed_str: Vec<String> = signed.iter().map(|m| format_signed_angle(*m)).collect();
    writeln!(s, "# Effective signed set: [{}]", signed_str.join(", ")).ok();

    let n = ideals.len();
    let snapped: Vec<(f32, f32)> = ideals
        .iter()
        .map(|&x| snap_to_magnitudes(x, mags))
        .collect();

    let signed_counts: Vec<usize> = signed
        .iter()
        .map(|&candidate| {
            snapped
                .iter()
                .filter(|(snap, _)| (snap - candidate).abs() < 0.5)
                .count()
        })
        .collect();
    let mag_counts: Vec<usize> = mags
        .iter()
        .map(|&m| {
            snapped
                .iter()
                .filter(|(snap, _)| (snap.abs() - m).abs() < 0.5)
                .count()
        })
        .collect();

    let counts_line = signed_counts
        .iter()
        .zip(signed.iter())
        .map(|(c, m)| format!("{}×{}", format_signed_angle(*m), c))
        .collect::<Vec<_>>()
        .join("  ");
    writeln!(s, "# Bend counts (signed): {}", counts_line).ok();

    let mag_counts_line = mag_counts
        .iter()
        .zip(mags.iter())
        .map(|(c, m)| format!("{}×{}", format_unsigned_angle(*m), c))
        .collect::<Vec<_>>()
        .join("  ");
    writeln!(s, "# Bend counts (per magnitude): {}", mag_counts_line).ok();

    let errors: Vec<f32> = snapped.iter().map(|(_, e)| *e).collect();
    let mean = errors.iter().sum::<f32>() / n as f32;
    let max = errors.iter().fold(0.0_f32, |a, &b| a.max(b));
    let rms = (errors.iter().map(|e| e * e).sum::<f32>() / n as f32).sqrt();
    writeln!(s, "# Cable ends measured:  {}", n).ok();
    writeln!(
        s,
        "# Snap error:           mean |Δ|={:.2}°  max |Δ|={:.2}°  RMS={:.2}°",
        mean, max, rms
    )
    .ok();
    writeln!(s, "#").ok();
    s
}

fn build_clearance_summary(fabric: &Fabric) -> String {
    use crate::fabric::attachment::segment_segment_distance;
    use std::fmt::Write;

    let mut s = String::new();
    let mut pair_distances: Vec<f32> = Vec::new();
    let mut ends_measured: usize = 0;

    for (_key, push_interval) in fabric.intervals.iter() {
        if !push_interval.has_role(Role::Pushing) {
            continue;
        }
        let alpha_pos = fabric.joints[push_interval.alpha_key].location;
        let omega_pos = fabric.joints[push_interval.omega_key].location;
        let push_dir = (omega_pos - alpha_pos).normalize();

        for interval_end in [IntervalEnd::Alpha, IntervalEnd::Omega] {
            let (end_pos, axis_dir, end_key) = match interval_end {
                IntervalEnd::Alpha => (alpha_pos, -push_dir, push_interval.alpha_key),
                IntervalEnd::Omega => (omega_pos, push_dir, push_interval.omega_key),
            };
            let Some(connections) = push_interval.connections(interval_end) else {
                continue;
            };

            let mut segs: Vec<(Vec3, Vec3)> = Vec::new();
            for (slot_idx, conn_opt) in connections.iter().enumerate() {
                let Some(connection) = conn_opt else { continue };
                let Some(pull_interval) = fabric.intervals.get(connection.pull_interval_key)
                else {
                    continue;
                };
                let pull_other_end = if pull_interval.alpha_key == end_key {
                    fabric.joints[pull_interval.omega_key].location
                } else {
                    fabric.joints[pull_interval.alpha_key].location
                };
                let (tab_pos, _bend, pull_end_pos, _ideal) = fabric.dimensions.tab_geometry(
                    end_pos,
                    axis_dir,
                    slot_idx,
                    pull_other_end,
                );
                segs.push((tab_pos, pull_end_pos));
            }

            if segs.len() >= 2 {
                ends_measured += 1;
                for i in 0..segs.len() {
                    for j in (i + 1)..segs.len() {
                        let d =
                            segment_segment_distance(segs[i].0, segs[i].1, segs[j].0, segs[j].1);
                        pair_distances.push(d);
                    }
                }
            }
        }
    }

    writeln!(s, "# === Tab arm clearance ===").ok();
    writeln!(
        s,
        "# Per joint-end, minimum 3D distance between any two arm segments (tab_pos -> pull_end_pos)."
    )
    .ok();
    if pair_distances.is_empty() {
        writeln!(s, "# No multi-cable joint-ends measured.").ok();
        writeln!(s, "#").ok();
        return s;
    }

    let n = pair_distances.len();
    let min_m = pair_distances.iter().copied().fold(f32::INFINITY, f32::min);
    let max_m = pair_distances.iter().copied().fold(0.0_f32, f32::max);
    let mean_m = pair_distances.iter().sum::<f32>() / n as f32;
    writeln!(s, "# Joint-ends measured: {}", ends_measured).ok();
    writeln!(s, "# Pairs measured:      {}", n).ok();
    writeln!(
        s,
        "# Clearance:           min={:.1}mm  mean={:.1}mm  max={:.1}mm",
        min_m * MM_PER_METER,
        mean_m * MM_PER_METER,
        max_m * MM_PER_METER,
    )
    .ok();
    writeln!(s, "#").ok();
    s
}

struct IntervalInfo {
    key: IntervalKey,
    is_push: bool,
    length: f32,
    strain: f32,
}

/// Build a (interval_key → displayed_length) map that replaces each member's
/// length with the rotational group mean, so the CSV shows identical mm
/// values across every triple instead of straddling rounding boundaries.
///
/// For pushes the length is alpha-to-omega joint distance (already in
/// `info.length`). For pulls it's the *shortened* length (tab-end to
/// tab-end), recomputed here from each pull's connection geometry.
///
/// Triples touching the central apex joints `YZ0` / `YZ1` are deliberately
/// excluded: the three rotational copies converging there land on three
/// different slots of the single apex push and legitimately have three
/// different lengths. Their displayed lengths stay the per-interval values.
fn build_group_mean_lengths(
    fabric: &Fabric,
    interval_infos: &[IntervalInfo],
) -> std::collections::HashMap<IntervalKey, f32> {
    use std::collections::HashMap;

    // For each interval, compute the length we'd display and capture its
    // canonical group key and a flag indicating apex-axis attachment.
    let mut per_interval: Vec<(IntervalKey, (String, String), f32, bool)> =
        Vec::with_capacity(interval_infos.len());

    for info in interval_infos {
        let interval = &fabric.intervals[info.key];
        let a_label = fabric.joint_label(interval.alpha_key);
        let b_label = fabric.joint_label(interval.omega_key);
        let touches_apex_axis =
            is_apex_axis_label(&a_label) || is_apex_axis_label(&b_label);
        let length = if info.is_push {
            info.length
        } else {
            pull_shortened_length(fabric, info.key)
        };
        per_interval.push((info.key, canonical_push_key(&a_label, &b_label), length, touches_apex_axis));
    }

    // Group by canonical key.
    let mut groups: std::collections::BTreeMap<(String, String), Vec<(IntervalKey, f32, bool)>> =
        std::collections::BTreeMap::new();
    for (key, canon, len, apex) in per_interval {
        groups.entry(canon).or_default().push((key, len, apex));
    }

    // For non-apex groups of size >= 2, assign every member the group mean.
    // Singletons and apex-touching groups keep their per-interval lengths.
    let mut out: HashMap<IntervalKey, f32> = HashMap::new();
    for (_canon, members) in groups {
        let any_apex = members.iter().any(|(_, _, apex)| *apex);
        if any_apex || members.len() < 2 {
            for (key, len, _) in members {
                out.insert(key, len);
            }
        } else {
            let mean = members.iter().map(|(_, l, _)| *l).sum::<f32>() / members.len() as f32;
            for (key, _, _) in members {
                out.insert(key, mean);
            }
        }
    }
    out
}

/// `YZ0` / `YZ1` are the only joints on Open Claw's central rotational axis
/// and don't rotate; cables landing there share a single push end at three
/// different slots. They are the sole source of legitimate within-triple
/// length spread.
fn is_apex_axis_label(label: &str) -> bool {
    label == "YZ0" || label == "YZ1"
}

/// Recompute a pull's "shortened" length — tab-endpoint to tab-endpoint
/// — matching exactly how the CSV row computes it from positions. Falls back
/// to joint-to-joint distance when the pull isn't attached to any push end.
fn pull_shortened_length(fabric: &Fabric, key: IntervalKey) -> f32 {
    let pull = &fabric.intervals[key];
    let alpha_pos = pull_endpoint_position(fabric, key, pull.alpha_key);
    let omega_pos = pull_endpoint_position(fabric, key, pull.omega_key);
    (omega_pos - alpha_pos).length()
}

/// Walks the push intervals to find where this pull's `near` end is attached
/// and returns the tab endpoint (shortened position). If unattached, the
/// joint location itself.
fn pull_endpoint_position(fabric: &Fabric, pull_key: IntervalKey, near: JointKey) -> Vec3 {
    for (_pk, push) in fabric.intervals.iter() {
        if !push.has_role(Role::Pushing) {
            continue;
        }
        for end in [IntervalEnd::Alpha, IntervalEnd::Omega] {
            let end_joint = match end {
                IntervalEnd::Alpha => push.alpha_key,
                IntervalEnd::Omega => push.omega_key,
            };
            if end_joint != near {
                continue;
            }
            let Some(conns) = push.connections(end) else { continue };
            for (slot_idx, conn_opt) in conns.iter().enumerate() {
                let Some(conn) = conn_opt else { continue };
                if conn.pull_interval_key != pull_key {
                    continue;
                }
                let alpha_pos = fabric.joints[push.alpha_key].location;
                let omega_pos = fabric.joints[push.omega_key].location;
                let push_dir = (omega_pos - alpha_pos).normalize();
                let (end_pos, axis_dir) = match end {
                    IntervalEnd::Alpha => (alpha_pos, -push_dir),
                    IntervalEnd::Omega => (omega_pos, push_dir),
                };
                let pull = &fabric.intervals[pull_key];
                let other = if pull.alpha_key == near {
                    fabric.joints[pull.omega_key].location
                } else {
                    fabric.joints[pull.alpha_key].location
                };
                let (_tab_pos, _bend, pull_end_pos, _ideal) =
                    fabric.dimensions.tab_geometry(end_pos, axis_dir, slot_idx, other);
                return pull_end_pos;
            }
        }
    }
    fabric.joints[near].location
}

fn write_dimensions_comments(file: &mut File, dims: &FabricDimensions) -> io::Result<()> {
    let h = &dims.connector;
    let mm = |m: f32| m * 1000.0;
    let a = h.push_radius.f32();
    let b = h.push_radius_margin.f32();
    let t1 = h.disc_thickness.f32();
    let t2 = h.disc_separator_thickness.f32();
    let c = t1 / 2.0;
    let d = h.tab_extension.f32();
    let e = h.tab_hole_diameter.f32();
    let cap = h.cap_thickness.f32();
    writeln!(file, "#")?;
    writeln!(file, "# === Connector parameters (see diagram) ===")?;
    writeln!(file, "# A  radius van de buis (push_radius):        {:.1}mm  ({:.5}m)", mm(a), a)?;
    writeln!(file, "# B  marge (push_radius_margin):              {:.1}mm  ({:.5}m)", mm(b), b)?;
    writeln!(file, "# C  offset door radius (= t1/2):             {:.1}mm  ({:.5}m)", mm(c), c)?;
    writeln!(file, "# D  randafstand (tab_extension):           {:.1}mm  ({:.5}m)", mm(d), d)?;
    writeln!(file, "# E  diameter gat (tab_hole_diameter):      {:.1}mm  ({:.5}m)", mm(e), e)?;
    writeln!(file, "# t1 dikte staal (disc_thickness):             {:.1}mm  ({:.5}m)", mm(t1), t1)?;
    writeln!(file, "# t2 dikte POM (disc_separator):              {:.1}mm  ({:.5}m)", mm(t2), t2)?;
    writeln!(file, "#    cap_thickness:                           {:.1}mm  ({:.5}m)", mm(cap), cap)?;
    writeln!(file, "#    pull_radius:                             {:.1}mm  ({:.5}m)", mm(dims.pull_radius.f32()), dims.pull_radius.f32())?;
    writeln!(file, "#")?;
    writeln!(file, "# === Afgeleide waarden ===")?;
    writeln!(file, "#    A + B + C  = {:.1}mm  (halve breedte schijf)", mm(a + b + c))?;
    writeln!(file, "#    C + D + E  = {:.1}mm  (tab lengte)", mm(c + d + e))?;
    writeln!(file, "#    t1 + t2    = {:.1}mm  (schijf + separator)", mm(t1 + t2))?;
    writeln!(
        file,
        "#    disc_center_offset(0) = {:.1}mm  (as-afstand tot centrum eerste schijf)",
        mm(h.disc_center_offset(0).f32()),
    )?;
    writeln!(file, "#")?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::dsl::fabric_library::{self, FabricName};
    use crate::build::dsl::fabric_plan_executor::{ExecutorStage, FabricPlanExecutor};

    /// Build Open Claw through Building (so `slacken` has run) and stop —
    /// no pretensing ticks. This is the moment the CSV captures.
    fn build_to_slack() -> FabricPlanExecutor {
        let plan = fabric_library::get_fabric_plan(FabricName::OpenClaw);
        let mut executor = FabricPlanExecutor::new(plan);
        while *executor.stage() == ExecutorStage::Building {
            let _ = executor.iterate();
        }
        executor
    }

    /// Drive the full slack-CSV pipeline:
    ///   1. Per-push slot assignment (generic).
    ///   2. Enforce 3-fold symmetry (OpenClaw-specific).
    ///   3. Recompute bend magnitudes (generic).
    ///   4. Write the engineering CSV.
    ///   5. Assert: every rotational triple of intervals has identical
    ///      length, slot, and bend angle at each end.
    ///
    /// The CSV path is `OpenClaw-<date>.csv` in the working directory — the
    /// same file the manufacturing pipeline reads.
    #[test]
    fn test_open_claw_threefold_symmetry() {
        let mut executor = build_to_slack();

        executor.fabric.update_all_attachment_connections();
        apply_threefold_symmetry(&mut executor.fabric);
        executor.fabric.recompute_bend_magnitudes();

        let date = chrono::Local::now().format("%Y-%m-%d");
        let filename = format!("OpenClaw-{}.csv", date);
        write_csv(&executor.fabric, &filename).expect("CSV write failed");

        verify_threefold_symmetry(&executor.fabric);
    }

    /// All 180 cables should group into 60 triples by 3-fold symmetry, with
    /// within-triple length spreads under 1 mm. (Apex-attached cables are
    /// allowed a small budget — three rotational copies converging on the
    /// single apex push must occupy three different slots, so their lengths
    /// differ by the disc step.)
    #[test]
    fn test_open_claw_cable_triples() {
        let mut executor = build_to_slack();
        executor.fabric.update_all_attachment_connections();
        apply_threefold_symmetry(&mut executor.fabric);

        let fabric = &executor.fabric;
        let mut cables: Vec<(String, String, f32)> = Vec::new();
        for (_key, interval) in fabric.intervals.iter() {
            if !interval.role.is_pull_like() {
                continue;
            }
            let a = fabric.joint_label(interval.alpha_key);
            let b = fabric.joint_label(interval.omega_key);
            cables.push((a, b, interval.length(&fabric.joints)));
        }
        assert_eq!(cables.len(), 180, "expected 180 pull intervals in OpenClaw");

        let mut groups: BTreeMap<(String, String), Vec<(String, String, f32)>> = BTreeMap::new();
        for cable in &cables {
            groups
                .entry(canonical_push_key(&cable.0, &cable.1))
                .or_default()
                .push(cable.clone());
        }

        let wrong_sized: Vec<_> = groups.iter().filter(|(_, v)| v.len() != 3).collect();
        assert!(
            wrong_sized.is_empty(),
            "{} cable groups did not have exactly 3 members",
            wrong_sized.len()
        );
        assert_eq!(groups.len(), 60, "expected 60 cable triples");

        const TOLERANCE_MM: f32 = 1.0;
        let mut worst: Vec<(String, f32, f32, f32)> = Vec::new();
        for ((ka, kb), group) in &groups {
            let mut lengths: Vec<f32> = group.iter().map(|c| c.2).collect();
            lengths.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let spread_mm = (lengths[2] - lengths[0]) * MM_PER_METER;
            if spread_mm > TOLERANCE_MM {
                worst.push((
                    format!("({}, {})", ka, kb),
                    lengths[0] * MM_PER_METER,
                    lengths[1] * MM_PER_METER,
                    lengths[2] * MM_PER_METER,
                ));
            }
        }
        if !worst.is_empty() {
            eprintln!("Cable triples exceeding {:.1}mm spread:", TOLERANCE_MM);
            for (key, a, b, c) in &worst {
                eprintln!("  {key}: {:.2}/{:.2}/{:.2}mm  (Δ={:.2}mm)", a, b, c, c - a);
            }
        }
        // Apex-attached cables are geometrically forced apart (the 2 triples
        // touching YZ0/YZ1 land on three slots of the single apex push).
        // Everything else should be sub-mm.
        let apex_triples = worst
            .iter()
            .filter(|(key, _, _, _)| key.contains("YZ0") || key.contains("YZ1"))
            .count();
        let non_apex_outliers = worst.len() - apex_triples;
        assert_eq!(
            non_apex_outliers, 0,
            "{} non-apex cable triples have spreads above {}mm",
            non_apex_outliers, TOLERANCE_MM,
        );
    }

    /// For each push group (rotationally-equivalent triple), assert that all
    /// three members agree on length and on the slot/bend assignment at
    /// every cable end. Tolerance is 0.1 mm / 0.5° to allow tiny f32 drift.
    fn verify_threefold_symmetry(fabric: &Fabric) {
        // Group pushes.
        let mut push_groups: BTreeMap<(String, String), Vec<IntervalKey>> = BTreeMap::new();
        for (key, interval) in fabric.intervals.iter() {
            if !interval.has_role(Role::Pushing) {
                continue;
            }
            let a = fabric.joint_label(interval.alpha_key);
            let b = fabric.joint_label(interval.omega_key);
            push_groups
                .entry(canonical_push_key(&a, &b))
                .or_default()
                .push(key);
        }

        const LEN_TOL_M: f32 = 1.0e-4; // 0.1 mm
        let mut failures: Vec<String> = Vec::new();

        for ((ka, kb), members) in &push_groups {
            if members.len() < 2 {
                continue;
            }
            // Push lengths.
            let lengths: Vec<f32> = members
                .iter()
                .map(|&k| fabric.intervals[k].length(&fabric.joints))
                .collect();
            let lmin = lengths.iter().copied().fold(f32::INFINITY, f32::min);
            let lmax = lengths.iter().copied().fold(0.0_f32, f32::max);
            if lmax - lmin > LEN_TOL_M {
                failures.push(format!(
                    "push ({ka}, {kb}) length spread {:.4}m across {} members",
                    lmax - lmin,
                    members.len()
                ));
            }

            // Slot fingerprint at each end: sorted list of (canonical_other_label, slot)
            // pairs. Members must produce identical fingerprints.
            let fingerprint = |push_key: IntervalKey, end: IntervalEnd| -> Vec<(String, usize)> {
                let push = &fabric.intervals[push_key];
                let near = match end {
                    IntervalEnd::Alpha => push.alpha_key,
                    IntervalEnd::Omega => push.omega_key,
                };
                let mut v: Vec<(String, usize)> = Vec::new();
                if let Some(conns) = push.connections(end) {
                    for (slot, opt) in conns.iter().enumerate() {
                        let Some(pc) = opt else { continue };
                        let pull = &fabric.intervals[pc.pull_interval_key];
                        let other = if pull.alpha_key == near {
                            pull.omega_key
                        } else {
                            pull.alpha_key
                        };
                        let label = fabric.joint_label(other);
                        // Canonical of the far-end label = rotation-orbit identifier.
                        let canon = {
                            let l0 = label.clone();
                            let l1 = rotate_label_once(&l0);
                            let l2 = rotate_label_once(&l1);
                            std::cmp::min(std::cmp::min(l0, l1), l2)
                        };
                        v.push((canon, slot));
                    }
                }
                v.sort();
                v
            };

            let rep = members[0];
            for end in [IntervalEnd::Alpha, IntervalEnd::Omega] {
                let rep_fp = fingerprint(rep, end);
                for &m in &members[1..] {
                    let m_fp_a = fingerprint(m, IntervalEnd::Alpha);
                    let m_fp_o = fingerprint(m, IntervalEnd::Omega);
                    // Member's matching end may be alpha or omega depending on swap.
                    if m_fp_a != rep_fp && m_fp_o != rep_fp {
                        failures.push(format!(
                            "push ({ka}, {kb}) end {:?}: member {} fingerprint disagrees",
                            end, fabric.joint_label(fabric.intervals[m].alpha_key)
                        ));
                    }
                }
            }
        }

        if !failures.is_empty() {
            for f in &failures {
                eprintln!("  ✗ {}", f);
            }
            panic!("{} symmetry failures", failures.len());
        }
    }
}
