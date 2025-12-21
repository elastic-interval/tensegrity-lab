
#[cfg(test)]
mod tests {
    use crate::fabric::Fabric;
    use crate::fabric::physics::presets::VIEWING;
    use crate::fabric::material::Material;
    use cgmath::Point3;

    #[test]
    fn test_single_push_interval_drop() {
        eprintln!("\n=== Testing Single Push Interval Drop ===\n");

        // Create fabric with 2 joints and 1 push interval
        let mut fabric = Fabric::new("test_drop".to_string());

        // Create two joints at 5 meters altitude, 1 meter apart horizontally
        // Coordinates are in meters directly
        let altitude_m = 5.0;

        let joint1_index = fabric.create_joint(Point3::new(0.0, altitude_m, 0.0));
        let joint2_index = fabric.create_joint(Point3::new(1.0, altitude_m, 0.0));

        // Create push interval between them (1.0 meter ideal length)
        fabric.create_interval(joint1_index, joint2_index, 1.0, Material::Push.default_role());

        eprintln!("Initial setup:");
        eprintln!("  Altitude: {:.2}m", altitude_m);
        eprintln!("  Surface: Frozen (locks on contact)");
        eprintln!("  Interval: Push, 1 meter length");
        eprintln!();

        let physics = VIEWING;

        // Measure falling motion every 50ms (1000 iterations)
        let mut frame = 0;
        let mut hit_surface = false;

        eprintln!("Time(ms)  Altitude(m)  Velocity(m/s)  Accel(m/s²)");
        eprintln!("--------  -----------  -------------  -----------");

        let mut last_velocity = 0.0;

        loop {
            // Run 1000 iterations (50ms of simulated time)
            for _ in 0..1000 {
                fabric.iterate(&physics);
            }

            frame += 1;
            let time_ms = frame * 50;

            // Calculate average altitude and velocity of the two joints
            let joint0 = &fabric.joints[fabric.joint_by_id[0]];
            let joint1 = &fabric.joints[fabric.joint_by_id[1]];
            let avg_altitude_m = (joint0.location.y + joint1.location.y) / 2.0;
            let velocity_m_s = (joint0.velocity.y + joint1.velocity.y) / 2.0;

            // Calculate acceleration from velocity change
            let delta_v = velocity_m_s - last_velocity;
            let delta_t = 0.050; // 50ms
            let acceleration = delta_v / delta_t;

            eprintln!("{:8}  {:11.3}  {:13.3}  {:11.2}",
                time_ms,
                avg_altitude_m,
                velocity_m_s,
                acceleration
            );

            last_velocity = velocity_m_s;

            // Check if hit surface (altitude <= 0)
            if avg_altitude_m <= 0.0 {
                hit_surface = true;
                eprintln!("\n✓ Hit surface at t={}ms", time_ms);
                eprintln!("  Final altitude: {:.3}m", avg_altitude_m);
                break;
            }

            // Stop after 2 seconds to avoid infinite loop
            if time_ms >= 2000 {
                eprintln!("\n⚠ Did not hit surface after 2 seconds");
                eprintln!("  Final altitude: {:.3}m", avg_altitude_m);
                break;
            }
        }

        eprintln!("\n=== Expected behavior ===");
        eprintln!("Free fall from 5m should take: t = √(2h/g) = √(10/9.8) = 1.01 seconds");
        eprintln!("Final velocity should be: v = √(2gh) = √(2×9.8×5) = 9.9 m/s");
        eprintln!("Acceleration should be constant: 9.8 m/s²");

        if hit_surface {
            eprintln!("\n✓ Test completed - interval hit surface");
        } else {
            eprintln!("\n✗ Test incomplete - interval still falling");
        }
    }
}
