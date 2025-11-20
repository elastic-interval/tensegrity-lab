#[cfg(test)]
mod tests {
    use crate::build::brick_builders::build_brick_library;
    use crate::build::fabric_builders::build_fabric_library;
    use crate::build::tenscript::brick_library::BrickLibrary;
    use crate::build::tenscript::fabric_library::FabricLibrary;
    use crate::build::tenscript::fabric_plan_executor::{FabricPlanExecutor, ExecutorStage};
    use crate::fabric::Fabric;

    /// Benchmark data point from UI reference run
    #[derive(Debug)]
    struct Benchmark {
        time: f32,
        joints: usize,
        height_mm: f32,
        radius: f32,
        ground: usize,
    }

    /// Executor benchmarks - Values collected from FabricPlanExecutor
    fn ui_benchmarks() -> Vec<Benchmark> {
        vec![
            // BUILD phase - From FabricPlanExecutor (frame-independent)
            Benchmark { time:   0.0, joints:   0, height_mm:     0.0, radius:  0.000, ground: 0 },
            Benchmark { time:   3.0, joints:  62, height_mm:  6377.5, radius:  6.036, ground: 0 },
            Benchmark { time:   6.0, joints: 116, height_mm:  7514.0, radius: 10.724, ground: 0 },
            Benchmark { time:   9.0, joints: 170, height_mm:  8780.1, radius: 14.126, ground: 3 },
            Benchmark { time:  12.0, joints: 176, height_mm:  9260.4, radius: 14.191, ground: 0 },
            Benchmark { time:  15.0, joints: 176, height_mm:  9920.4, radius: 13.927, ground: 0 },
            Benchmark { time:  18.0, joints: 176, height_mm: 10469.5, radius: 13.664, ground: 0 },
            Benchmark { time:  21.0, joints: 176, height_mm: 10939.5, radius: 13.401, ground: 0 },
            Benchmark { time:  24.0, joints: 176, height_mm: 11349.8, radius: 13.139, ground: 0 },
            Benchmark { time:  27.0, joints: 176, height_mm: 11713.6, radius: 12.877, ground: 0 },
            Benchmark { time:  30.0, joints: 176, height_mm: 12041.9, radius: 12.615, ground: 3 },
            Benchmark { time:  33.0, joints: 176, height_mm: 12375.9, radius: 12.361, ground: 0 },
            Benchmark { time:  36.0, joints: 176, height_mm: 12683.0, radius: 12.118, ground: 0 },
            Benchmark { time:  39.0, joints: 176, height_mm: 12967.4, radius: 11.870, ground: 0 },
            Benchmark { time:  42.0, joints: 176, height_mm: 13226.3, radius: 11.621, ground: 0 },
            Benchmark { time:  45.0, joints: 176, height_mm: 13451.7, radius: 11.383, ground: 0 },
        ]
    }

    /// Check if fabric state matches benchmark (with tolerance)
    fn check_benchmark(fabric: &Fabric, frame: usize, benchmark: &Benchmark, tolerance_pct: f32) {
        let fabric_time = frame as f32 / 4000.0;
        let bounding_radius = fabric.bounding_radius();
        let (min_y, max_y) = fabric.altitude_range();
        let height_mm = (max_y - min_y) * fabric.scale;
        let ground_tolerance = 10.0 / fabric.scale;
        let ground_count = fabric.joints.iter()
            .filter(|j| j.location.y.abs() < ground_tolerance)
            .count();

        // Log current state
        eprintln!("[{:8.1}s] Radius: {:7.3}m | Height: {:7.1}mm | Ground: {:3} | Joints: {:3}",
            fabric_time, bounding_radius, height_mm, ground_count, fabric.joints.len());

        // Check joints (exact)
        assert_eq!(
            fabric.joints.len(), benchmark.joints,
            "At {:.1}s: Expected {} joints, got {}",
            fabric_time, benchmark.joints, fabric.joints.len()
        );

        // Check height (with tolerance)
        if benchmark.height_mm > 0.0 {
            let height_diff_pct = ((height_mm - benchmark.height_mm) / benchmark.height_mm * 100.0).abs();
            assert!(
                height_diff_pct < tolerance_pct,
                "At {:.1}s: Height {:.1}mm differs from benchmark {:.1}mm by {:.1}% (tolerance: {:.1}%)",
                fabric_time, height_mm, benchmark.height_mm, height_diff_pct, tolerance_pct
            );
        }

        // Check radius (with tolerance)
        if benchmark.radius > 0.0 {
            let radius_diff_pct = ((bounding_radius - benchmark.radius) / benchmark.radius * 100.0).abs();
            assert!(
                radius_diff_pct < tolerance_pct,
                "At {:.1}s: Radius {:.3}m differs from benchmark {:.3}m by {:.1}% (tolerance: {:.1}%)",
                fabric_time, bounding_radius, benchmark.radius, radius_diff_pct, tolerance_pct
            );
        }

        // Check ground contacts (exact for key checkpoints at 150s and 270s+)
        if fabric_time >= 150.0 && fabric_time <= 151.0 || fabric_time >= 270.0 {
            assert_eq!(
                ground_count, benchmark.ground,
                "At {:.1}s: Expected {} ground contacts, got {}",
                fabric_time, benchmark.ground, ground_count
            );
        }
    }

