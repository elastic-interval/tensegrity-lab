//! A rough earthquake: the ground shaking horizontally at a few frequencies
//! at once, in random directions, until it is stopped, carrying along
//! whatever rests on it. It reaches about half of g, a strong quake's; the
//! seismic loads for the site are the engineer's.

use crate::fabric::Fabric;
use crate::units::Seconds;
use crate::Age;
use glam::Vec3;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::f32::consts::TAU;

const SHAKES: usize = 4;
/// The ground's acceleration at the quake's height, in metres per second².
const PEAK_ACCELERATION: f32 = 5.0;
/// Starting and stopping, quickly but without a jolt.
const SWELL: Seconds = Seconds(0.5);

#[derive(Clone, Debug)]
struct Shake {
    toward: Vec3,
    hertz: f32,
    phase: f32,
}

#[derive(Clone, Debug)]
pub struct Quake {
    shakes: Vec<Shake>,
    since: Age,
    /// When it was asked to stop: it dies away over the swell after.
    stopping: Option<Age>,
}

impl Quake {
    /// A quake starting at `since`, its shakes drawn from `seed`.
    pub fn starting(since: Age, seed: u64) -> Quake {
        let mut random = ChaCha8Rng::seed_from_u64(seed);
        let shakes = (0..SHAKES)
            .map(|_| {
                let angle = random.gen_range(0.0..TAU);
                Shake {
                    toward: Vec3::new(angle.cos(), 0.0, angle.sin()),
                    hertz: random.gen_range(0.5..2.0),
                    phase: random.gen_range(0.0..TAU),
                }
            })
            .collect();
        Quake { shakes, since, stopping: None }
    }

    /// Die away from `age` on.
    pub fn stop(&mut self, age: Age) {
        self.stopping.get_or_insert(age);
    }

    pub fn stopping(&self) -> bool {
        self.stopping.is_some()
    }

    /// How far the ground has moved at `age`, back where it was once stopped.
    fn displacement(&self, age: Age) -> Vec3 {
        let elapsed = age.elapsed_since(self.since).0;
        let rising = elapsed / SWELL.0;
        let falling = self.stopping.map_or(1.0, |stopping| 1.0 - age.elapsed_since(stopping).0 / SWELL.0);
        let swell = rising.min(falling).clamp(0.0, 1.0);
        let shaking: Vec3 = self
            .shakes
            .iter()
            .map(|shake| {
                let omega = TAU * shake.hertz;
                // Each shake's share of the acceleration, a = ω²·x.
                let amplitude = PEAK_ACCELERATION / SHAKES as f32 / (omega * omega);
                shake.toward * amplitude * (omega * elapsed + shake.phase).sin()
            })
            .sum();
        shaking * swell
    }
}

impl Fabric {
    /// How far the ground has moved from where it lies still.
    pub fn ground_shift(&self) -> Vec3 {
        self.quake.as_ref().map_or(Vec3::ZERO, |quake| quake.displacement(self.age))
    }

    /// Move every joint resting on the ground, within `contact` of it, as
    /// far as the ground moves in this tick.
    pub(crate) fn apply_quake(&mut self, contact: f32) {
        let Some(quake) = &self.quake else {
            return;
        };
        let mut next = self.age;
        next.tick();
        let shift = quake.displacement(next) - quake.displacement(self.age);
        let over = quake.stopping.is_some_and(|stopping| next.elapsed_since(stopping).0 >= SWELL.0);
        for joint in self.joints.values_mut().filter(|joint| joint.location.y <= contact) {
            joint.location += shift;
        }
        if over {
            self.quake = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::dsl::fabric_library::{self, FabricName};
    use crate::build::dsl::fabric_plan_executor::FabricPlanExecutor;
    use crate::fabric::physics::presets::VIEWING;
    use crate::fabric::physics::Physics;

    /// Headless Hug stands through 20 s of quake: it stays finite, and once
    /// the quake is stopped it comes back to where it stood.
    #[test]
    fn headless_hug_stands_through_a_quake() {
        let mut executor = FabricPlanExecutor::new(fabric_library::get_fabric_plan(FabricName::HeadlessHug));
        while !executor.is_complete() {
            let _ = executor.iterate();
        }
        let physics = Physics { surface: executor.physics.surface, ..VIEWING };
        let mut fabric = executor.fabric;
        fabric.take_motion();
        let before: Vec<Vec3> = fabric.joints.values().map(|joint| joint.location).collect();
        fabric.quake = Some(Quake::starting(fabric.age, 1));
        let per_second = Age::iterations_per_second() as usize;
        let mut furthest = 0.0f32;
        for tick in 0..35 * per_second {
            if tick == 20 * per_second {
                fabric.quake.as_mut().unwrap().stop(fabric.age);
            }
            fabric.iterate(&physics);
            if tick % 1000 == 0 {
                let moved = fabric.joints.values().zip(&before).map(|(joint, b)| joint.location.distance(*b));
                furthest = furthest.max(moved.fold(0.0, f32::max));
            }
        }
        assert!(fabric.quake.is_none(), "the quake is over");
        assert!(fabric.joints.values().all(|joint| joint.location.is_finite()), "the figure diverged");
        let after = fabric.joints.values().zip(&before).map(|(joint, b)| joint.location.distance(*b));
        let settled = after.fold(0.0, f32::max);
        let motion = fabric.take_motion().expect("motion was measured");
        eprintln!(
            "swayed {:.0} mm, {:.0} mm off after, cable max {:.1} kN, strut max {:.1} kN, slack {}",
            furthest * 1000.0,
            settled * 1000.0,
            motion.max_cable_force.to_kilonewtons(),
            motion.max_strut_force.to_kilonewtons(),
            motion.slack_cable_count
        );
        assert!(furthest > 0.01, "the quake moved it {:.0} mm", furthest * 1000.0);
        assert!(settled < 0.05, "it ended {:.0} mm from where it stood", settled * 1000.0);
    }
}
