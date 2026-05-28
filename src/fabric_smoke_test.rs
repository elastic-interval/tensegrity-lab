#[cfg(test)]
mod tests {
    use crate::build::dsl::fabric_library::{self, FabricName};
    use crate::build::dsl::fabric_plan_executor::{ExecutorStage, FabricPlanExecutor};
    use glam::Vec3;
    use strum::IntoEnumIterator;

    /// Every named fabric should build to completion AND run all configured
    /// shape, pretense, fall, settle phases through to Complete. The cap is
    /// generous (5M iterations ≈ 250s fabric time); anything slower than that
    /// almost certainly indicates a stuck phase (e.g. stale approaching count).
    #[test]
    fn test_all_fabrics_build() {
        use crate::build::dsl::fabric_plan_executor::IterateResult;
        const MAX_ITERS: u64 = 5_000_000;
        for fabric_name in FabricName::iter() {
            let plan = fabric_library::get_fabric_plan(fabric_name);
            let mut executor = FabricPlanExecutor::new(plan);
            let mut completed = false;
            for _ in 0..MAX_ITERS {
                if matches!(executor.iterate(), IterateResult::Complete) {
                    completed = true;
                    break;
                }
            }
            let fabric = &executor.fabric;
            assert!(!fabric.joints.is_empty(), "{fabric_name}: built with zero joints");
            assert!(!fabric.intervals.is_empty(), "{fabric_name}: built with zero intervals");
            assert!(completed,
                "{fabric_name}: plan did not reach Complete within {MAX_ITERS} iterations (stuck at {:?})",
                executor.stage());
        }
    }

/// Per-step full symmetry check: at every build step (starting from
    /// the seed alone), verify that the joints in the fabric form a
    /// mirror-symmetric set — for every joint off the mirror plane,
    /// there should exist another joint at the mirror-image position.
    /// Reports how many steps pass before symmetry breaks.
    ///
    /// Uses the +X axis as the candidate mirror plane (X=0 plane).
    #[test]
    fn test_headless_hug_position_symmetry_per_build_step() {
        use crate::build::dsl::build_phase::{assign_axial_labels, BuildPhase};
        use crate::fabric::Fabric;

        let plan = fabric_library::get_fabric_plan(FabricName::HeadlessHug);
        let mut fabric =
            Fabric::new("HeadlessHug".into()).with_dimensions(plan.dimensions);
        let mut build_phase: BuildPhase = plan.build_phase;

        // Step 0: seed only
        build_phase.init(&mut fabric);
        assign_axial_labels(&mut fabric);
        check_position_symmetry(&fabric, 0);

        let mut step = 0;
        while build_phase.is_building() {
            step += 1;
            build_phase.build_step(&mut fabric);
            assign_axial_labels(&mut fabric);
            check_position_symmetry(&fabric, step);
        }
        eprintln!("All {step} build steps preserved position symmetry");
    }

    fn check_position_symmetry(fabric: &crate::fabric::Fabric, step: usize) {
        let labeled: Vec<(String, Vec3)> = fabric
            .joints
            .iter()
            .filter_map(|(k, j)| j.label.map(|_| (fabric.joint_label(k), j.location)))
            .collect();

        // Try X-flip, Y-flip, Z-flip; choose the axis with the most partners.
        let tol = 0.05;
        let count_partners = |flip: fn(Vec3) -> Vec3| -> usize {
            labeled
                .iter()
                .filter(|(_, loc)| {
                    let target = flip(*loc);
                    (*loc - target).length() > tol
                        && labeled
                            .iter()
                            .any(|(_, l)| (*l - target).length() < tol)
                })
                .count()
        };
        let off_plane = |flip: fn(Vec3) -> Vec3| -> usize {
            labeled
                .iter()
                .filter(|(_, loc)| (*loc - flip(*loc)).length() > tol)
                .count()
        };
        let candidates: &[(&str, fn(Vec3) -> Vec3)] = &[
            ("X=0", |v| Vec3::new(-v.x, v.y, v.z)),
            ("Y=0", |v| Vec3::new(v.x, -v.y, v.z)),
            ("Z=0", |v| Vec3::new(v.x, v.y, -v.z)),
        ];
        let (name, flip) = candidates
            .iter()
            .max_by_key(|(_, f)| count_partners(*f))
            .copied()
            .unwrap();
        let matched = count_partners(flip);
        let off = off_plane(flip);
        let on_plane = labeled.len() - off;

        let unmatched: Vec<&(String, Vec3)> = labeled
            .iter()
            .filter(|(_, loc)| {
                let target = flip(*loc);
                (*loc - target).length() > tol
                    && !labeled.iter().any(|(_, l)| (*l - target).length() < tol)
            })
            .collect();

        eprintln!(
            "step {step:>2}: {} labeled joints, plane {} → matched {}/{} off-plane, {} on-plane",
            labeled.len(),
            name,
            matched,
            off,
            on_plane,
        );

        if !unmatched.is_empty() {
            for (label, loc) in unmatched.iter().take(8) {
                eprintln!(
                    "  ✗ {label} ({:+.3}, {:+.3}, {:+.3}) has no mirror partner",
                    loc.x, loc.y, loc.z
                );
            }
            panic!(
                "step {step}: {}/{} off-plane joints have no mirror partner (plane {name})",
                unmatched.len(),
                off
            );
        }
    }

