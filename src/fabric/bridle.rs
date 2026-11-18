//! A bridle takes two cables off one strut end through a free node: the
//! cables J–A and J–B become a link J–N and cables N–A and N–B, so the stack
//! at J carries one connector where it had two. In hardware the link is a
//! fork rod from J's cross-tube, and N is a triangular node plate the two
//! cables pin to (`connector::node_plate`). A cable from a plate too short
//! to make is a fork rod instead (`ConnectorSystem::link_length`).

use crate::connector::fork::BRIDLE_LINK;
use crate::connector::system::{hinge_struts, ConnectorSystem};
use crate::units::{Meters, Percent, Unit};
use glam::Vec3;
use crate::fabric::interval::Role;
use crate::fabric::joint::JointLabel;
use crate::fabric::interval::Span::Fixed;
use crate::fabric::{Fabric, IntervalKey, JointKey};

impl Fabric {
    /// Replace the cables from `joint` to each of `fars` with a bridle hung on
    /// a fork rod from `joint`, its node plate `BRIDLE_LINK` out along their
    /// mean direction, close enough to keep the forces much as they were.
    /// The physics then finds where it settles. At a hinge pin the link stays
    /// on the stack that carried the first cable. The node is labelled after
    /// `joint` (see `JointLabel::bridle`), so each joint can host one bridle,
    /// except that a joint on the mirror plane can host one on each side,
    /// its two cables both to one side's twins (`Z2bL`, `Z2bR`). `None`,
    /// changing nothing, when a cable is missing, the cables pull every way
    /// at once, or `joint` already has a bridle with that label.
    pub fn bridle(&mut self, joint: JointKey, fars: &[JointKey]) -> Option<JointKey> {
        let cable = |far: JointKey| {
            self.joining((joint, far))
                .filter(|key| self.intervals[*key].role.is_pull_like())
        };
        let cables: Vec<IntervalKey> = fars.iter().map(|&far| cable(far)).collect::<Option<_>>()?;
        let side = |far: JointKey| match self.joints[far].label {
            Some(JointLabel::Twin { letter, .. }) => Some(letter),
            _ => None,
        };
        let one_side = match fars {
            [a, b] => side(*a).filter(|&letter| side(*b) == Some(letter)),
            _ => None,
        };
        let label = match (self.joints[joint].label, one_side) {
            (Some(JointLabel::Axial { index }), Some(letter)) => Some(JointLabel::BridleAxialTwin { index, letter }),
            (host, _) => host.map(|label| label.bridle()),
        };
        let hosted = |(&node, &link): (&JointKey, &IntervalKey)| {
            self.intervals[link].touches(joint) && self.joints[node].label == label
        };
        if self.bridle_nodes.iter().any(hosted) {
            return None;
        }
        let at = self.location(joint);
        let bisector: Vec3 = fars.iter().map(|&far| (self.location(far) - at).normalize()).sum();
        if bisector.length() < 1e-3 {
            return None;
        }
        let node = self.create_joint(at + bisector.normalize() * BRIDLE_LINK.f32());
        self.joints[node].label = label;
        let strut = self.hinge_strut.remove(&(cables[0], joint));
        for cable in &cables[1..] {
            self.hinge_strut.remove(&(*cable, joint));
        }
        let roles: Vec<Role> = cables.iter().map(|&cable| self.remove_interval(cable).role).collect();
        let link = self.create_slack_interval(joint, node, Role::Pulling);
        for (&far, role) in fars.iter().zip(roles) {
            self.create_slack_interval(node, far, role);
        }
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
    /// Give each node plate and hinge pin the mass of its hardware, when the
    /// fabric counts masses from its real parts.
    pub fn place_hardware_masses(&mut self) {
        let Some(hardware) = self.dimensions.hardware else {
            return;
        };
        let nodes: Vec<JointKey> = self.bridle_nodes.keys().copied().collect();
        for node in nodes {
            self.joints[node].point_mass = Some(hardware.plate);
        }
        let pins: Vec<JointKey> = self.joints.keys().filter(|&joint| hinge_struts(self, joint).is_some()).collect();
        for pin in pins {
            self.joints[pin].point_mass = Some(hardware.hinge);
        }
    }

    /// Shorten each bridle's cables by `pretenst` of their span, as tight as
    /// the struts' extension makes the rest of the fabric.
    pub fn tighten_bridles(&mut self, pretenst: Percent) {
        let nodes: Vec<JointKey> = self.bridle_nodes.keys().copied().collect();
        for node in nodes {
            for interval in self.intervals.values_mut().filter(|i| i.touches(node)) {
                if let Fixed { length } = interval.span {
                    interval.span = Fixed { length: length * (1.0 - pretenst.as_factor()) };
                }
            }
        }
    }

    /// Four cables or more on one plate don't find their own balance as three
    /// do: where the plate settles decides what each carries, and a short one
    /// can come out slack. Cut any carrying less than `pretenst` to carry it
    /// where the plate is now. Plates with three cables are left alone.
    pub fn tension_plates(&mut self, pretenst: Percent) {
        let factor = pretenst.as_factor();
        let nodes: Vec<JointKey> = self.bridle_nodes.keys().copied().collect();
        for node in nodes {
            let cables: Vec<IntervalKey> = self
                .intervals
                .iter()
                .filter(|(_, i)| i.touches(node) && i.role.is_pull_like())
                .map(|(key, _)| key)
                .collect();
            if cables.len() <= 3 {
                continue;
            }
            for cable in cables {
                let length = self.intervals[cable].length(&self.joints).f32();
                if let Fixed { length: rest } = self.intervals[cable].span {
                    if length / rest.f32() - 1.0 < factor {
                        self.intervals[cable].span = Fixed { length: Meters(length / (1.0 + factor)) };
                    }
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
