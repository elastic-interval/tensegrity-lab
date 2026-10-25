//! A bridle takes two cables off one strut end through a free node: the
//! cables J–A and J–B become a link J–N and cables N–A and N–B, so the stack
//! at J carries one connector where it had two. In hardware the link is a
//! shackle from J's cross-tube, and N is either a triangular node plate the
//! two cables' terminals pin to (`connector::node_plate`), or, where a cable
//! from a plate would be too short to make, a clamp gripping one continuous
//! cable A → N → B over a saddle (`connector::bridle_clamp`).

use crate::connector::fork::{min_cable_length, BRIDLE_LINK, CLAMP_LINK, PLATE_HOLE_ALLOWANCE};
use crate::connector::system::ConnectorSystem;
use crate::units::{Meters, Unit};
use crate::fabric::interval::Role;
use crate::fabric::interval::Span::Fixed;
use crate::fabric::{Fabric, IntervalKey, JointKey};

/// How much shorter than their span `tighten_bridles` makes a bridle's
/// cables.
const BRIDLE_PRETENSION: f32 = 0.005;

impl Fabric {
    /// Replace the cables `joint`–`a` and `joint`–`b` with a bridle hung on a
    /// shackle from `joint`, its node out along their bisector, close enough
    /// to keep the forces much as they were. The node is a plate,
    /// `BRIDLE_LINK` out, unless a cable from there to `a` or `b` would be too
    /// short to make; then it is a clamp, `CLAMP_LINK` out, its continuous
    /// cable needing only one terminal a side. The physics then finds where
    /// it settles. At a hinge pin the link stays on the stack that carried
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
        // A cable from the node needs a minimum length pin to pin; measured
        // joint to joint, that adds what the pins lose at either end: the
        // hole's offset in the plate, as much again at the strut's
        // cross-tube or the next plate, and a centimetre to spare.
        let min = min_cable_length() + 2.0 * PLATE_HOLE_ALLOWANCE.f32() + 0.010;
        let ends = [at, self.location(a), self.location(b)];
        let plate_at = at + bisector.normalize() * BRIDLE_LINK.f32();
        let clamp = !ends[1..].iter().all(|end| plate_at.distance(*end) >= min);
        let node_at = if clamp { at + bisector.normalize() * CLAMP_LINK.f32() } else { plate_at };
        let node = self.create_joint(node_at);
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
        self.bridle_shackles.insert(link);
        if clamp {
            self.bridle_clamps.insert(node);
        }
        Some(node)
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

    /// Shorten each clamp's link by as far as the clamp has to slide to hang
    /// its eye a shackle's length from the pivot on the strut end (see
    /// `BridleClamp`): the clamp starts `CLAMP_LINK` out, but where its eye
    /// lands depends on the bend and on the slot its shackle gets, so the
    /// link is fitted once the slots are known, and again as the shape
    /// settles.
    pub fn hang_clamps(&mut self) {
        if self.bridle_clamps.is_empty() {
            return;
        }
        self.update_all_attachment_connections();
        let Some(connector) = self.connector.as_ref() else {
            return;
        };
        let excess: Vec<(IntervalKey, f32)> = self
            .bridle_clamps
            .iter()
            .filter_map(|&node| {
                let link = self.bridle_nodes[&node];
                let host = self.intervals[link].other_joint(node);
                let slide = connector.bridle_clamp(node)?.corner - self.location(node);
                let toward_host = (self.location(host) - self.location(node)).normalize();
                Some((link, slide.dot(toward_host)))
            })
            .collect();
        for (link, excess) in excess {
            if let Fixed { length } = self.intervals[link].span {
                self.intervals[link].span = Fixed { length: Meters(length.f32() - excess) };
            }
        }
        // The shape is still moving: leave no slots, plates or clamps behind
        // for the renderer until they are rebuilt on Viewing.
        if let Some(connector) = self.connector.as_mut() {
            *connector = ConnectorSystem::new(connector.dimensions.clone());
        }
    }
}
