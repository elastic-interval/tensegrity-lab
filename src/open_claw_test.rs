#[cfg(test)]
mod tests {
    use crate::build::dsl::fabric_library::{self, FabricName};
    use crate::build::dsl::fabric_plan_executor::FabricPlanExecutor;
    use crate::units::{Unit, MM_PER_METER};

    const EXPECTED_GROUND_CONTACTS: usize = 3;

    fn find_scale_for_6m_base(fabric_name: FabricName) {
        use crate::fabric::interval::Role;
        use crate::units::Meters;

        const TARGET_EDGE_MM: f32 = 6000.0;
        const TOLERANCE_PCT: f32 = 2.0;
        const MAX_ROUNDS: usize = 5;

        let base_plan = fabric_library::get_fabric_plan(fabric_name);
        let mut scale = base_plan.dimensions.scale;

        for round in 0..MAX_ROUNDS {
            let mut plan = base_plan.clone();
            plan.dimensions.scale = scale;
            let mut executor = FabricPlanExecutor::new(plan);
            while !executor.is_complete() {
                let _ = executor.iterate();
            }
            let fabric = &executor.fabric;

            // Find ground contacts
            let ground_tolerance = 10.0 / MM_PER_METER * fabric.scale().max(1.0);
            let ground_joints: Vec<_> = fabric
                .joints
                .values()
                .filter(|j| j.location.y.abs() < ground_tolerance)
                .collect();
            assert_eq!(
                ground_joints.len(),
                EXPECTED_GROUND_CONTACTS,
                "{fabric_name}: expected 3 feet, got {}",
                ground_joints.len()
            );

            // Measure base triangle edges
            let mut edge_lengths_mm = Vec::new();
            for i in 0..ground_joints.len() {
                for k in (i + 1)..ground_joints.len() {
                    let d = (ground_joints[i].location - ground_joints[k].location).length();
                    edge_lengths_mm.push(d * MM_PER_METER);
                }
            }
            assert_eq!(edge_lengths_mm.len(), 3);
            let avg_edge_mm: f32 = edge_lengths_mm.iter().sum::<f32>() / 3.0;

            // Check equilateral
            for (i, &edge) in edge_lengths_mm.iter().enumerate() {
                let diff_pct = ((edge - avg_edge_mm) / avg_edge_mm * 100.0).abs();
                assert!(
                    diff_pct < TOLERANCE_PCT,
                    "{fabric_name}: edge {i} ({edge:.0}mm) differs from avg ({avg_edge_mm:.0}mm) by {diff_pct:.1}%"
                );
            }

            let edge_diff_pct =
                ((avg_edge_mm - TARGET_EDGE_MM) / TARGET_EDGE_MM * 100.0).abs();

            // Report
            let (min_y, max_y) = fabric.altitude_range();
            let height_mm = (max_y - min_y) * MM_PER_METER;
            let max_push_mm = fabric
                .intervals
                .values()
                .filter(|iv| iv.has_role(Role::Pushing))
                .map(|iv| iv.length(&fabric.joints) * MM_PER_METER)
                .fold(0.0_f32, f32::max);
            eprintln!(
                "{fabric_name} round {round}: scale={:.4}, edges={:.0},{:.0},{:.0}mm, avg={avg_edge_mm:.0}mm ({edge_diff_pct:.1}% from 6m), height={height_mm:.0}mm, max strut={max_push_mm:.0}mm",
                scale.f32(), edge_lengths_mm[0], edge_lengths_mm[1], edge_lengths_mm[2]
            );

            if edge_diff_pct < TOLERANCE_PCT {
                eprintln!("{fabric_name}: CONVERGED at scale={:.4}", scale.f32());
                return;
            }

            // Adjust scale proportionally
            scale = Meters(scale.f32() * TARGET_EDGE_MM / avg_edge_mm);
        }
        panic!("{fabric_name}: failed to converge to 6m base after {MAX_ROUNDS} rounds");
    }

    #[test]
    fn test_open_claw_base_triangle() {
        find_scale_for_6m_base(FabricName::OpenClaw);
    }

