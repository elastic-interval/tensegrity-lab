use glam::Vec3;
use rand::prelude::*;
use rand::rngs::ThreadRng;

use crate::fabric::interval::Role;
use crate::fabric::{Fabric, IntervalKey, JointKey};
use crate::units::Meters;

struct KleinFabric {
    fabric: Fabric,
    joint_keys: Vec<JointKey>,
    random: ThreadRng,
}

impl KleinFabric {
    fn new() -> KleinFabric {
        KleinFabric {
            fabric: Fabric::new("klein".to_string()),
            joint_keys: Vec::new(),
            random: rand::rng(),
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
        self.fabric.create_fixed_interval(alpha_key, omega_key, Role::Pushing, Meters(8.0))
    }

    fn pull(&mut self, alpha: isize, omega: isize) -> IntervalKey {
        let alpha_key = self.joint_keys[alpha as usize];
        let omega_key = self.joint_keys[omega as usize];
        self.fabric.create_fixed_interval(alpha_key, omega_key, Role::Pulling, Meters(1.0))
    }

    fn coord(&mut self) -> f32 {
        self.random.random_range(-1000..1000) as f32 / 1000.0
    }
}

pub fn generate_klein(width: usize, height: usize, shift: usize) -> Fabric {
    let (w, h, sh) = (
        (width * 2) as isize,
        (height * 2 + 1) as isize,
        shift as isize,
    );
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
                // createFace(a, b, d)
                // createFace(a, c, d)
            }
        }
    }
    kf.fabric
}
