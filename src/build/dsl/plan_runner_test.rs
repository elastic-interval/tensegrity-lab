#[cfg(test)]
mod tests {
    use crate::build::dsl::fabric_library::{self, FabricName};
    use crate::build::dsl::fabric_plan_executor::{ExecutorStage, FabricPlanExecutor};
    use crate::units::{Unit, MM_PER_METER};

    const EXPECTED_GROUND_CONTACTS: usize = 3;
    const EXPECTED_HEIGHT_MM: f32 = 9119.0;
    const HEIGHT_TOLERANCE_PCT: f32 = 1.0;
    const EXPECTED_LANDING_SECONDS: f32 = 35.0;

    #[test]
    fn test_executor_phases() {
        eprintln!("\n=== Testing FabricPlanExecutor Phases ===\n");

        let plan = fabric_library::get_fabric_plan(FabricName::Triped);

        // Create executor
        let mut executor = FabricPlanExecutor::new_for_test(plan);
        let mut current_stage = ExecutorStage::Building;

        // Run until completion
        let mut iteration = 0;
        while !executor.is_complete() && iteration < 5_000_000 {
            // Log stage transitions
            let stage = executor.stage();
            if stage != &current_stage {
                eprintln!(
                    "[{:7}] Stage transition: {:?} -> {:?} (fabric age: {})",
                    iteration, current_stage, stage, executor.fabric.age
                );
                current_stage = stage.clone();
            }

            // Log periodic state
            if iteration % 20000 == 0 {
                let (min_y, max_y) = executor.fabric.altitude_range();
                let height_mm = (max_y - min_y) * MM_PER_METER;
                let radius = executor.fabric.bounding_radius();
                let ground_tolerance = 10.0 / MM_PER_METER;
                let ground_count = executor
                    .fabric
                    .joints
                    .values()
                    .filter(|j| j.location.y.abs() < ground_tolerance)
                    .count();
                eprintln!(
                    "[{:7}] {:?} | joints:{:3} height:{:8.3}mm radius:{:8.5}m ground:{} age:{}",
                    iteration,
                    stage,
                    executor.fabric.joints.len(),
                    height_mm,
                    radius,
                    ground_count,
                    executor.fabric.age
                );
            }

            // Do one iteration
            let _ = executor.iterate();
            iteration += 1;
        }

        eprintln!(
            "\n✓ Executor completed at iteration {} (fabric age: {})",
            iteration, executor.fabric.age
        );
        eprintln!("Final joints: {}", executor.fabric.joints.len());

        // Check that we reached completion
        assert!(executor.is_complete(), "Executor should have completed");
        assert_eq!(executor.stage(), &ExecutorStage::Complete);
    }

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

    #[test]
    fn test_triped_lands_on_three_feet() {
        let plan = fabric_library::get_fabric_plan(FabricName::Triped);
        let mut executor = FabricPlanExecutor::new_for_test(plan);

        let max_seconds = EXPECTED_LANDING_SECONDS * 2.0;
        let mut landed = false;

        while executor.fabric.age.as_duration().as_secs_f32() < max_seconds {
            let _ = executor.iterate();

            let fabric = &executor.fabric;
            let ground_tolerance = 10.0 / MM_PER_METER * fabric.scale().max(1.0);
            let ground_count = fabric
                .joints
                .values()
                .filter(|j| j.location.y.abs() < ground_tolerance)
                .count();

            if ground_count == EXPECTED_GROUND_CONTACTS {
                landed = true;
                let (min_y, max_y) = fabric.altitude_range();
                let height_mm = (max_y - min_y) * MM_PER_METER;
                let age = fabric.age.as_duration().as_secs_f32();

                eprintln!("Landed at {:.1}s with height {:.0}mm", age, height_mm);

                let height_diff_pct =
                    ((height_mm - EXPECTED_HEIGHT_MM) / EXPECTED_HEIGHT_MM * 100.0).abs();
                assert!(
                    height_diff_pct < HEIGHT_TOLERANCE_PCT,
                    "Height {:.1}mm differs from expected {:.1}mm by {:.1}%",
                    height_mm,
                    EXPECTED_HEIGHT_MM,
                    height_diff_pct
                );
                break;
            }
        }

        assert!(
            landed,
            "Triped failed to land on {} feet within {:.0}s",
            EXPECTED_GROUND_CONTACTS, max_seconds
        );
    }

