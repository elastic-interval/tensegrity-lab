//! A bridle takes two cables off one strut end through a free node: the
//! cables J–A and J–B become a short link J–N and cables N–A and N–B, so the
//! stack at J carries one connector where it had two. In hardware N is a
//! clamp gripping one continuous cable A → N → B over a saddle, hung from
//! J's connector on a shackle (the link); see `connector::bridle_clamp`.

use crate::connector::fork::BRIDLE_LINK;
use crate::fabric::interval::Role;
use crate::fabric::{Fabric, IntervalKey, JointKey};
use crate::units::Unit;

impl Fabric {
    /// Replace the cables `joint`–`a` and `joint`–`b` with a bridle whose
    /// node starts `BRIDLE_LINK` out along their bisector; the physics then
    /// finds where it settles. At a hinge pin the link stays on the stack
    /// that carried the first cable. The node is labelled after `joint` (see
    /// `JointLabel::bridle`), so each joint can host one bridle. `None`,
    /// changing nothing, when either cable is missing, the two run in
    /// opposite directions, or `joint` already has a bridle.
    pub fn bridle(&mut self, joint: JointKey, a: JointKey, b: JointKey) -> Option<JointKey> {
        let cable = |far: JointKey| {
            self.joining((joint, far))
                .filter(|key| self.intervals[*key].role.is_pull_like())
        };
        let (cable_a, cable_b) = (cable(a)?, cable(b)?);
        let host = self.joints[joint].label;
        let hosting = |&link: &IntervalKey| self.intervals[link].touches(joint);
        if self.bridle_nodes.values().any(hosting) {
            return None;
        }
        let at = self.location(joint);
        let bisector = (self.location(a) - at).normalize() + (self.location(b) - at).normalize();
        if bisector.length() < 1e-3 {
            return None;
        }
        let node = self.create_joint(at + bisector.normalize() * BRIDLE_LINK.f32());
        self.joints[node].label = host.map(|label| label.bridle());
        let strut = self.hinge_strut.remove(&(cable_a, joint));
        self.hinge_strut.remove(&(cable_b, joint));
        let role_a = self.remove_interval(cable_a).role;
        let role_b = self.remove_interval(cable_b).role;
        let link = self.create_slack_interval(joint, node, Role::Pulling);
        self.create_slack_interval(node, a, role_a);
        self.create_slack_interval(node, b, role_b);
        if let Some(strut) = strut {
            self.hinge_strut.insert((link, joint), strut);
        }
        self.bridle_nodes.insert(node, link);
        Some(node)
    }
}
