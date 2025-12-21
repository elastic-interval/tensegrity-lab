use std::f32::consts::PI;

use cgmath::{EuclideanSpace, InnerSpace, Point3, Vector3};

use crate::fabric::interval::Role;
use crate::fabric::Fabric;

/// Generate a tensegrity Möbius band.
///
/// Creates a zigzag strip that twists 180° as it goes around.
/// Joints alternate top/bottom positions. Each joint connects to:
/// - offset+1: adjacent joint (pull - across width)
/// - offset+2: skip one joint (pull - along edge)
/// - offset+3: diagonal (push - strut crossing the tile)
///
/// This creates overlapping rectangular tiles with crossing struts.
pub fn generate_mobius(segments: usize) -> Fabric {
    let mut fabric = Fabric::new("Mobius".to_string());


    // Joint count must be odd to complete the Möbius twist
    let joint_count = segments * 2 + 1;

    // Band proportions - width should be reasonable relative to radius
    let band_width = 2.0;
    let radius = 5.0 + (segments as f32 * 0.1); // Grows slowly with segments

    // Möbius strip parametric position
    // Joints alternate bottom/top as they go around
    let location = |bottom: bool, angle: f32| -> Point3<f32> {
        let major = Vector3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
        let outwards = major.normalize();
        let up = Vector3::unit_y();
        // The twist: cross-section rotates by angle/2
        let ray = outwards * (angle / 2.0).sin() + up * (angle / 2.0).cos();
        let minor = ray * band_width * if bottom { -0.5 } else { 0.5 };
        Point3::from_vec(major + minor)
    };

    // Create joints around the loop, alternating bottom/top
    let mut positions: Vec<Point3<f32>> = Vec::with_capacity(joint_count);
    for joint_index in 0..joint_count {
        let angle = joint_index as f32 / joint_count as f32 * PI * 2.0;
        let pos = location(joint_index % 2 == 0, angle);
        positions.push(pos);
        fabric.create_joint(pos);
    }

    // Consistent interval lengths - should roughly match geometry
    // Based on test output: edge~2.1, width~2.3, diagonal~3.8
    let push_length = 4.0;   // Diagonal struts (slightly longer to push)
    let pull_edge = 2.0;     // Along the band edge (offset 0 to 2)
    let pull_width = 2.2;    // Across the band width (offset 0 to 1)

    // Create intervals with overlapping pattern (matching original TS pattern)
    // joint(offset) = (jointIndex * 2 + offset) % joint_count
    for joint_index in 0..joint_count {
        let j = |offset: usize| (joint_index * 2 + offset) % joint_count;

        // Pull along edge (offset 0 to 2)
        fabric.create_interval(j(0), j(2), pull_edge, Role::Pulling);
        // Pull across width (offset 0 to 1)
        fabric.create_interval(j(0), j(1), pull_width, Role::Pulling);
        // Push diagonal (offset 0 to 3)
        fabric.create_interval(j(0), j(3), push_length, Role::Pushing);
    }

    fabric
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fabric::interval::Role;
    use crate::fabric::physics::presets::PRETENSING;

    #[test]
    fn test_generate_mobius() {
        let fabric = generate_mobius(50);

        assert!(!fabric.joints.is_empty(), "Should have joints");
        assert!(!fabric.intervals.is_empty(), "Should have intervals");

        let push_count = fabric.intervals.values()
            .filter(|i| i.role == Role::Pushing)
            .count();
        let pull_count = fabric.intervals.values()
            .filter(|i| i.role == Role::Pulling)
            .count();

        println!("Mobius (50 segments): {} joints, {} struts, {} cables",
            fabric.joints.len(), push_count, pull_count);

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
            let alpha = &fabric.joints[interval.alpha_index];
            let omega = &fabric.joints[interval.omega_index];
            let actual = ((omega.location.x - alpha.location.x).powi(2)
                + (omega.location.y - alpha.location.y).powi(2)
                + (omega.location.z - alpha.location.z).powi(2)).sqrt();
            let ideal = interval.ideal();
            let strain = (actual - ideal) / ideal * 100.0;
            println!("Interval {}: {:?} actual={:.3} ideal={:.3} strain={:.1}%",
                i, interval.role, actual, ideal, strain);
        }

        println!("Before iteration: frozen={}, max_velocity={:.2}", fabric.frozen, fabric.max_velocity());

        // Run a few iterations and check velocities
        for iter in 0..10 {
            fabric.iterate(&physics);
            let max_vel = fabric.max_velocity();
            println!("Iteration {}: frozen={} max_velocity = {:.2}", iter, fabric.frozen, max_vel);
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

        let joint_count = fabric.joints.len();
        for interval in fabric.intervals.values() {
            assert!(interval.alpha_index < joint_count,
                "Alpha joint {} should be < {}", interval.alpha_index, joint_count);
            assert!(interval.omega_index < joint_count,
                "Omega joint {} should be < {}", interval.omega_index, joint_count);
            assert!(interval.ideal() > 0.0, "Interval should have positive ideal length");
        }
    }
}
