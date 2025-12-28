use std::f32::consts::PI;

use glam::Vec3;

use crate::fabric::interval::Role;
use crate::fabric::joint_path::JointPath;
use crate::fabric::{Fabric, JointKey};
use crate::units::Meters;

/// Generate a tensegrity Möbius band.
///
/// Creates a zigzag strip that twists 180° as it goes around.
/// Joints alternate top/bottom positions. Each joint connects to:
/// - offset+1: adjacent joint (pull - across width)
/// - offset+2: skip one joint (pull - along edge)
/// - offset+3: diagonal (push - strut crossing the tile)
///
/// This creates overlapping rectangular tiles with crossing struts.
/// Joints are named sequentially (0, 1, 2, ...) following the band topology.
pub fn generate_mobius(segments: usize) -> Fabric {
    let mut fabric = Fabric::new("Mobius".to_string());
    let mut joint_keys: Vec<JointKey> = Vec::new();

    // Joint count must be odd to complete the Möbius twist
    let joint_count = segments * 2 + 1;

    // Band proportions - width should be reasonable relative to radius
    let band_width = 2.0;
    let radius = 5.0 + (segments as f32 * 0.1); // Grows slowly with segments

    // Möbius strip parametric position
    // Joints alternate bottom/top as they go around
    let location = |bottom: bool, angle: f32| -> Vec3 {
        let major = Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
        let outwards = major.normalize();
        let up = Vec3::Y;
        // The twist: cross-section rotates by angle/2
        let ray = outwards * (angle / 2.0).sin() + up * (angle / 2.0).cos();
        let minor = ray * band_width * if bottom { -0.5 } else { 0.5 };
        major + minor
    };

    // Create joints around the loop, alternating bottom/top
    // Use sequential local_index to reflect the band topology
    for joint_index in 0..joint_count {
        let angle = joint_index as f32 / joint_count as f32 * PI * 2.0;
        let pos = location(joint_index % 2 == 0, angle);
        let path = JointPath::new(joint_index as u8);
        let key = fabric.create_joint_with_path(pos, path);
        joint_keys.push(key);
    }

    // Consistent interval lengths - should roughly match geometry
    // Based on test output: edge~2.1, width~2.3, diagonal~3.8
    let push_length = Meters(4.0); // Diagonal struts (slightly longer to push)
    let pull_edge = Meters(2.0); // Along the band edge (offset 0 to 2)
    let pull_width = Meters(2.2); // Across the band width (offset 0 to 1)

    // Create intervals with overlapping pattern (matching original TS pattern)
    // joint(offset) = (jointIndex * 2 + offset) % joint_count
    for joint_index in 0..joint_count {
        let j = |offset: usize| joint_keys[(joint_index * 2 + offset) % joint_count];

        // Pull along edge (offset 0 to 2)
        fabric.create_fixed_interval(j(0), j(2), Role::Pulling, pull_edge);
        // Pull across width (offset 0 to 1)
        fabric.create_fixed_interval(j(0), j(1), Role::Pulling, pull_width);
        // Push diagonal (offset 0 to 3)
        fabric.create_fixed_interval(j(0), j(3), Role::Pushing, push_length);
    }

    fabric
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fabric::interval::Role;
    use crate::fabric::physics::presets::PRETENSING;
    use crate::units::Unit;

    #[test]
    fn test_generate_mobius() {
        let fabric = generate_mobius(50);

        assert!(!fabric.joints.is_empty(), "Should have joints");
        assert!(!fabric.intervals.is_empty(), "Should have intervals");

        let push_count = fabric
            .intervals
            .values()
            .filter(|i| i.role == Role::Pushing)
            .count();
        let pull_count = fabric
            .intervals
            .values()
            .filter(|i| i.role == Role::Pulling)
            .count();

        println!(
            "Mobius (50 segments): {} joints, {} struts, {} cables",
            fabric.joints.len(),
            push_count,
            pull_count
        );

        // 50 segments = 101 joints (2*segments + 1 for Möbius twist)
        assert_eq!(fabric.joints.len(), 101, "Should have 2*segments+1 joints");
        assert!(push_count > 0, "Should have pushing struts");
        assert!(pull_count > 0, "Should have pulling cables");
    }

    #[test]
    fn test_mobius_first_iterations() {
        let mut fabric = generate_mobius(20);
        let physics = PRETENSING;

        println!("\n=== Initial state ===");
        // Check actual vs ideal lengths for intervals
        for (i, (_key, interval)) in fabric.intervals.iter().take(6).enumerate() {
            let alpha = &fabric.joints[interval.alpha_key];
            let omega = &fabric.joints[interval.omega_key];
            let actual = ((omega.location.x - alpha.location.x).powi(2)
                + (omega.location.y - alpha.location.y).powi(2)
                + (omega.location.z - alpha.location.z).powi(2))
            .sqrt();
            let ideal = interval.ideal().f32();
            let strain = (actual - ideal) / ideal * 100.0;
            println!(
                "Interval {}: {:?} actual={:.3} ideal={:.3} strain={:.1}%",
                i, interval.role, actual, ideal, strain
            );
        }

        println!(
            "Before iteration: frozen={}, max_velocity={:.2}",
            fabric.frozen,
            fabric.max_velocity()
        );

        // Run a few iterations and check velocities
        for iter in 0..10 {
            fabric.iterate(&physics);
            let max_vel = fabric.max_velocity();
            println!(
                "Iteration {}: frozen={} max_velocity = {:.2}",
                iter, fabric.frozen, max_vel
            );
            if fabric.frozen {
                println!("  Fabric was frozen!");
                break;
            }
        }

        println!("Final frozen state: {}", fabric.frozen);
    }

    #[test]
    fn test_generate_mobius_creates_valid_intervals() {
        let fabric = generate_mobius(30);

        for interval in fabric.intervals.values() {
            // With SlotMap, we just verify the keys point to valid joints
            assert!(
                fabric.joints.get(interval.alpha_key).is_some(),
                "Alpha joint key should be valid"
            );
            assert!(
                fabric.joints.get(interval.omega_key).is_some(),
                "Omega joint key should be valid"
            );
            assert!(
                interval.ideal().f32() > 0.0,
                "Interval should have positive ideal length"
            );
        }
    }
}
