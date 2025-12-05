#[cfg(test)]
mod tests {
    use crate::build::dsl::fabric_library::{self, FabricName};
    use crate::build::dsl::fabric_plan_executor::{ExecutorStage, FabricPlanExecutor};
    use crate::fabric::Fabric;

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
    /// Updated for faster Triped construction (5s space, 1s vulcanize, 1s pretense)
    fn ui_benchmarks() -> Vec<Benchmark> {
        vec![
            Benchmark { age: 0.0, joints: 0, height_mm: 0.0, radius: 0.000, ground: 0 },
            // BUILD phase (fabric age 0-8s)
            Benchmark { age: 2.0, joints: 176, height_mm: 7608.0, radius: 12.232, ground: 0 },
            Benchmark { age: 4.0, joints: 176, height_mm: 10671.3, radius: 10.215, ground: 0 },
            Benchmark { age: 6.0, joints: 176, height_mm: 12251.1, radius: 8.152, ground: 0 },
            Benchmark { age: 8.0, joints: 176, height_mm: 9724.6, radius: 6.256, ground: 0 },
            // PRETENSE phase (fabric age ~8-9s)
            // FALL phase (fabric age ~9-17s)
            Benchmark { age: 10.0, joints: 168, height_mm: 9709.8, radius: 6.644, ground: 3 },
            Benchmark { age: 12.0, joints: 168, height_mm: 9654.7, radius: 6.699, ground: 3 },
            Benchmark { age: 14.0, joints: 168, height_mm: 9659.8, radius: 6.686, ground: 3 },
            Benchmark { age: 16.0, joints: 168, height_mm: 9625.0, radius: 6.664, ground: 3 },
            // SETTLE phase (fabric age ~17-20s)
            Benchmark { age: 19.0, joints: 168, height_mm: 9589.7, radius: 6.663, ground: 3 },
        ]
    }

    /// Check if fabric state matches benchmark (with tolerance)
    fn check_benchmark(fabric: &Fabric, benchmark: &Benchmark, tolerance_pct: f32) {
        let fabric_age = fabric.age.as_duration().as_secs_f32();
        let bounding_radius = fabric.bounding_radius();
        let (min_y, max_y) = fabric.altitude_range();
        let height_mm = (max_y - min_y) * fabric.scale;
        let ground_tolerance = 10.0 / fabric.scale;
        let ground_count = fabric
            .joints
            .iter()
            .filter(|j| j.location.y.abs() < ground_tolerance)
            .count();

        // Print benchmark format for easy copy-paste
        eprintln!(
            "            Benchmark {{ age: {:.1}, joints: {}, height_mm: {:.1}, radius: {:.3}, ground: {} }},",
            fabric_age,
            fabric.joints.len(),
            height_mm,
            bounding_radius,
            ground_count
        );

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
        let mut executor = FabricPlanExecutor::new(plan);
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
                let height_mm = if executor.fabric.scale > 0.0 {
                    (max_y - min_y) * executor.fabric.scale
                } else {
                    0.0
                };
                let radius = executor.fabric.bounding_radius();
                let ground_tolerance = 10.0 / executor.fabric.scale.max(1.0);
                let ground_count = executor.fabric.joints.iter()
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

        let mut executor = FabricPlanExecutor::new(plan);

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

        eprintln!(
            "\n✓ Checked {} benchmarks successfully!",
            benchmark_idx
        );

        executor.print_log();

        assert!(
            benchmark_idx >= benchmarks.len(),
            "Only checked {} of {} benchmarks",
            benchmark_idx,
            benchmarks.len()
        );
    }

    #[test]
    fn test_triped_full_execution() {
        eprintln!("\n=== Testing Triped Full Execution: BUILD → PRETENSE → CONVERGE ===\n");

        let plan = fabric_library::get_fabric_plan(FabricName::Triped);

        let mut executor = FabricPlanExecutor::new(plan);
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

        let mut executor = FabricPlanExecutor::new(plan);

        // Run until completion
        let mut iteration = 0;
        while !executor.is_complete() && iteration < 5_000_000 {
            let _ = executor.iterate();
            iteration += 1;
        }

        // Check final state
        let (min_y, max_y) = executor.fabric.altitude_range();
        let height_mm = (max_y - min_y) * executor.fabric.scale;
        let radius = executor.fabric.bounding_radius();
        let ground_tolerance = 10.0 / executor.fabric.scale;
        let ground_count = executor
            .fabric
            .joints
            .iter()
            .filter(|j| j.location.y.abs() < ground_tolerance)
            .count();

        eprintln!("\n=== FINAL CONVERGED STATE ===");
        eprintln!("Height: {:.1}mm ({:.2}m)", height_mm, height_mm / 1000.0);
        eprintln!("Radius: {:.3}m", radius);
        eprintln!("Ground contacts: {}", ground_count);
        eprintln!("Total joints: {}", executor.fabric.joints.len());
        eprintln!("Iterations: {}", iteration);

        // Report the state
        eprintln!("\n✓ Execution completed successfully");
    }
}
