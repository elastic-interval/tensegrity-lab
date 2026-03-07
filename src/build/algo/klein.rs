use glam::Vec3;
use rand::prelude::*;
use rand::rngs::ThreadRng;

use crate::fabric::interval::Role;
use crate::fabric::physics::presets::CONSTRUCTION;
use crate::fabric::{Fabric, IntervalKey, JointKey};
use crate::units::{Meters, Seconds};

/// Duration over which intervals approach their ideal lengths.
/// Long enough to avoid violent forces from random initial placement.
const APPROACH_SECONDS: Seconds = Seconds(2.0);

struct KleinFabric {
    fabric: Fabric,
    joint_keys: Vec<JointKey>,
    random: ThreadRng,
}

impl KleinFabric {
    fn new() -> KleinFabric {
        KleinFabric {
            fabric: Fabric::new("Klein".to_string()),
            joint_keys: Vec::new(),
            random: rand::thread_rng(),
        }
    }

    fn random_joint(&mut self) {
        let mut v = Vec3::new(1.0, 1.0, 1.0);
        while v.length_squared() > 1.0 {
            v.x = self.coord();
            v.y = self.coord();
            v.z = self.coord();
        }
        let key = self.fabric.create_joint(v);
        self.joint_keys.push(key);
    }

    fn push(&mut self, alpha: isize, omega: isize) -> IntervalKey {
        let alpha_key = self.joint_keys[alpha as usize];
        let omega_key = self.joint_keys[omega as usize];
        self.fabric.create_approaching_interval(
            alpha_key,
            omega_key,
            Meters(8.0),
            Role::Pushing,
            APPROACH_SECONDS,
        )
    }

    fn pull(&mut self, alpha: isize, omega: isize) -> IntervalKey {
        let alpha_key = self.joint_keys[alpha as usize];
        let omega_key = self.joint_keys[omega as usize];
        self.fabric.create_approaching_interval(
            alpha_key,
            omega_key,
            Meters(1.0),
            Role::Pulling,
            APPROACH_SECONDS,
        )
    }

    fn coord(&mut self) -> f32 {
        self.random.gen_range(-1000..1000) as f32 / 1000.0
    }
}

/// Generate a Klein bottle tensegrity structure.
///
/// Width must be even, height must be odd.
/// Default parameters: width=10, height=31, shift=0.
pub fn generate_klein(width: usize, height: usize, shift: usize) -> Fabric {
    assert!(width % 2 == 0, "Klein bottle width must be even, got {}", width);
    assert!(height % 2 == 1, "Klein bottle height must be odd, got {}", height);
    let (w, h, sh) = (width as isize, height as isize, shift as isize);
    let mut kf = KleinFabric::new();
    let joint = |x: isize, y: isize| {
        let flip = (y / h) % 2 == 1;
        let x_rel = if flip { sh - 1 - x } else { x };
        let x_mod = (w * 2 + x_rel) % w;
        let y_mod = y % h;
        (y_mod * w + x_mod) / 2
    };
    for _ in 0..w * h / 2 {
        kf.random_joint();
    }
    for y in 0..h {
        for x in 0..w {
            if (x + y) % 2 == 0 {
                let (a, b, c, d, e, f) = (
                    joint(x, y),
                    joint(x - 1, y + 1),
                    joint(x + 1, y + 1),
                    joint(x, y + 2),
                    joint(x - 1, y + 3),
                    joint(x + 1, y + 3),
                );
                kf.pull(a, b);
                kf.pull(a, c);
                kf.pull(a, d);
                kf.push(a, e);
                kf.push(a, f);
                kf.push(e, f);
            }
        }
    }
    // Settle with construction physics while intervals approach their targets.
    // Approach duration is 2s = 40_000 iterations at 50µs each.
    let physics = CONSTRUCTION;
    let settle_iterations = (APPROACH_SECONDS.0 / crate::Age::iteration_duration()) as usize + 5000;
    for _ in 0..settle_iterations {
        kf.fabric.iterate(&physics);
    }
    kf.fabric.zero_velocities();
    kf.fabric
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fabric::interval::Role;
    use crate::fabric::physics::presets::PRETENSING;

    #[test]
    fn test_generate_klein() {
        let fabric = generate_klein(10, 31, 0);

        let joint_count = fabric.joints.len();
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
            "Klein (10x31): {} joints, {} struts, {} cables",
            joint_count, push_count, pull_count
        );

        // width * height / 2 = 10 * 31 / 2 = 155 joints
        assert_eq!(joint_count, 155, "Should have width*height/2 joints");
        assert!(push_count > 0, "Should have pushing struts");
        assert!(pull_count > 0, "Should have pulling cables");
    }

    #[test]
    fn test_klein_settles() {
        let mut fabric = generate_klein(10, 31, 0);

        // After generation (which includes settling), the fabric should not be frozen
        assert!(!fabric.frozen, "Fabric should not be frozen after generation");

        // All approaches should have completed
        assert!(
            !fabric.has_approaching_intervals(),
            "All intervals should have finished approaching"
        );

        // Run additional iterations with pretensing physics - should not freeze
        let physics = PRETENSING;
        for _ in 0..1000 {
            fabric.iterate(&physics);
        }
        assert!(
            !fabric.frozen,
            "Fabric should remain stable under pretensing physics"
        );

        let max_vel = fabric.max_velocity();
        println!("Max velocity after 1000 pretensing iterations: {:.4}", max_vel);
        assert!(max_vel < 100.0, "Velocity should be reasonable, got {:.2}", max_vel);
    }
}
