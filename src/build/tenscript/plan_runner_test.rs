#[cfg(test)]
mod tests {
    use crate::build::tenscript::brick_library::BrickLibrary;
    use crate::build::tenscript::fabric_library::FabricLibrary;
    use crate::build::tenscript::plan_context::PlanContext;
    use crate::build::tenscript::plan_runner::PlanRunner;
    use crate::fabric::physics::presets::BASE_PHYSICS;
    use crate::fabric::Fabric;

    #[test]
    fn test_build_triped_headless() {
        println!("=== Building Triped Headlessly ===\n");

        // Load libraries
        let fabric_library = FabricLibrary::from_source()
            .expect("Failed to load fabric library");
        let brick_library = BrickLibrary::from_source()
            .expect("Failed to load brick library");

        // Get Triped plan
        let plan = fabric_library
            .fabric_plans
            .iter()
            .find(|p| p.name == "Triped")
            .expect("Triped not found")
            .clone();

        // Create fabric and physics
        let mut fabric = Fabric::new(plan.name.clone());
        let mut physics = BASE_PHYSICS;

        // Create context
        let mut context = PlanContext::new(&mut fabric, &mut physics, &brick_library);

        // Build the structure (45 seconds settle time to match UI)
        println!("Building structure (45s settle time)...");
        PlanRunner::run_headless(plan, &mut context, 45.0)
            .expect("Failed to build structure");

        println!("Build complete!\n");

        // Analyze the result
        let joint_count = context.fabric.joints.len();
        let interval_count = context.fabric.intervals.len();
        
        println!("Structure stats:");
        println!("  Joints: {}", joint_count);
        println!("  Intervals: {}", interval_count);
        println!("  Scale: {:.2}mm", context.fabric.scale);

        // Find ground contact points (y ≈ 0)
        let ground_tolerance = 0.05; // 50mm tolerance
        let ground_points: Vec<_> = context.fabric.joints
            .iter()
            .filter(|j| j.location.y.abs() < ground_tolerance)
            .collect();

        println!("\nGround contact:");
        println!("  Points within {}m of surface: {}", ground_tolerance, ground_points.len());

        // Get the 3 lowest points
        let mut sorted_joints: Vec<_> = context.fabric.joints.iter().collect();
        sorted_joints.sort_by(|a, b| a.location.y.partial_cmp(&b.location.y).unwrap());

        let lowest_3: Vec<f32> = sorted_joints
            .iter()
            .take(3)
            .map(|j| j.location.y)
            .collect();

        println!("  Lowest 3 points (y): {:?}", lowest_3);

        // Measure height
        let max_y = context.fabric.joints
            .iter()
            .map(|j| j.location.y)
            .fold(f32::NEG_INFINITY, f32::max);

        let min_y = context.fabric.joints
            .iter()
            .map(|j| j.location.y)
            .fold(f32::INFINITY, f32::min);

        let height_mm = (max_y - min_y) * context.fabric.scale;

        // Find the highest joints (like selecting in UI)
        let mut joints_by_height: Vec<_> = context.fabric.joints.iter().collect();
        joints_by_height.sort_by(|a, b| b.location.y.partial_cmp(&a.location.y).unwrap());

        println!("\nHeight:");
        println!("  Min Y: {:.4}m", min_y);
        println!("  Max Y: {:.4}m", max_y);
        println!("  Structure span: {:.2}mm", height_mm);
        println!("\nTop 5 joints (like UI selection):");
        for (i, joint) in joints_by_height.iter().take(5).enumerate() {
            let height_mm = joint.location.y * context.fabric.scale;
            println!("  {}. Joint at Y={:.4}m → {:.2}mm", i + 1, joint.location.y, height_mm);
        }

        // Verify structure is reasonable
        assert!(joint_count > 0, "Structure has no joints");
        assert!(height_mm > 1000.0, "Structure too short: {}mm", height_mm);
        assert!(height_mm < 15000.0, "Structure too tall: {}mm", height_mm);
        assert!(
            lowest_3.iter().all(|&y| y.abs() < 0.2),
            "Structure not stable on ground. Lowest points: {:?}",
            lowest_3
        );

        println!("\n✓ Structure built successfully!");
    }
}