    /// Log fabric state every N seconds of fabric time for deterministic debugging
    fn log_fabric_state(fabric: &Fabric, frame: usize, phase: &str) {
        const FRAMES_PER_SEC: f32 = 4000.0;
        let fabric_time = frame as f32 / FRAMES_PER_SEC;
        let bounding_radius = fabric.bounding_radius();
        let (min_y, max_y) = fabric.altitude_range();
        let height_mm = (max_y - min_y) * fabric.scale;

        // Count ground contacts (within 10mm)
        let ground_tolerance = 10.0 / fabric.scale;
        let ground_count = fabric.joints.iter()
            .filter(|j| j.location.y.abs() < ground_tolerance)
            .count();

        eprintln!("[{:8.1}s] {} | Radius: {:7.3}m | Height: {:7.1}mm | Ground: {:3} | Joints: {:3}",
            fabric_time, phase, bounding_radius, height_mm, ground_count, fabric.joints.len());
    }

    #[test]
    fn test_executor_phases() {
        eprintln!("\n=== Testing FabricPlanExecutor Phases ===\n");

        let fabric_library = FabricLibrary::new(build_fabric_library());
        let brick_library = BrickLibrary::new(build_brick_library());

        let plan = fabric_library
            .fabric_plans
            .iter()
            .find(|p| p.name == "Triped")
            .expect("Triped not found")
            .clone();

        // Create executor
        let mut executor = FabricPlanExecutor::new(plan);
        let mut current_stage = ExecutorStage::Building;

        // Run until completion, logging key points (allow up to 5M iterations for full convergence)
        let mut iteration = 0;
        while !executor.is_complete() && iteration < 5_000_000 {
            // Log stage transitions
            let stage = executor.stage();
            if stage != &current_stage {
                eprintln!("[{:7}] Stage transition: {:?} -> {:?} (fabric age: {})",
                    iteration, current_stage, stage, executor.fabric.age);
                current_stage = stage.clone();
            }

            // Log periodic state during building phase
            if matches!(stage, ExecutorStage::Building) && iteration % 20000 == 0 {
                let (min_y, max_y) = executor.fabric.altitude_range();
                let height_mm = if executor.fabric.scale > 0.0 {
                    (max_y - min_y) * executor.fabric.scale
                } else {
                    0.0
                };
                let radius = executor.fabric.bounding_radius();
                eprintln!("[{:7}] Building | joints:{:3} height:{:8.3}mm radius:{:8.5}m age:{}",
                    iteration,
                    executor.fabric.joints.len(),
                    height_mm,
                    radius,
                    executor.fabric.age
                );
            }

            // Do one iteration
            executor.iterate(&brick_library).expect("Iteration failed");
            iteration += 1;
        }

        eprintln!("\n✓ Executor completed at iteration {} (fabric age: {})", iteration, executor.fabric.age);
        eprintln!("Final joints: {}", executor.fabric.joints.len());

        // Check that we reached completion
        assert!(executor.is_complete(), "Executor should have completed");
        assert_eq!(executor.stage(), &ExecutorStage::Complete);
    }