    /// Walk HeadlessHug's build phase step-by-step, checking after each
    /// step that the labels generated so far are mirror-partner-consistent.
    /// Panics IMMEDIATELY at the first step that violates symmetry, with
    /// the step number and the offending labels. Pinpoints exactly where
    /// the partnership breaks.
    #[test]
    fn test_headless_hug_symmetry_per_build_step() {
        use crate::build::dsl::build_phase::{assign_axial_labels, BuildPhase};
        use crate::fabric::Fabric;

        let plan = fabric_library::get_fabric_plan(FabricName::HeadlessHug);
        let mut fabric = Fabric::new("HeadlessHug".into()).with_dimensions(plan.dimensions);
        let mut build_phase: BuildPhase = plan.build_phase;
        build_phase.init(&mut fabric);

        let mut step = 0;
        loop {
            // Build one step (or stop if done)
            if !build_phase.is_building() {
                break;
            }
            step += 1;
            build_phase.build_step(&mut fabric);
            assign_axial_labels(&mut fabric);

            // Collect labels created so far
            let labeled: Vec<(String, Vec3)> = fabric
                .joints
                .iter()
                .filter_map(|(k, j)| j.label.map(|_| (fabric.joint_label(k), j.location)))
                .collect();

            // For each labeled joint with first char in {A,B,C,D}, the same
            // suffix should exist with the paired letter (A↔B, C↔D).
            let partner_letter = |c: char| match c {
                'A' => Some('B'), 'B' => Some('A'),
                'C' => Some('D'), 'D' => Some('C'),
                _ => None,
            };

            for (label, _loc) in &labeled {
                let Some(first) = label.chars().next() else { continue };
                let Some(other_first) = partner_letter(first) else { continue };
                // Iterate only A→ and C→ to avoid double reporting
                if !(first == 'A' || first == 'C') {
                    continue;
                }
                let partner_label = format!("{other_first}{}", &label[1..]);
                let has_partner = labeled.iter().any(|(l, _)| l == &partner_label);
                if !has_partner {
                    let total_a_or_c = labeled
                        .iter()
                        .filter(|(l, _)| l.starts_with(first))
                        .count();
                    let total_partner = labeled
                        .iter()
                        .filter(|(l, _)| l.starts_with(other_first))
                        .count();
                    panic!(
                        "Step {step}: label {label} has no partner {partner_label}.\n\
                         Labels with '{first}' so far: {total_a_or_c}\n\
                         Labels with '{other_first}' so far: {total_partner}\n\
                         (label generation is asymmetric at this build step)"
                    );
                }
            }

            eprintln!(
                "step {step}: {} labeled joints, all partnered ✓",
                labeled.len()
            );
        }

        eprintln!("All {step} build steps preserved label partnership.");
    }

