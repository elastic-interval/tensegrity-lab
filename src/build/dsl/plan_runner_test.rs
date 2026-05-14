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
            let mut executor = FabricPlanExecutor::new_for_test(plan);
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

    #[test]
    fn test_open_claw_foot_positions() {
        let plan = fabric_library::get_fabric_plan(FabricName::OpenClaw);
        let mut executor = FabricPlanExecutor::new_for_test(plan);
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
