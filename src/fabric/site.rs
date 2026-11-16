//! A building site: the fabric being put together or lifted by a crane.
//!
//! The builds' frozen surface never lets go of a joint, and the simulation's
//! struts only push, so one nothing compresses can be drawn out like a
//! telescope. On a site the ground holds what presses on it and lets go of
//! what lifts, every strut is pinned at its length, the crane's hook is held
//! where the driver has it, and ground anchors stay where they are driven in.

use crate::fabric::interval::{Role, Span};
use crate::fabric::{Fabric, JointKey};
use crate::units::{Meters, Unit};
use crate::Age;
use glam::Vec3;

/// How fast a joint dragged over the sliding ground loses its speed, per second.
const GROUND_DRAG: f32 = 20.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ground {
    /// Holding what presses on it where it is, as chocked feet are.
    Gripping,
    /// Letting what lies on it be dragged along, slowly.
    Sliding,
}

/// The crane's hook, and the eyes its sling legs go to.
#[derive(Clone, Debug)]
pub struct Hook {
    pub joint: JointKey,
    pub eyes: Vec<JointKey>,
    /// In metres above the ground.
    pub height: f32,
    /// Where the crane holds the hook, when not straight above the eyes.
    pub place: Option<Vec3>,
    /// How fast it is rising, in metres per second.
    pub climb: f32,
}

impl Hook {
    /// Sling legs `leg` long from one hook to `eyes`, the hook straight
    /// above them where the first leg comes taut.
    pub fn taut(fabric: &mut Fabric, eyes: Vec<JointKey>, leg: Meters) -> Hook {
        let above = Self::above(fabric, &eyes);
        let height = eyes
            .iter()
            .map(|&eye| {
                let at = fabric.location(eye);
                let aside = Vec3::new(at.x - above.x, 0.0, at.z - above.z).length();
                at.y + (leg.f32() * leg.f32() - aside * aside).max(0.0).sqrt()
            })
            .fold(f32::MAX, f32::min);
        Hook::at(fabric, eyes, leg, height, None)
    }

    /// The same, with the hook at `height`, held at `place` if given.
    pub fn at(fabric: &mut Fabric, eyes: Vec<JointKey>, leg: Meters, height: f32, place: Option<Vec3>) -> Hook {
        let above = place.unwrap_or_else(|| Self::above(fabric, &eyes));
        let joint = fabric.create_joint(Vec3::new(above.x, height, above.z));
        for &eye in &eyes {
            fabric.create_fixed_interval(eye, joint, Role::GuyLine, leg);
        }
        Hook { joint, eyes, height, place, climb: 0.0 }
    }

    /// The middle of the eyes.
    pub fn above(fabric: &Fabric, eyes: &[JointKey]) -> Vec3 {
        eyes.iter().map(|&eye| fabric.location(eye)).sum::<Vec3>() / eyes.len() as f32
    }
}

#[derive(Clone, Debug)]
pub struct Site {
    pub ground: Ground,
    pub hook: Option<Hook>,
    /// Ground anchors, which a guy line would otherwise lift.
    pub anchors: Vec<(JointKey, Vec3)>,
}

impl Site {
    pub fn on(ground: Ground) -> Site {
        Site { ground, hook: None, anchors: Vec::new() }
    }
}

impl Fabric {
    /// The site's hold on the fabric, at the end of a tick.
    pub(crate) fn apply_site(&mut self) {
        let Some(mut site) = self.site.take() else {
            return;
        };
        let dt = Age::iteration_duration().f32();
        if let Some(hook) = &mut site.hook {
            hook.height += hook.climb * dt;
            let above = hook.place.unwrap_or_else(|| Hook::above(self, &hook.eyes));
            let joint = &mut self.joints[hook.joint];
            joint.location = Vec3::new(above.x, hook.height, above.z);
            joint.velocity = Vec3::ZERO;
        }
        for &(anchor, at) in &site.anchors {
            self.joints[anchor].location = at;
            self.joints[anchor].velocity = Vec3::ZERO;
        }
        self.pin_struts();
        let kept = if site.ground == Ground::Sliding { 1.0 - GROUND_DRAG * dt } else { 0.0 };
        for joint in self.joints.values_mut().filter(|joint| joint.location.y < 0.0) {
            joint.location.y = 0.0;
            joint.velocity = Vec3::new(joint.velocity.x * kept, joint.velocity.y.max(0.0), joint.velocity.z * kept);
        }
        self.site = Some(site);
    }

    /// A strut stretched past its length, or past where it has got to while
    /// it lengthens, is brought back, its ends no longer parting. One being
    /// drawn in is left to go slack.
    fn pin_struts(&mut self) {
        for interval in self.intervals.values() {
            let drawn_in = matches!(interval.span, Span::Approaching { start_length, target_length, .. } if target_length < start_length);
            if interval.role != Role::Pushing || interval.is_out() || drawn_in {
                continue;
            }
            let length = interval.ideal();
            let (alpha, omega) = (interval.alpha_key, interval.omega_key);
            let along = self.joints[omega].location - self.joints[alpha].location;
            let spanned = along.length();
            if spanned <= length.f32() {
                continue;
            }
            let unit = along / spanned;
            let half = unit * (spanned - length.f32()) / 2.0;
            let parting = (self.joints[omega].velocity - self.joints[alpha].velocity).dot(unit).max(0.0) / 2.0;
            self.joints[alpha].location += half;
            self.joints[omega].location -= half;
            self.joints[alpha].velocity += unit * parting;
            self.joints[omega].velocity -= unit * parting;
        }
    }
}
