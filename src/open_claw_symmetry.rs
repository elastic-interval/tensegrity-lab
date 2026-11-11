//! Open Claw's threefold symmetry: enforcing it on the slot assignments, and
//! the tests that hold it.
//!
//!   - Rotation helpers for the joint-label scheme (`docs/joint-naming.md`)
//!     — A→B→C sector cycling on the sector letter (`A12` → `B12` → `C12`).
//!   - [`apply_threefold_symmetry`] — copies one representative's slot
//!     assignments to its rotational partners after the generic per-push
//!     algorithm has run, so floating-point ε doesn't flip cables between
//!     adjacent slots within a triple.
//!   - [`test_open_claw_threefold_symmetry`] asserts every rotational triple
//!     agrees on length and slot; [`test_open_claw_cable_triples`] looks at
//!     cable lengths alone.
//!
//! Open Claw's engineering CSV is archived on an old branch.

use std::collections::BTreeMap;

use crate::fabric::attachment::{PullConnection, ATTACHMENT_POINTS};
use crate::fabric::interval::Role;
use crate::fabric::{Fabric, IntervalEnd, IntervalKey, JointKey};

// ─────────────────────────────────────────────────────────────────────────────
// Rotation helpers — Open Claw label scheme
// ─────────────────────────────────────────────────────────────────────────────

/// Rotate a joint label by 120° about the central axis (A→B→C).
///
/// Two label shapes participate (see `docs/joint-naming.md`):
///   - Off-axis (`<sector><number>`): `A12` → `B12` → `C12` → `A12`. The
///     sector letter, one of `A`/`B`/`C`, cycles forwards; the number, shared
///     by all three copies, stays. A cable end's slot suffix (`A12.2`)
///     rides along.
///   - On-axis singleton (`Z<index>`): unchanged — these joints lie on the
///     rotation axis and map to themselves under the 120° rotation.
///
/// Any other shape (a `JointPath` fallback string, an empty label, etc.) is
/// returned unchanged.
fn rotate_label_once(label: &str) -> String {
    let bytes = label.as_bytes();
    if bytes.len() < 2 {
        return label.to_string();
    }
    let next_sector = match bytes[0] {
        b'A' => b'B',
        b'B' => b'C',
        b'C' => b'A',
        _ => return label.to_string(),
    };
    if !bytes[1..].iter().all(|b| b.is_ascii_digit() || *b == b'.') {
        return label.to_string();
    }
    let mut out = Vec::with_capacity(bytes.len());
    out.push(next_sector);
    out.extend_from_slice(&bytes[1..]);
    String::from_utf8(out).expect("ascii in -> ascii out")
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

    // Slot assignments changed; the collision culprits must reflect them.
    fabric.mark_connector_culprits();
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

    let Some(connector) = fabric.connector.as_mut() else { return };
    let Some(connections) = connector.connections_mut(member_key) else { return };
    connections.alpha = [None; ATTACHMENT_POINTS];
    connections.omega = [None; ATTACHMENT_POINTS];
    for (pull_key, slot) in alpha_pulls {
        if slot < ATTACHMENT_POINTS {
            connections.alpha[slot] = Some(PullConnection {
                pull_interval_key: pull_key,
                attachment_index: slot,
                reach: None,
                lift: 0,
            });
        }
    }
    for (pull_key, slot) in omega_pulls {
        if slot < ATTACHMENT_POINTS {
            connections.omega[slot] = Some(PullConnection {
                pull_interval_key: pull_key,
                attachment_index: slot,
                reach: None,
                lift: 0,
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
    let Some(connections) = fabric
        .connector
        .as_ref()
        .and_then(|connector| connector.connections(push_key, end))
    else {
        return out;
    };
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
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::dsl::fabric_library::{self, FabricName};
    use crate::build::dsl::fabric_plan_executor::{ExecutorStage, FabricPlanExecutor};
    use crate::units::{Unit, MM_PER_METER};

    /// Build Open Claw through Building (so `slacken` has run) and stop —
    /// no pretensing ticks: the slack moment.
    fn build_to_slack() -> FabricPlanExecutor {
        let plan = fabric_library::get_fabric_plan(FabricName::OpenClaw);
        let mut executor = FabricPlanExecutor::new(plan);
        while *executor.stage() == ExecutorStage::Building {
            let _ = executor.iterate();
        }
        executor
    }

    /// Asserts the symmetry invariant — every rotational triple agrees on
    /// length, slot, and bend at each end — without writing anything to disk.
    /// Runs on every `cargo test`.
    #[test]
    fn test_open_claw_threefold_symmetry() {
        let mut executor = build_to_slack();

        executor.fabric.update_all_attachment_connections();
        apply_threefold_symmetry(&mut executor.fabric);

        verify_threefold_symmetry(&executor.fabric);
    }

    /// With the prototype connector (5 mm cap, washer 2 mm, pivot radius 30 mm,
    /// 18 mm tube, 12 mm pin), slots ordered for fewest colliding forks and the
    /// boss arms in the capsule model, it finds 18 colliding cable-end
    /// assemblies (6 per leg, threefold-symmetric) — construction issues, shown red in the app's connector view (C key).
    /// This test pins that number so any change — improvement, regression,
    /// or geometry drift — must come to our attention before building.
    #[test]
    fn test_open_claw_connector_collisions() {
        let mut executor = build_to_slack();
        executor.fabric.update_all_attachment_connections();
        apply_threefold_symmetry(&mut executor.fabric);

        let culprits = executor
            .fabric
            .connector
            .as_ref()
            .map(|connector| connector.culprits.len())
            .unwrap_or(0);
        assert_eq!(
            culprits, 18,
            "connector collision culprits changed (now {}): inspect in the app \
             (C toggles the connector view; culprits are red), then either fix \
             the slot assignment/hardware and update this baseline, or \
             investigate the geometry drift",
            culprits
        );
    }

    /// All 180 cables should group into 60 triples by 3-fold symmetry, with
    /// within-triple length spreads under 1 mm. (Apex-attached cables are
    /// allowed a small budget — three rotational copies converging on the
    /// single apex push must occupy three different slots, so their lengths
    /// differ by the ring step.)
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
            cables.push((a, b, interval.length(&fabric.joints).f32()));
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
        // Apex-attached cables are geometrically forced apart (the triples
        // touching the axis singletons `Z<n>` land on three slots of the
        // single apex push). Everything else should be sub-mm.
        let apex_triples = worst
            .iter()
            .filter(|(key, _, _, _)| key.contains("(Z") || key.contains(", Z"))
            .count();
        let non_apex_outliers = worst.len() - apex_triples;
        assert_eq!(
            non_apex_outliers, 0,
            "{} non-apex cable triples have spreads above {}mm",
            non_apex_outliers, TOLERANCE_MM,
        );
    }

    /// For each push group (rotationally-equivalent triple), assert that all
    /// three members agree on length and on the slot assignment at every
    /// cable end. Tolerance is 0.1 mm to allow tiny f32 drift.
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
                .map(|&k| fabric.intervals[k].length(&fabric.joints).f32())
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
                let conns_opt = fabric
                    .connector
                    .as_ref()
                    .and_then(|connector| connector.connections(push_key, end));
                if let Some(conns) = conns_opt {
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
