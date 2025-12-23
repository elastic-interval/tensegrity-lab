#[cfg(test)]
mod tests {
    use crate::build::dsl::fabric_library::{self, FabricName};
    use crate::build::dsl::fabric_plan_executor::{ExecutorStage, FabricPlanExecutor};
    use crate::fabric::Fabric;
    use crate::units::MM_PER_METER;

    /// Check RECAPTURE env var to capture new benchmark values without asserting.
    /// Run: RECAPTURE=1 cargo test test_all_build_benchmarks -- --nocapture
    /// Then copy the printed Benchmark lines into ui_benchmarks().
    fn recapture_mode() -> bool {
        std::env::var("RECAPTURE").is_ok()
    }

    /// Check GROUND_ONLY env var to only check ground contacts instead of full benchmarks.
    /// Use when experimenting with different altitude/scale values.
    /// Run: GROUND_ONLY=1 cargo test test_all_build_benchmarks -- --nocapture
    fn ground_only_mode() -> bool {
        std::env::var("GROUND_ONLY").is_ok()
    }

    /// Benchmark data point from UI reference run
    #[derive(Debug)]
    struct Benchmark {
        age: f32,
        joints: usize,
        height_mm: f32,
        radius: f32,
        ground: usize,
    }

    /// Executor benchmarks - age is fabric.age (scaled by physics.time_scale)
    /// Updated for Triped with scale(M(1.03)), coordinates in meters directly
    /// Recaptured after 3x faster pretensing (PRETENSE_STEP_SECONDS = 0.067)
    fn ui_benchmarks() -> Vec<Benchmark> {
        vec![
            Benchmark {
                age: 0.0,
                joints: 0,
                height_mm: 0.0,
                radius: 0.000,
                ground: 0,
            },
            // BUILD phase (fabric age 0-6s)
            Benchmark {
                age: 2.0,
                joints: 172,
                height_mm: 6470.1,
                radius: 12.246,
                ground: 0,
            },
            Benchmark {
                age: 4.0,
                joints: 172,
                height_mm: 10989.7,
                radius: 8.315,
                ground: 0,
            },
            Benchmark {
                age: 6.0,
                joints: 172,
                height_mm: 9297.3,
                radius: 6.042,
                ground: 0,
            },
            // PRETENSE phase with holistic pretensing (fabric age ~6-20s)
            Benchmark {
                age: 10.0,
                joints: 165,
                height_mm: 9177.9,
                radius: 6.039,
                ground: 0,
            },
            Benchmark {
                age: 15.0,
                joints: 165,
                height_mm: 9209.6,
                radius: 6.039,
                ground: 0,
            },
            Benchmark {
                age: 20.0,
                joints: 165,
                height_mm: 9237.4,
                radius: 6.039,
                ground: 0,
            },
            // FALL/SETTLE phase - structure lands on 3 joints (fabric age ~20-33s)
            Benchmark {
                age: 25.0,
                joints: 165,
                height_mm: 9208.2,
                radius: 7.162,
                ground: 3,
            },
            Benchmark {
                age: 30.0,
                joints: 165,
                height_mm: 9210.8,
                radius: 7.162,
                ground: 3,
            },
            Benchmark {
                age: 33.0,
                joints: 165,
                height_mm: 9210.4,
                radius: 7.162,
                ground: 3,
            },
        ]
    }

    /// Check if fabric state matches benchmark (with tolerance).
    /// If recapture_mode() is true, only prints values without asserting.
    /// If ground_only_mode() is true, only checks ground contacts (for experimental scales).
    fn check_benchmark(fabric: &Fabric, benchmark: &Benchmark, tolerance_pct: f32) {
        let fabric_age = fabric.age.as_duration().as_secs_f32();
        let bounding_radius = fabric.bounding_radius();
        let (min_y, max_y) = fabric.altitude_range();
        let height_mm = (max_y - min_y) * MM_PER_METER;
        // Scale ground tolerance with fabric scale for small structures
        let ground_tolerance = 10.0 / MM_PER_METER * fabric.scale().max(1.0);
        let ground_count = fabric
            .joints
            .values()
            .filter(|j| j.location.y.abs() < ground_tolerance)
            .count();

        // Print benchmark format for easy copy-paste (matches rustfmt style)
        eprintln!(
            "            Benchmark {{\n                age: {:.1},\n                joints: {},\n                height_mm: {:.1},\n                radius: {:.3},\n                ground: {},\n            }},",
            fabric_age,
            fabric.joints.len(),
            height_mm,
            bounding_radius,
            ground_count
        );

        // Skip assertions when recapturing
        if recapture_mode() {
            return;
        }

        // When ground_only_mode is true, only check ground contacts at end
        if ground_only_mode() {
            if benchmark.ground > 0 {
                assert_eq!(
                    ground_count, benchmark.ground,
                    "At age {:.1}s: Expected {} ground contacts, got {}",
                    fabric_age, benchmark.ground, ground_count
                );
            }
            return;
        }

        // Full benchmark checking (for normal altitude 7.5M, scale 1.03M)

        // Check joints (exact)
        assert_eq!(
            fabric.joints.len(),
            benchmark.joints,
            "At age {:.1}s: Expected {} joints, got {}",
            fabric_age,
            benchmark.joints,
            fabric.joints.len()
        );

        // Check height (with tolerance)
        if benchmark.height_mm > 0.0 {
            let height_diff_pct =
                ((height_mm - benchmark.height_mm) / benchmark.height_mm * 100.0).abs();
            assert!(
                height_diff_pct < tolerance_pct,
                "At age {:.1}s: Height {:.1}mm differs from benchmark {:.1}mm by {:.1}% (tolerance: {:.1}%)",
                fabric_age, height_mm, benchmark.height_mm, height_diff_pct, tolerance_pct
            );
        }

        // Check radius (with tolerance)
        if benchmark.radius > 0.0 {
            let radius_diff_pct =
                ((bounding_radius - benchmark.radius) / benchmark.radius * 100.0).abs();
            assert!(
                radius_diff_pct < tolerance_pct,
                "At age {:.1}s: Radius {:.3}m differs from benchmark {:.3}m by {:.1}% (tolerance: {:.1}%)",
                fabric_age, bounding_radius, benchmark.radius, radius_diff_pct, tolerance_pct
            );
        }

        // Check ground contacts at end
        if benchmark.ground > 0 {
            assert_eq!(
                ground_count, benchmark.ground,
                "At age {:.1}s: Expected {} ground contacts, got {}",
                fabric_age, benchmark.ground, ground_count
            );
        }
    }

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

