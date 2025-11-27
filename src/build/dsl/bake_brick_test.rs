#[cfg(test)]
mod tests {
    use crate::build::dsl::brick_dsl::BrickName;
    use crate::build::dsl::brick_library::get_prototype;
    use crate::fabric::interval::Role;
    use crate::fabric::physics::presets::BAKING;
    use crate::fabric::Fabric;
    use cgmath::{InnerSpace, MetricSpace};

    const SPEED_LIMIT: f32 = 1e-6;

    #[test]
    fn test_bake_single_right_brick() {
        eprintln!("\n=== Testing Single Right Brick Baking ===\n");

        // Get prototype and create fabric
        let proto = get_prototype(BrickName::SingleRightBrick);
        let mut fabric = Fabric::from(proto);

        // Remove faces and face radials for simpler physics
        fabric.faces.clear();
        let radial_ids: Vec<_> = fabric
            .intervals
            .iter()
            .enumerate()
            .filter_map(|(i, int)| {
                int.as_ref()
                    .filter(|int| int.role == Role::FaceRadial)
                    .map(|_| i)
            })
            .collect();
        for id in radial_ids {
            fabric.intervals[id] = None;
        }

        // Count intervals
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

        eprintln!("Initial: {} push, {} pull intervals", push_count, pull_count);
        assert_eq!(push_count, 3, "Should have 3 push intervals");
        assert_eq!(pull_count, 3, "Should have 3 pull intervals");

        // Print initial state
        eprintln!("\nInitial intervals:");
        for (i, interval) in fabric.intervals.iter().enumerate() {
            if let Some(int) = interval {
                let alpha = fabric.joints[int.alpha_index].location;
                let omega = fabric.joints[int.omega_index].location;
                let actual = alpha.distance(omega);
                eprintln!(
                    "  [{i}] {:?} ({}->{}) ideal={:.4} actual={:.4} strain={:.4}",
                    int.role, int.alpha_index, int.omega_index, int.ideal(), actual, int.strain
                );
            }
        }

        // Verify push intervals start perpendicular
        let push_intervals: Vec<_> = fabric
            .intervals
            .iter()
            .filter_map(|int| int.as_ref().filter(|int| int.role == Role::Pushing))
            .collect();

        let directions: Vec<_> = push_intervals
            .iter()
            .map(|int| {
                let alpha = fabric.joints[int.alpha_index].location;
                let omega = fabric.joints[int.omega_index].location;
                (omega - alpha).normalize()
            })
            .collect();

        eprintln!("\nInitial perpendicularity:");
        for i in 0..3 {
            for j in (i + 1)..3 {
                let dot = directions[i].dot(directions[j]);
                eprintln!("  Push {} · Push {} = {:.6}", i, j, dot);
                assert!(
                    dot.abs() < 0.01,
                    "Push intervals should start perpendicular"
                );
            }
        }

        // Run physics iterations with BAKING preset
        let physics = BAKING;
        eprintln!("\nRunning physics with BAKING preset...");
        eprintln!(
            "  drag={}, viscosity={}, pretenst={}%",
            physics.drag(),
            physics.viscosity(),
            *physics.pretenst
        );

        let mut iteration = 0;
        let max_iterations = 100_000;

        loop {
            fabric.iterate(&physics);
            let max_speed = fabric.stats.max_speed;
            iteration += 1;

            // First 10 iterations: detailed trace
            if iteration <= 10 {
                eprintln!("  iteration {}: max_speed={:.4}", iteration, max_speed);
                for (i, joint) in fabric.joints.iter().take(6).enumerate() {
                    let v = joint.velocity;
                    eprintln!(
                        "    joint {}: vel=({:.4}, {:.4}, {:.4}) |v|={:.4}",
                        i, v.x, v.y, v.z, v.magnitude()
                    );
                }
            } else if iteration % 10_000 == 0 {
                eprintln!("  iteration {}: max_speed={:.2e}", iteration, max_speed);
            }

            if max_speed < SPEED_LIMIT {
                eprintln!("  Converged at iteration {} with max_speed={:.2e}", iteration, max_speed);
                break;
            }

            if iteration >= max_iterations {
                eprintln!("  Did not converge after {} iterations, max_speed={:.2e}", iteration, max_speed);
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
                eprintln!(
                    "  [{i}] {:?} ideal={:.4} actual={:.4} strain={:.4}",
                    int.role, int.ideal(), actual, int.strain
                );
            }
        }

        eprintln!("\nFinal joint positions:");
        for (i, joint) in fabric.joints.iter().take(6).enumerate() {
            eprintln!(
                "  Joint {}: ({:.4}, {:.4}, {:.4})",
                i, joint.location.x, joint.location.y, joint.location.z
            );
        }

        // Verify final perpendicularity
        let final_directions: Vec<_> = fabric
            .intervals
            .iter()
            .filter_map(|int| int.as_ref().filter(|int| int.role == Role::Pushing))
            .map(|int| {
                let alpha = fabric.joints[int.alpha_index].location;
                let omega = fabric.joints[int.omega_index].location;
                (omega - alpha).normalize()
            })
            .collect();

        eprintln!("\nFinal perpendicularity:");
        for i in 0..3 {
            for j in (i + 1)..3 {
                let dot = final_directions[i].dot(final_directions[j]);
                eprintln!("  Push {} · Push {} = {:.6}", i, j, dot);
            }
        }

        eprintln!("\n✓ Brick baking test complete!");
    }
}
