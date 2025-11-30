#[cfg(test)]
mod tests {
    use crate::build::dsl::brick_dsl::BrickName;
    use crate::build::dsl::brick_library::get_prototype;
    use crate::fabric::interval::Role;
    use crate::fabric::physics::presets::BAKING;
    use cgmath::MetricSpace;
    use strum::IntoEnumIterator;

    const SPEED_LIMIT: f32 = 1e-6;
    const MAX_ITERATIONS: usize = 100_000;

    fn test_brick(brick_name: BrickName) {
        eprintln!("\n=== Testing {} ===\n", brick_name);

        let proto = get_prototype(brick_name);
        let mut fabric = proto.to_fabric(brick_name.face_scaling());

        // Count intervals by role
        let push_count = fabric
            .intervals
            .iter()
            .filter(|i| matches!(i, Some(int) if int.role == Role::Pushing))
            .count();
        let pull_count = fabric
            .intervals
            .iter()
            .filter(|i| matches!(i, Some(int) if int.role == Role::Pulling))
            .count();
        let radial_count = fabric
            .intervals
            .iter()
            .filter(|i| matches!(i, Some(int) if int.role == Role::FaceRadial))
            .count();

        eprintln!("Structure:");
        eprintln!("  joints: {}", fabric.joints.len());
        eprintln!("  push intervals: {}", push_count);
        eprintln!("  pull intervals: {}", pull_count);
        eprintln!("  face radials: {}", radial_count);
        eprintln!("  faces: {}", fabric.faces.len());

        // Print initial intervals
        eprintln!("\nInitial intervals:");
        for (i, interval) in fabric.intervals.iter().enumerate() {
            if let Some(int) = interval {
                let alpha = fabric.joints[int.alpha_index].location;
                let omega = fabric.joints[int.omega_index].location;
                let actual = alpha.distance(omega);
                eprintln!(
                    "  [{i}] {:?} ({}->{}) ideal={:.4} actual={:.4}",
                    int.role,
                    int.alpha_index,
                    int.omega_index,
                    int.ideal(),
                    actual
                );
            }
        }

        // Run physics
        let physics = BAKING;
        eprintln!("\nRunning physics...");

        let mut iteration = 0;
        loop {
            fabric.iterate(&physics);
            let max_speed = fabric.stats.max_speed;
            iteration += 1;

            if iteration % 10_000 == 0 {
                eprintln!("  iteration {}: max_speed={:.2e}", iteration, max_speed);
            }

            if max_speed < SPEED_LIMIT {
                eprintln!("  Converged at iteration {}", iteration);
                break;
            }

            if iteration >= MAX_ITERATIONS {
                eprintln!(
                    "  Did not converge after {} iterations, max_speed={:.2e}",
                    iteration, max_speed
                );
                break;
            }
        }

        // Print final state
        eprintln!("\nFinal intervals:");
        for (i, interval) in fabric.intervals.iter().enumerate() {
            if let Some(int) = interval {
                let alpha = fabric.joints[int.alpha_index].location;
                let omega = fabric.joints[int.omega_index].location;
                let actual = alpha.distance(omega);
                let diff_pct = ((actual - int.ideal()) / int.ideal() * 100.0).abs();
                eprintln!(
                    "  [{i}] {:?} ideal={:.4} actual={:.4} diff={:.1}%",
                    int.role,
                    int.ideal(),
                    actual,
                    diff_pct
                );
            }
        }

        eprintln!("\n✓ {} complete!", brick_name);
    }

    #[test]
    fn test_bake_all_bricks() {
        for brick_name in BrickName::iter() {
            test_brick(brick_name);
        }
    }
}