    #[test]
    fn test_triped_full_execution() {
        eprintln!("\n=== Testing Triped Full Execution: BUILD → PRETENSE → CONVERGE ===\n");

        let plan = fabric_library::get_fabric_plan(FabricName::Triped);

        let mut executor = FabricPlanExecutor::new_for_test(plan);
        let mut current_stage = ExecutorStage::Building;
        let mut stage_entry_times: Vec<(ExecutorStage, usize)> = vec![(ExecutorStage::Building, 0)];

        eprintln!("Starting execution...");
        let mut iteration = 0;
        while !executor.is_complete() && iteration < 5_000_000 {
            let stage = executor.stage();

            // Track stage transitions
            if stage != &current_stage {
                let fabric_time = executor.fabric.age;
                eprintln!(
                    "[iter {:7}] Stage: {:?} -> {:?} (age: {})",
                    iteration, current_stage, stage, fabric_time
                );
                stage_entry_times.push((stage.clone(), iteration));
                current_stage = stage.clone();
            }

            let _ = executor.iterate();
            iteration += 1;
        }

        let (zero_g_ext, grav_ext) = executor.extension_counts();
        eprintln!(
            "\n✓ Execution completed at iteration {} (age: {})",
            iteration, executor.fabric.age
        );
        eprintln!(
            "Extension counts: zero-G={}, gravity={}",
            zero_g_ext, grav_ext
        );

        // Check for final stage transition to Complete
        let final_stage = executor.stage();
        if final_stage != &current_stage {
            stage_entry_times.push((final_stage.clone(), iteration));
        }

        // Verify all stages were reached
        let stages: Vec<_> = stage_entry_times.iter().map(|(s, _)| s).collect();
        assert!(
            stages.contains(&&ExecutorStage::Building),
            "Should have Building stage"
        );
        assert!(
            stages.contains(&&ExecutorStage::ZeroGPretensing),
            "Should have ZeroGPretensing stage"
        );
        assert!(
            stages.contains(&&ExecutorStage::Falling),
            "Should have Falling stage"
        );
        assert!(
            stages.contains(&&ExecutorStage::Settling),
            "Should have Settling stage"
        );
        assert!(
            stages.contains(&&ExecutorStage::Complete),
            "Should have Complete stage"
        );

        // Print execution log
        executor.print_log();
    }

