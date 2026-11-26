//! A movement sensor on a joint of a piece, as a small accelerometer on a
//! node would be: it feels the joint move, and each time the joint's swing
//! turns round, it tells how hard the joint was moving. The music comes
//! from these, so from the tensegrity's own movement, not from the table
//! that sets it going.

use crate::fabric::{Fabric, JointKey};
use glam::Vec3;

/// How many ticks between readings: a thousand a second.
pub const SENSING: usize = 20;
/// How long the joint's movement is smoothed over, seconds, so the ringing
/// of its cords isn't taken for a swing.
const SMOOTHING: f32 = 0.03;
/// Slower than this, metres a second, the joint is between swings.
const STILL: f32 = 0.02;
/// The shortest time between turns, seconds.
const SHORTEST: f32 = 0.12;
/// A swing this fast at its fastest, metres a second, is a full one.
const FULL: f32 = 0.3;

pub struct Sensor {
    key: JointKey,
    /// How the joint moves, smoothed; how it moved when it moved fastest
    /// since it last turned, and how fast that was; and when it turned,
    /// seconds. It turns when it comes round to move against its fastest,
    /// whether it swings to and fro or round in a loop.
    velocity: Vec3,
    fastest: Vec3,
    peak: f32,
    turned: f32,
}

impl Sensor {
    /// On the joint named `name` in `fabric`, if it has one.
    pub fn on(fabric: &Fabric, name: &str) -> Option<Sensor> {
        let key = fabric.joints.iter().find(|(_, joint)| joint.label.is_some_and(|label| label.to_string() == name))?.0;
        Some(Sensor { key, velocity: Vec3::ZERO, fastest: Vec3::ZERO, peak: 0.0, turned: f32::MIN })
    }

    /// A reading of `fabric` at `now`, seconds, `dt` after the last: how
    /// hard, if the joint's swing turned round.
    pub fn read(&mut self, fabric: &Fabric, now: f32, dt: f32) -> Option<f32> {
        let velocity = fabric.joints.get(self.key)?.velocity;
        self.feel(velocity, now, dt)
    }

    fn feel(&mut self, velocity: Vec3, now: f32, dt: f32) -> Option<f32> {
        self.velocity += (velocity - self.velocity) * (dt / SMOOTHING).min(1.0);
        let speed = self.velocity.length();
        if speed > self.peak {
            (self.peak, self.fastest) = (speed, self.velocity);
        }
        if speed < STILL || self.fastest.dot(self.velocity) >= 0.0 || now - self.turned < SHORTEST {
            return None;
        }
        let hard = (self.peak / FULL).min(1.5);
        (self.peak, self.fastest, self.turned) = (speed, self.velocity, now);
        Some(hard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    /// A joint swinging to and fro turns at each end, twice a cycle, as
    /// hard as it swung; and so does one going round in a loop.
    #[test]
    fn a_swing_turns_at_each_end() {
        let mut sensor = Sensor { key: JointKey::default(), velocity: Vec3::ZERO, fastest: Vec3::ZERO, peak: 0.0, turned: f32::MIN };
        let (hertz, fastest, dt) = (1.5, 0.3, 0.001);
        let turns: Vec<f32> = (0..4000)
            .filter_map(|tick| {
                let now = tick as f32 * dt;
                let velocity = Vec3::X * fastest * (TAU * hertz * now).cos();
                sensor.feel(velocity, now, dt)
            })
            .collect();
        assert_eq!(turns.len(), 12, "{turns:?}");
        assert!(turns.iter().skip(1).all(|hard| (hard - 1.0).abs() < 0.1), "{turns:?}");
        let mut sensor = Sensor { key: JointKey::default(), velocity: Vec3::ZERO, fastest: Vec3::ZERO, peak: 0.0, turned: f32::MIN };
        let looping = (0..4000)
            .filter(|tick| {
                let phase = TAU * hertz * *tick as f32 * dt;
                let velocity = Vec3::new(phase.cos(), 0.0, 0.5 * phase.sin()) * fastest;
                sensor.feel(velocity, *tick as f32 * dt, dt).is_some()
            })
            .count();
        assert!((11..=12).contains(&looping), "{looping} turns going round");
    }
}