    /// Lock the per-magnitude hinge-bend count distribution to the inventory
    /// already ordered from the factory. Source of truth: the CSV at
    /// `docs/open-claw-2026-05-14b.csv` reported
    /// `Bend counts (per magnitude): 12°×80  30°×100  49°×120  68°×60`.
    ///
    /// A small tolerance is allowed per magnitude because the CSV is captured
    /// at a slightly-different fabric moment than this test (the interactive
    /// binary may run a handful of extra pretensing ticks between the `Slack`
    /// broadcast and the event-handler that writes the CSV). Cables whose
    /// ideal bend sits near a snap boundary can flip between adjacent
    /// magnitudes across these tiny geometric shifts. The factory order
    /// includes spare pieces of each angle to absorb a few-cable mismatch.
    ///
    /// The test fires when the drift exceeds the spare buffer — any larger
    /// shift would indicate a code or parameter change that risks shipping
    /// a CSV the manufactured inventory can't cover.
    #[test]
    fn test_open_claw_bend_counts_match_factory_inventory() {
        use crate::build::dsl::fabric_plan_executor::ExecutorStage;
        use crate::fabric::bend_optimizer::snap_to_magnitudes;

        const FACTORY_COUNTS: [usize; 4] = [80, 100, 120, 60]; // 12°, 30°, 49°, 68°
        const TOLERANCE: usize = 5;

        let plan = fabric_library::get_fabric_plan(FabricName::OpenClaw);
        let mut executor = FabricPlanExecutor::new(plan);
        // Iterate only through Building; the slack moment is broadcast when
        // it transitions out (inside `transition_to_pretense`).
        while *executor.stage() == ExecutorStage::Building {
            let _ = executor.iterate();
        }
        // Mirror what snapshot_csv does before counting.
        executor.fabric.update_all_attachment_connections();
        executor.fabric.recompute_bend_magnitudes();

        let fabric = &executor.fabric;
        let mags = &fabric.dimensions.hinge.bend_magnitudes;
        assert_eq!(
            mags.as_slice(),
            &[12.0, 30.0, 49.0, 68.0],
            "locked bend magnitudes have drifted from the factory inventory"
        );

        let ideals = fabric.collect_ideal_bend_angles();
        let snapped: Vec<f32> = ideals
            .iter()
            .map(|&x| snap_to_magnitudes(x, mags).0)
            .collect();
        assert_eq!(snapped.len(), 360, "cable-end count drifted");

        let count_for = |m: f32| -> usize {
            snapped
                .iter()
                .filter(|s| (s.abs() - m).abs() < 0.5)
                .count()
        };

        let actual = [
            count_for(12.0),
            count_for(30.0),
            count_for(49.0),
            count_for(68.0),
        ];
        eprintln!(
            "Bend counts: 12°×{}  30°×{}  49°×{}  68°×{}  (factory {}/{}/{}/{}, tolerance ±{})",
            actual[0], actual[1], actual[2], actual[3],
            FACTORY_COUNTS[0], FACTORY_COUNTS[1], FACTORY_COUNTS[2], FACTORY_COUNTS[3],
            TOLERANCE,
        );
        assert_eq!(actual.iter().sum::<usize>(), 360, "totals drifted from 360");

        let labels = [12, 30, 49, 68];
        for i in 0..4 {
            let diff = (actual[i] as isize - FACTORY_COUNTS[i] as isize).unsigned_abs();
            assert!(
                diff <= TOLERANCE,
                "{}° count {} differs from factory {} by {} (tolerance {})",
                labels[i], actual[i], FACTORY_COUNTS[i], diff, TOLERANCE,
            );
        }
    }

    #[test]
    fn test_open_claw_foot_positions() {
        let plan = fabric_library::get_fabric_plan(FabricName::OpenClaw);
        let mut executor = FabricPlanExecutor::new(plan);
        while !executor.is_complete() {
            let _ = executor.iterate();
        }
        let fabric = &executor.fabric;
        let ground_tolerance = 10.0 / MM_PER_METER * fabric.scale().max(1.0);
        let mut feet: Vec<_> = fabric
            .joints
            .values()
            .filter(|j| j.location.y.abs() < ground_tolerance)
            .map(|j| j.location)
            .collect();
        assert_eq!(feet.len(), 3, "expected 3 ground contacts");
        // Sort by angle from centroid for consistent ordering
        let centroid = feet.iter().copied().sum::<glam::Vec3>() / 3.0;
        feet.sort_by(|a, b| {
            let aa = (a.z - centroid.z).atan2(a.x - centroid.x);
            let ba = (b.z - centroid.z).atan2(b.x - centroid.x);
            aa.partial_cmp(&ba).unwrap()
        });
        // Print in CSV space (Z-up): sim (x, y, z) → csv (x, -z, y)
        // Positions in meters for Blender
        eprintln!("\n=== OpenClaw foot positions (Blender Z-up, meters) ===");
        eprintln!("TOWER_POSITIONS = [");
        for (i, foot) in feet.iter().enumerate() {
            eprintln!(
                "    ({:.4}, {:.4}, {:.4}),  # foot {}",
                foot.x, -foot.z, foot.y, i
            );
        }
        eprintln!("]");
        // Verify edge lengths
        for i in 0..3 {
            let j = (i + 1) % 3;
            let d = (feet[i] - feet[j]).length() * MM_PER_METER;
            eprintln!("edge {}-{}: {:.0}mm", i, j, d);
        }
    }
}