    #[test]
    fn check_final_settled_state() {
        eprintln!("\n=== Checking Final Settled State ===\n");

        let plan = fabric_library::get_fabric_plan(FabricName::Triped);

        let mut executor = FabricPlanExecutor::new_for_test(plan);

        // Run until completion
        let mut iteration = 0;
        while !executor.is_complete() && iteration < 5_000_000 {
            let _ = executor.iterate();
            iteration += 1;
        }

        // Check final state
        let (min_y, max_y) = executor.fabric.altitude_range();
        let height_mm = (max_y - min_y) * MM_PER_METER;
        let radius = executor.fabric.bounding_radius();
        let ground_tolerance = 10.0 / MM_PER_METER;
        let ground_count = executor
            .fabric
            .joints
            .values()
            .filter(|j| j.location.y.abs() < ground_tolerance)
            .count();

        let centroid = executor.fabric.centroid();
        eprintln!("\n=== FINAL CONVERGED STATE ===");
        eprintln!(
            "Centroid: ({:.4}, {:.4}, {:.4})",
            centroid.x, centroid.y, centroid.z
        );
        eprintln!("Height: {:.1}mm ({:.2}m)", height_mm, height_mm / 1000.0);
        eprintln!("Radius: {:.3}m", radius);
        eprintln!("Ground contacts: {}", ground_count);
        eprintln!("Total joints: {}", executor.fabric.joints.len());
        eprintln!("Iterations: {}", iteration);

        // Analyze joints by path depth for symmetry
        use std::collections::HashMap;
        eprintln!("\n=== JOINT PATH DEPTH ANALYSIS ===");
        let mut by_depth: HashMap<usize, usize> = HashMap::new();
        for joint in executor.fabric.joints.values() {
            *by_depth.entry(joint.path.depth()).or_insert(0) += 1;
        }
        let mut depth_counts: Vec<_> = by_depth.into_iter().collect();
        depth_counts.sort_by_key(|(depth, _)| *depth);
        eprintln!(
            "Joints grouped by path depth ({} unique depths):",
            depth_counts.len()
        );
        for (depth, count) in &depth_counts {
            // Show if count is divisible by 3 (symmetric across Triped's 3 legs)
            let sym = if count % 3 == 0 {
                format!("({}×3)", count / 3)
            } else {
                "".to_string()
            };
            eprintln!("  depth {} : {} joints {}", depth, count, sym);
        }

        // Analyze push intervals grouped by their symmetric key
        use crate::fabric::interval::Role;
        use crate::fabric::interval::Span;
        eprintln!("\n=== PUSH INTERVALS BY SYMMETRIC KEY ===");
        let mut push_by_key: HashMap<(usize, u8), usize> = HashMap::new();
        for interval in executor.fabric.intervals.values() {
            if interval.has_role(Role::Pushing) {
                let alpha = &executor.fabric.joints[interval.alpha_key];
                let omega = &executor.fabric.joints[interval.omega_key];
                // Push intervals have both joints with same symmetric key
                assert_eq!(
                    alpha.path.symmetric_key(),
                    omega.path.symmetric_key(),
                    "Push interval joints should have same symmetric key"
                );
                *push_by_key.entry(alpha.path.symmetric_key()).or_insert(0) += 1;
            }
        }
        let mut push_key_counts: Vec<_> = push_by_key.into_iter().collect();
        push_key_counts.sort_by_key(|(key, _)| *key);
        eprintln!(
            "Push intervals grouped by symmetric key ({} groups):",
            push_key_counts.len()
        );
        for ((depth, axis), count) in &push_key_counts {
            let sym = if count % 3 == 0 {
                format!("({}×3)", count / 3)
            } else {
                "".to_string()
            };
            eprintln!(
                "  (depth={}, axis={}) : {} pushes {}",
                depth, axis, count, sym
            );
        }

        eprintln!("\n=== PUSH INTERVAL LENGTH ANALYSIS ===");
        let mut push_lengths: Vec<f32> = Vec::new(); // length in mm
        for interval in executor.fabric.intervals.values() {
            if interval.has_role(Role::Pushing) {
                if let Span::Fixed { length } = interval.span {
                    push_lengths.push(length.f32() * 1000.0);
                }
            }
        }
        push_lengths.sort_by(|a, b| a.partial_cmp(b).unwrap());

        if !push_lengths.is_empty() {
            let min_length = push_lengths.iter().fold(f32::INFINITY, |a, &b| a.min(b));
            let max_length = push_lengths
                .iter()
                .fold(f32::NEG_INFINITY, |a, &b| a.max(b));
            eprintln!("Push intervals: {}", push_lengths.len());
            eprintln!("Length range: {:.0}mm to {:.0}mm", min_length, max_length);
            eprintln!("\nShortest 5:");
            for length in push_lengths.iter().take(5) {
                eprintln!("  {:.0}mm", length);
            }
            eprintln!("\nLongest 5:");
            for length in push_lengths.iter().rev().take(5) {
                eprintln!("  {:.0}mm", length);
            }
        }

        // Report the state
        eprintln!("\n✓ Execution completed successfully");
    }

}