    /// Verify the two SingleTwist orientations — `OnSpin(Left)` (direct)
    /// and `OnSpin(Right)` (mirrored via `BakedBrick::mirror`) — are TRUE
    /// mirror images at each local_index, across some axis-plane (X=0,
    /// Y=0, or Z=0). If this fails, SingleTwist's mirror is broken at
    /// the brick level (independent of any seed-face issues).
    #[test]
    fn test_single_twist_mirror_preserves_local_index_partnership() {
        use crate::build::dsl::brick_dsl::{BrickName, BrickRole};
        use crate::build::dsl::brick_library::get_brick;
        use crate::build::dsl::Spin;

        let left = get_brick(BrickName::SingleTwistLeft, BrickRole::OnSpin(Spin::Left));
        let right = get_brick(BrickName::SingleTwistLeft, BrickRole::OnSpin(Spin::Right));
        assert_eq!(
            left.joints.len(),
            right.joints.len(),
            "joint counts differ"
        );
        let n = left.joints.len();

        // Pick the mirror axis that fits the most pairs.
        let tol = 1e-3;
        let count_matches = |flip: fn(Vec3) -> Vec3| {
            (0..n)
                .filter(|&i| (flip(left.joints[i].location) - right.joints[i].location).length() < tol)
                .count()
        };
        let candidates: &[(&str, fn(Vec3) -> Vec3)] = &[
            ("x→-x", |v| Vec3::new(-v.x, v.y, v.z)),
            ("y→-y", |v| Vec3::new(v.x, -v.y, v.z)),
            ("z→-z", |v| Vec3::new(v.x, v.y, -v.z)),
        ];
        let (axis_name, flip) = candidates
            .iter()
            .max_by_key(|(_, f)| count_matches(*f))
            .copied()
            .unwrap();
        let matched = count_matches(flip);
        eprintln!("best mirror axis: {axis_name} matches {matched}/{n} joint pairs");
        for i in 0..n {
            eprintln!(
                "  i={i}  left={:?}  right={:?}  expect-right={:?}",
                left.joints[i].location,
                right.joints[i].location,
                flip(left.joints[i].location),
            );
        }
        assert_eq!(matched, n, "only {matched}/{n} joint pairs are mirror-related under {axis_name}");
    }

/// HeadlessHug should label its 4 limbs as A, B, C, D such that
    /// A↔B and C↔D are mirror pairs of joints. For every label of letter
    /// A, the SAME suffix should also appear with letter B (and likewise
    /// C↔D). This is the visible "partner" relationship the user is
    /// driving toward — labels of partner joints differ ONLY in the
    /// first character.
    ///
    /// The test grounds this in geometry: each (A, B-suffix) pair must
    /// also correspond to two joints at mirror-image positions in the
    /// fabric. If the labels exist but the positions aren't mirror-related,
    /// the labelling is incorrectly pairing them.
    #[test]
    fn test_headless_hug_mirror_partner_labels_match() {
        let plan = fabric_library::get_fabric_plan(FabricName::HeadlessHug);
        let mut executor = FabricPlanExecutor::new(plan);
        while *executor.stage() == ExecutorStage::Building {
            let _ = executor.iterate();
        }
        let fabric = &executor.fabric;

        let labeled: Vec<(String, Vec3)> = fabric
            .joints
            .iter()
            .filter_map(|(k, j)| j.label.map(|_| (fabric.joint_label(k), j.location)))
            .collect();

        // For every label with first char in {A,B,C,D}, the same suffix
        // should exist with the paired letter (A↔B, C↔D), and the two
        // joints should be at fabric mirror-image positions.
        let partner_letter = |c: char| match c {
            'A' => Some('B'), 'B' => Some('A'),
            'C' => Some('D'), 'D' => Some('C'),
            _ => None,
        };
        let mut missing: Vec<String> = Vec::new();
        let mut non_mirror: Vec<String> = Vec::new();
        let pos_tol = 0.05;

        for (label, loc) in &labeled {
            let Some(first) = label.chars().next() else { continue };
            let Some(other_first) = partner_letter(first) else { continue };
            // Only iterate one direction (A → B, C → D) to avoid duplicate reports
            if !(first == 'A' || first == 'C') {
                continue;
            }
            let partner_label = format!("{other_first}{}", &label[1..]);
            let partner = labeled.iter().find(|(l, _)| l == &partner_label);
            match partner {
                Some((_, partner_loc)) => {
                    // Verify physical partnership: same Y, same Z, opposite X (try a few axes).
                    let candidates = [
                        Vec3::new(-loc.x, loc.y, loc.z),
                        Vec3::new(loc.x, loc.y, -loc.z),
                        Vec3::new(-loc.x, loc.y, -loc.z),
                    ];
                    let is_mirror_partner = candidates
                        .iter()
                        .any(|target| (*partner_loc - *target).length() < pos_tol);
                    if !is_mirror_partner {
                        non_mirror.push(format!(
                            "{label} {loc:?} ↔ {partner_label} {partner_loc:?}: not at any axis-mirror image"
                        ));
                    }
                }
                None => {
                    missing.push(format!("{label} has no matching {partner_label}"));
                }
            }
        }

        if !missing.is_empty() || !non_mirror.is_empty() {
            if !missing.is_empty() {
                eprintln!("=== missing partner labels ({}): ===", missing.len());
                for m in missing.iter().take(15) { eprintln!("  ✗ {m}"); }
            }
            if !non_mirror.is_empty() {
                eprintln!("=== partner labels exist but not mirror-positioned ({}): ===", non_mirror.len());
                for m in non_mirror.iter().take(15) { eprintln!("  ✗ {m}"); }
            }
            panic!(
                "{} missing partner labels, {} mirror-position failures",
                missing.len(),
                non_mirror.len()
            );
        }
    }

}