    #[test]
    fn test_all_build_benchmarks() {
        eprintln!("\n=== Testing All BUILD Phase Benchmarks ===\n");

        let fabric_library = FabricLibrary::new(build_fabric_library());
        let brick_library = BrickLibrary::new(build_brick_library());

        let plan = fabric_library
            .fabric_plans
            .iter()
            .find(|p| p.name == "Triped")
            .expect("Triped not found")
            .clone();

        // Use FabricPlanExecutor instead of manual stage management
        let mut executor = FabricPlanExecutor::new(plan);

        // Get BUILD phase benchmarks
        let build_benchmarks = ui_benchmarks();
        let build_benchmarks: Vec<_> = build_benchmarks.iter()
            .filter(|b| b.time <= 45.0)
            .collect();
        let mut benchmark_idx = 0;

        eprintln!("Running through BUILD phase, checking {} benchmarks...\n", build_benchmarks.len());

        // Run until all benchmarks checked (up to 50s to be safe)
        let max_iterations = (50.0 * 4000.0) as usize;

        // Check iteration 0 before running any iterations
        if benchmark_idx < build_benchmarks.len() {
            let benchmark = build_benchmarks[benchmark_idx];
            if benchmark.time == 0.0 {
                check_benchmark(&executor.fabric, 0, benchmark, 0.1);
                benchmark_idx += 1;
            }
        }

        // Run executor iteration by iteration, checking benchmarks
        for current_iteration in 1..=max_iterations {
            executor.iterate(&brick_library).expect("Iteration failed");

            // Check if we've hit a benchmark time
            if benchmark_idx < build_benchmarks.len() {
                let benchmark = build_benchmarks[benchmark_idx];
                let benchmark_iteration = (benchmark.time * 4000.0) as usize;

                if current_iteration == benchmark_iteration {
                    check_benchmark(&executor.fabric, current_iteration, benchmark, 0.1);
                    benchmark_idx += 1;
                }
            }

            // Stop if all benchmarks checked
            if benchmark_idx >= build_benchmarks.len() {
                break;
            }
        }

        eprintln!("\n✓ Checked {} BUILD phase benchmarks successfully!", benchmark_idx);

        // Print execution log
        executor.print_log();

        assert!(benchmark_idx >= build_benchmarks.len(),
            "Only checked {} of {} benchmarks", benchmark_idx, build_benchmarks.len());
    }

    #[test]
    fn test_triped_full_execution() {
        eprintln!("\n=== Testing Triped Full Execution: BUILD → PRETENSE → CONVERGE ===\n");

        let fabric_library = FabricLibrary::new(build_fabric_library());
        let brick_library = BrickLibrary::new(build_brick_library());

        let plan = fabric_library
            .fabric_plans
            .iter()
            .find(|p| p.name == "Triped")
            .expect("Triped not found")
            .clone();

        let mut executor = FabricPlanExecutor::new(plan);
        let mut current_stage = ExecutorStage::Building;
        let mut stage_entry_times: Vec<(ExecutorStage, usize)> = vec![];

        eprintln!("Starting execution...");
        let mut iteration = 0;
        while !executor.is_complete() && iteration < 5_000_000 {
            let stage = executor.stage();

            // Track stage transitions
            if stage != &current_stage {
                let fabric_time = executor.fabric.age;
                eprintln!("[iter {:7}] Stage: {:?} -> {:?} (age: {})",
                    iteration, current_stage, stage, fabric_time);
                stage_entry_times.push((stage.clone(), iteration));
                current_stage = stage.clone();
            }

            executor.iterate(&brick_library).expect("Iteration failed");
            iteration += 1;
        }

        eprintln!("\n✓ Execution completed at iteration {} (age: {})", iteration, executor.fabric.age);

        // Verify all stages were reached
        let stages: Vec<_> = stage_entry_times.iter().map(|(s, _)| s).collect();
        assert!(stages.contains(&&ExecutorStage::Building), "Should have Building stage");
        assert!(stages.contains(&&ExecutorStage::Pretensing), "Should have Pretensing stage");
        assert!(stages.contains(&&ExecutorStage::Converging), "Should have Converging stage");
        assert!(stages.contains(&&ExecutorStage::Complete), "Should have Complete stage");

        // Print execution log
        executor.print_log();
    }
}
