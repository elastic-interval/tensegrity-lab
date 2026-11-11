//! A simple wind: a steady breeze with a gust swelling over it and dying
//! away again, over and over, blowing horizontally from one direction.
//! Each strut and cable feels drag from the wind across it,
//! ½ · ρ · Cd · diameter · length · v², shared between its two ends.
//!
//! Rough on purpose: for seeing how the figure answers a gust, and for
//! comparing designs. The design wind loads, from the code for the site,
//! are the engineer's.

use crate::fabric::interval::Role;
use crate::fabric::Fabric;
use crate::units::{Newtons, Seconds};
use crate::Age;
use glam::Vec3;
use std::f32::consts::TAU;

const AIR_DENSITY: f32 = 1.25;
/// A cylinder across the wind.
const DRAG_COEFFICIENT: f32 = 1.2;

#[derive(Clone, Debug)]
pub struct Wind {
    /// Horizontal, the way the wind blows.
    pub toward: Vec3,
    /// Metres per second between gusts.
    pub speed: f32,
    /// Metres per second at a gust's height.
    pub gust_speed: f32,
    /// From one gust to the next.
    pub period: Seconds,
    pub since: Age,
}

impl Wind {
    /// A fresh breeze, 10 m/s gusting to 15 every 8 s, blowing `toward`.
    pub fn gusting(toward: Vec3, since: Age) -> Wind {
        Wind {
            toward: Vec3::new(toward.x, 0.0, toward.z).normalize(),
            speed: 10.0,
            gust_speed: 15.0,
            period: Seconds(8.0),
            since,
        }
    }

    /// Metres per second at `age`: rising smoothly to the gust and back.
    pub fn speed_at(&self, age: Age) -> f32 {
        let phase = age.elapsed_since(self.since) / self.period;
        let swell = 0.5 * (1.0 - (TAU * phase).cos());
        self.speed + (self.gust_speed - self.speed) * swell
    }
}

impl Fabric {
    /// Add the wind's drag on every member to its joints' forces, in the
    /// simulation's millinewtons.
    pub(crate) fn apply_wind(&mut self) {
        let Some(wind) = &self.wind else {
            return;
        };
        let speed = wind.speed_at(self.age);
        let flow = wind.toward * speed;
        let (push_diameter, pull_diameter) = (2.0 * self.dimensions.push_radius.0, 2.0 * self.dimensions.pull_radius.0);
        for interval in self.intervals.values() {
            if interval.has_role(Role::Support) {
                continue;
            }
            let (alpha, omega) = interval.locations(&self.joints);
            let along = omega - alpha;
            let length = along.length();
            if length < 1e-6 {
                continue;
            }
            let across = flow - along * (flow.dot(along) / (length * length));
            let diameter = if interval.has_role(Role::Pushing) { push_diameter } else { pull_diameter };
            let drag = Newtons(0.5 * AIR_DENSITY * DRAG_COEFFICIENT * diameter * length * across.length() * across.length());
            let force = across.normalize_or_zero() * (drag.to_millinewtons() / 2.0);
            self.joints[interval.alpha_key].force += force;
            self.joints[interval.omega_key].force += force;
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

    /// Headless Hug standing, and the physics it is viewed with.
    fn standing() -> (Fabric, Physics) {
        let mut executor = FabricPlanExecutor::new(fabric_library::get_fabric_plan(FabricName::HeadlessHug));
        while !executor.is_complete() {
            let _ = executor.iterate();
        }
        let physics = Physics { surface: executor.physics.surface, ..VIEWING };
        executor.fabric.take_motion();
        (executor.fabric, physics)
    }

    fn apex(fabric: &Fabric) -> Vec3 {
        fabric.joints.values().map(|joint| joint.location).fold(Vec3::ZERO, |top, p| if p.y > top.y { p } else { top })
    }

    /// Blow `toward` for two gusts: how far the apex moves downwind at
    /// most, and the fabric's motion over that time.
    fn blow(toward: Vec3) -> (f32, f32, crate::fabric::FabricMotion) {
        let (mut fabric, physics) = standing();
        let start = apex(&fabric);
        fabric.wind = Some(Wind::gusting(toward, fabric.age));
        let mut furthest = 0.0f32;
        let ticks = (Seconds(1.0) / Age::iteration_duration()) as usize;
        for _ in 0..16 {
            for _ in 0..ticks {
                fabric.iterate(&physics);
            }
            furthest = furthest.max((apex(&fabric) - start).dot(toward.normalize()));
        }
        let finite = fabric.joints.values().all(|joint| joint.location.is_finite());
        assert!(finite, "the figure diverged in the wind");
        let motion = fabric.take_motion().expect("motion was measured");
        (furthest, apex(&fabric).y, motion)
    }

    /// Headless Hug in a gust from the front leans back with it, a little,
    /// and stands.
    #[test]
    fn headless_hug_leans_in_the_wind() {
        let (downwind, _, motion) = blow(Vec3::new(0.0, 0.0, -1.0));
        assert!(downwind > 0.005, "the apex moved {downwind:.3} m downwind");
        assert!(downwind < 1.0, "the apex moved {downwind:.3} m downwind");
        assert!(motion.max_cable_force.to_kilonewtons() < 20.0);
    }

    /// The wind from five directions round one side (the other side mirrors
    /// them), two gusts each: how far the apex is blown downwind, the highest
    /// cable and strut forces, and the cables slack at the end.
    ///
    /// ```
    /// cargo test --release --lib measure_headless_hug_in_wind -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore]
    fn measure_headless_hug_in_wind() {
        eprintln!("from        apex  cable max  strut max  slack (10 m/s gusting to 15)");
        for (name, toward) in [
            ("front", Vec3::new(0.0, 0.0, -1.0)),
            ("front-side", Vec3::new(1.0, 0.0, -1.0)),
            ("side", Vec3::new(1.0, 0.0, 0.0)),
            ("back-side", Vec3::new(1.0, 0.0, 1.0)),
            ("back", Vec3::new(0.0, 0.0, 1.0)),
        ] {
            let (downwind, _, motion) = blow(toward);
            eprintln!(
                "{name:<10} {:>4.0} mm  {:>6.1} kN  {:>6.1} kN  {:>3}",
                downwind * 1000.0,
                motion.max_cable_force.to_kilonewtons(),
                motion.max_strut_force.to_kilonewtons(),
                motion.slack_cable_count
            );
        }
    }
}