    #[test]
    fn test_all_build_benchmarks() {
        eprintln!("\n=== Testing All Phase Benchmarks ===\n");

        let plan = fabric_library::get_fabric_plan(FabricName::Triped);

        let mut executor = FabricPlanExecutor::new_for_test(plan);

        let benchmarks = ui_benchmarks();
        let mut benchmark_idx = 0;

        eprintln!(
            "Running through all phases, checking {} benchmarks...\n",
            benchmarks.len()
        );

        let max_iterations = (210.0 * 4000.0) as usize;

        // Check age 0 before running any iterations
        if benchmark_idx < benchmarks.len() {
            let benchmark = &benchmarks[benchmark_idx];
            if benchmark.age == 0.0 {
                check_benchmark(&executor.fabric, benchmark, 0.1);
                benchmark_idx += 1;
            }
        }

        // Run executor iteration by iteration, checking benchmarks by fabric age
        for _ in 1..=max_iterations {
            let _ = executor.iterate();

            // Check if we've reached a benchmark age
            if benchmark_idx < benchmarks.len() {
                let benchmark = &benchmarks[benchmark_idx];
                let fabric_age = executor.fabric.age.as_duration().as_secs_f32();

                if fabric_age >= benchmark.age {
                    check_benchmark(&executor.fabric, benchmark, 5.0);
                    benchmark_idx += 1;
                }
            }

            // Stop if all benchmarks checked
            if benchmark_idx >= benchmarks.len() {
                break;
            }
        }

        eprintln!("\n✓ Checked {} benchmarks successfully!", benchmark_idx);

        executor.print_log();

        if ground_only_mode() {
            // In experimental mode, we only care that ground contacts were checked
            // (fabric may freeze at unusual scales, which is expected)
            eprintln!("(ground_only_mode - skipping full benchmark count assertion)");
        } else {
            assert!(
                benchmark_idx >= benchmarks.len(),
                "Only checked {} of {} benchmarks",
                benchmark_idx,
                benchmarks.len()
            );
        }
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

        eprintln!(
            "\n✓ Execution completed at iteration {} (age: {})",
            iteration, executor.fabric.age
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
            stages.contains(&&ExecutorStage::Pretensing),
            "Should have Pretensing stage"
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

        eprintln!("\n=== FINAL CONVERGED STATE ===");
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
                    alpha.path.symmetric_key(), omega.path.symmetric_key(),
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
            eprintln!("  (depth={}, axis={}) : {} pushes {}", depth, axis, count, sym);
        }

        eprintln!("\n=== PUSH INTERVAL STRAIN ANALYSIS ===");
        let mut push_strains: Vec<(f32, f32, f32)> = Vec::new(); // (rest_length_mm, target_length_mm, strain%)
        for interval in executor.fabric.intervals.values() {
            if interval.has_role(Role::Pushing) {
                if let Span::Pretensing {
                    rest_length,
                    target_length,
                    ..
                } = interval.span
                {
                    let extension = target_length - rest_length;
                    let strain_pct = (extension / rest_length) * 100.0;
                    push_strains.push((rest_length * 1000.0, target_length * 1000.0, strain_pct));
                }
            }
        }
        push_strains.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        if !push_strains.is_empty() {
            let min_strain = push_strains
                .iter()
                .map(|x| x.2)
                .fold(f32::INFINITY, f32::min);
            let max_strain = push_strains
                .iter()
                .map(|x| x.2)
                .fold(f32::NEG_INFINITY, f32::max);
            eprintln!("Push intervals: {}", push_strains.len());
            eprintln!("Strain range: {:.2}% to {:.2}%", min_strain, max_strain);
            eprintln!("\nShortest 5:");
            for (rest, target, strain) in push_strains.iter().take(5) {
                eprintln!(
                    "  rest:{:.0}mm → target:{:.0}mm  strain:{:.2}%",
                    rest, target, strain
                );
            }
            eprintln!("\nLongest 5:");
            for (rest, target, strain) in push_strains.iter().rev().take(5) {
                eprintln!(
                    "  rest:{:.0}mm → target:{:.0}mm  strain:{:.2}%",
                    rest, target, strain
                );
            }
        }

        // Report the state
        eprintln!("\n✓ Execution completed successfully");
    }
}
