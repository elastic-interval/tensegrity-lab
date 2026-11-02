//! A bridle takes two cables off one strut end through a free node: the
//! cables J–A and J–B become a link J–N and cables N–A and N–B, so the stack
//! at J carries one connector where it had two. In hardware the link is a
//! fork rod from J's cross-tube, and N is a triangular node plate the two
//! cables pin to (`connector::node_plate`). A cable from a plate too short
//! to make is a fork rod instead (`ConnectorSystem::link_length`).

use crate::connector::fork::BRIDLE_LINK;
use crate::connector::system::{hinge_struts, ConnectorSystem};
use crate::units::{Meters, Unit};
use crate::fabric::interval::Role;
use crate::fabric::interval::Span::Fixed;
use crate::fabric::{Fabric, IntervalKey, JointKey};

/// How much shorter than their span `tighten_bridles` makes a bridle's
/// cables.
const BRIDLE_PRETENSION: f32 = 0.005;

impl Fabric {
    /// Replace the cables `joint`–`a` and `joint`–`b` with a bridle hung on a
    /// fork rod from `joint`, its node plate `BRIDLE_LINK` out along their
    /// bisector, close enough to keep the forces much as they were. The
    /// physics then finds where it settles. At a hinge pin the link stays on the stack that carried
    /// the first cable. The node is labelled after `joint` (see
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
        self.bridle_hangers.insert(link);
        Some(node)
    }

    /// Keep every hinge clean: a hinge pin carries only its two struts, so a
    /// cable ending at one, with no strut of its own there, is split by a
    /// node plate, with a leg from the plate to the stack on each strut. The
    /// node sits half-way along the cable; the plate itself lies where its
    /// forces balance (`ConnectorSystem::build_plate`). The legs are two parallel intervals to the pin,
    /// each recorded as its strut's (`hinge_strut`). Returns what couldn't be
    /// split: each pin takes one, as its plate is named after it.
    pub fn split_hinge_cables(&mut self) -> Vec<String> {
        let mut failures = Vec::new();
        let pins: Vec<JointKey> = self.joints.keys().collect();
        for pin in pins {
            let Some(struts) = hinge_struts(self, pin) else {
                continue;
            };
            let cables: Vec<IntervalKey> = self
                .intervals
                .iter()
                .filter(|(key, interval)| {
                    interval.role.is_pull_like() && interval.touches(pin) && !self.hinge_strut.contains_key(&(*key, pin))
                })
                .map(|(key, _)| key)
                .collect();
            let at = self.location(pin);
            for (index, cable) in cables.into_iter().enumerate() {
                let far = self.intervals[cable].other_joint(pin);
                if index > 0 {
                    failures.push(format!("second cable at hinge {}, to {}", self.joint_label(pin), self.joint_label(far)));
                    continue;
                }
                let node = self.create_joint((at + self.location(far)) / 2.0);
                self.joints[node].label = self.joints[pin].label.map(|label| label.bridle());
                let role = self.remove_interval(cable).role;
                let legs = struts.map(|strut| {
                    let leg = self.create_slack_interval(pin, node, Role::Pulling);
                    self.hinge_strut.insert((leg, pin), strut);
                    leg
                });
                self.create_slack_interval(node, far, role);
                self.bridle_nodes.insert(node, legs[0]);
            }
        }
        failures
    }

    /// Pretension every bridle's cables, as `set_pretenst` does the struts:
    /// left exactly at span after `slacken`, they would let their free node
    /// drift until one hung slack.
    pub fn tighten_bridles(&mut self) {
        let nodes: Vec<JointKey> = self.bridle_nodes.keys().copied().collect();
        for node in nodes {
            for interval in self.intervals.values_mut().filter(|i| i.touches(node)) {
                if let Fixed { length } = interval.span {
                    interval.span = Fixed { length: length * (1.0 - BRIDLE_PRETENSION) };
                }
            }
        }
    }

    /// Shorten each bridle's link by as far as its plate has to slide to
    /// hang a hanger's length from the pivot on the strut end (see
    /// `NodePlate`): the node starts `BRIDLE_LINK` out, but where its hole
    /// lands depends on the cables' directions and on the slot its hanger
    /// gets, so the link is fitted once the slots are known, and again as
    /// the shape settles.
    pub fn hang_bridles(&mut self) {
        if self.bridle_nodes.is_empty() {
            return;
        }
        self.update_all_attachment_connections();
        let Some(connector) = self.connector.as_ref() else {
            return;
        };
        let excess: Vec<(IntervalKey, f32)> = self
            .bridle_nodes
            .iter()
            .filter_map(|(&node, &link)| {
                let host = self.intervals[link].other_joint(node);
                let slide = connector.node_plate(node)?.slide;
                let toward_host = (self.location(host) - self.location(node)).normalize();
                Some((link, slide.dot(toward_host)))
            })
            .collect();
        for (link, excess) in excess {
            if let Fixed { length } = self.intervals[link].span {
                self.intervals[link].span = Fixed { length: Meters(length.f32() - excess) };
            }
        }
        // The shape is still moving: leave no slots or plates behind
        // for the renderer until they are rebuilt on Viewing.
        if let Some(connector) = self.connector.as_mut() {
            *connector = ConnectorSystem::new(connector.dimensions.clone());
        }
    }
}
