//! One-step changes to a brick's topology, named by joint so a change can
//! be carried round the brick's symmetry.

use crate::build::dsl::brick::{BrickPrototype, FaceDef, PullDef};
use crate::build::dsl::brick_dsl::JointName;
use crate::build::dsl::Spin;
use glam::{Quat, Vec3};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    RemoveStrut(JointName, JointName),
    RemovePull(JointName, JointName),
    RemoveFace([JointName; 3]),
    AddPull(JointName, JointName),
    AddFace([JointName; 3]),
    /// Face, the vertex leaving it, the strut end taking its place.
    Rewire([JointName; 3], JointName, JointName),
}

/// A settled prototype's geometry by joint name, unscaled to prototype lengths.
pub struct Shape {
    pub location: Vec<(JointName, Vec3)>,
    pub scale: f32,
}

impl Shape {
    fn at(&self, name: JointName) -> Vec3 {
        self.location.iter().find(|(n, _)| *n == name).unwrap().1
    }
}

fn ends(proto: &BrickPrototype) -> Vec<JointName> {
    proto.pushes.iter().flat_map(|p| [p.alpha, p.omega]).collect()
}

fn strut_of(proto: &BrickPrototype, end: JointName) -> usize {
    proto.pushes.iter().position(|p| p.alpha == end || p.omega == end).unwrap()
}

fn same_pair(a: (JointName, JointName), b: (JointName, JointName)) -> bool {
    a == b || a == (b.1, b.0)
}

fn same_face(a: [JointName; 3], b: [JointName; 3]) -> bool {
    a.iter().all(|j| b.contains(j))
}

/// Every one-step change to the prototype.
pub fn changes(proto: &BrickPrototype) -> Vec<Change> {
    let ends = ends(proto);
    let mut changes = vec![];
    for push in &proto.pushes {
        changes.push(Change::RemoveStrut(push.alpha, push.omega));
    }
    for pull in &proto.pulls {
        changes.push(Change::RemovePull(pull.alpha, pull.omega));
    }
    for face in &proto.faces {
        changes.push(Change::RemoveFace(face.joints));
    }
    for (i, &a) in ends.iter().enumerate() {
        for &b in &ends[i + 1..] {
            let pulled = proto.pulls.iter().any(|p| same_pair((p.alpha, p.omega), (a, b)));
            if strut_of(proto, a) != strut_of(proto, b) && !pulled {
                changes.push(Change::AddPull(a, b));
            }
        }
    }
    for (i, &a) in ends.iter().enumerate() {
        for (j, &b) in ends.iter().enumerate().skip(i + 1) {
            for &c in &ends[j + 1..] {
                let struts: BTreeSet<_> = [a, b, c].iter().map(|&e| strut_of(proto, e)).collect();
                let exists = proto.faces.iter().any(|f| same_face(f.joints, [a, b, c]));
                if struts.len() == 3 && !exists {
                    changes.push(Change::AddFace([a, b, c]));
                }
            }
        }
    }
    for face in &proto.faces {
        for &leaving in &face.joints {
            for &arriving in &ends {
                let others: Vec<_> = face.joints.iter().copied().filter(|&j| j != leaving).collect();
                let struts: BTreeSet<_> = others.iter().chain([&arriving]).map(|&e| strut_of(proto, e)).collect();
                let replaced = face.joints.map(|j| if j == leaving { arriving } else { j });
                let exists = proto.faces.iter().any(|f| same_face(f.joints, replaced));
                if struts.len() == 3 && !exists {
                    changes.push(Change::Rewire(face.joints, leaving, arriving));
                }
            }
        }
    }
    changes
}

/// The prototype with the changes applied. A new pull rests 10% short of
/// its span in the parent's shape, so it starts out taut.
pub fn apply(proto: &BrickPrototype, changes: &[Change], parent: &Shape) -> BrickPrototype {
    let mut proto = proto.clone();
    for change in changes {
        match *change {
            Change::RemoveStrut(alpha, omega) => {
                proto.pushes.retain(|p| !same_pair((p.alpha, p.omega), (alpha, omega)));
                let gone = |j: JointName| j == alpha || j == omega;
                proto.pulls.retain(|p| !gone(p.alpha) && !gone(p.omega));
                proto.faces.retain(|f| !f.joints.iter().any(|&j| gone(j)));
            }
            Change::RemovePull(alpha, omega) => {
                proto.pulls.retain(|p| !same_pair((p.alpha, p.omega), (alpha, omega)))
            }
            Change::RemoveFace(joints) => proto.faces.retain(|f| !same_face(f.joints, joints)),
            Change::AddPull(alpha, omega)
                if proto.pulls.iter().any(|p| same_pair((p.alpha, p.omega), (alpha, omega))) => {}
            Change::AddFace(joints) if proto.faces.iter().any(|f| same_face(f.joints, joints)) => {}
            Change::AddPull(alpha, omega) => {
                let span = (parent.at(omega) - parent.at(alpha)).length();
                proto.pulls.push(PullDef {
                    alpha,
                    omega,
                    ideal: span / parent.scale / 1.1,
                    material: "pull".to_string(),
                });
            }
            Change::AddFace(joints) => proto.faces.push(FaceDef {
                spin: Spin::Left,
                joints,
                aliases: vec![],
                scale_overrides: vec![],
            }),
            Change::Rewire(joints, leaving, arriving) => {
                for face in proto.faces.iter_mut().filter(|f| same_face(f.joints, joints)) {
                    face.joints = face.joints.map(|j| if j == leaving { arriving } else { j });
                }
            }
        }
    }
    proto
}

/// Where each joint goes under one turn of the symmetry, found from the
/// settled shape. `None` if some joint has no image there.
pub fn turn(shape: &Shape, axis: Vec3, order: u32) -> Option<Vec<(JointName, JointName)>> {
    let rotation = Quat::from_axis_angle(axis, std::f32::consts::TAU / order as f32);
    let centre = shape.location.iter().map(|(_, p)| *p).sum::<Vec3>() / shape.location.len() as f32;
    let size = shape.location.iter().map(|(_, p)| (*p - centre).length()).fold(0.0, f32::max);
    shape
        .location
        .iter()
        .map(|(name, p)| {
            let target = centre + rotation * (*p - centre);
            let (image, at) = shape.location.iter().min_by(|a, b| {
                (a.1 - target).length().partial_cmp(&(b.1 - target).length()).unwrap()
            })?;
            ((*at - target).length() < 1.0e-3 * size).then_some((*name, *image))
        })
        .collect()
}

/// The change together with its images under every turn of the symmetry.
pub fn orbit(change: &Change, turn: &[(JointName, JointName)], order: u32) -> Vec<Change> {
    let image = |j: JointName| turn.iter().find(|(from, _)| *from == j).unwrap().1;
    let mut images = vec![change.clone()];
    for _ in 1..order {
        let last = images.last().unwrap();
        let next = match *last {
            Change::RemoveStrut(a, b) => Change::RemoveStrut(image(a), image(b)),
            Change::RemovePull(a, b) => Change::RemovePull(image(a), image(b)),
            Change::RemoveFace(f) => Change::RemoveFace(f.map(image)),
            Change::AddPull(a, b) => Change::AddPull(image(a), image(b)),
            Change::AddFace(f) => Change::AddFace(f.map(image)),
            Change::Rewire(f, l, a) => Change::Rewire(f.map(image), image(l), image(a)),
        };
        images.push(next);
    }
    images
}
