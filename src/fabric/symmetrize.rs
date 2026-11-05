//! Making a mirror-symmetric fabric exactly symmetric. The simulation
//! settles a hair off — millimetres — but the real build is cut from
//! mirrored lengths, so connector work should see the symmetric shape: a
//! borderline collision on one side must not pass unnoticed on the other.

use crate::build::dsl::labelling::LabelSymmetry;
use crate::fabric::joint::JointLabel;
use crate::fabric::{Fabric, JointKey};
use glam::Vec3;
use std::collections::BTreeMap;

impl Fabric {
    /// Move each mirror pair of joints (by label) to the mean of one and the
    /// other's reflection, and each joint on the mirror plane onto it. The
    /// plane runs through the pairs' midpoints, square to their averaged
    /// separation, and is kept vertical: tilted to fit a fabric settled a hair
    /// off, it would lift one side's feet and guy anchors off the ground,
    /// where they come loose. No-op without a mirror label symmetry.
    pub fn symmetrize_mirror(&mut self) {
        let Some(symmetry @ LabelSymmetry::Mirror { .. }) = self.label_symmetry.clone() else {
            return;
        };
        let by_label: BTreeMap<JointLabel, JointKey> = self
            .joints
            .iter()
            .filter_map(|(key, joint)| joint.label.map(|label| (label, key)))
            .collect();
        let mut pairs: Vec<(JointKey, JointKey)> = Vec::new();
        let mut on_plane: Vec<JointKey> = Vec::new();
        for (&label, &key) in &by_label {
            let image = symmetry.map_label(label);
            if image == label {
                on_plane.push(key);
            } else if label < image {
                if let Some(&twin) = by_label.get(&image) {
                    pairs.push((key, twin));
                }
            }
        }
        let Some(&(first_a, first_b)) = pairs.first() else {
            return;
        };
        let location = |key: JointKey| self.joints[key].location;
        let reference = location(first_a) - location(first_b);
        let normal = pairs
            .iter()
            .map(|&(a, b)| {
                let apart = location(a) - location(b);
                if apart.dot(reference) < 0.0 {
                    -apart
                } else {
                    apart
                }
            })
            .sum::<Vec3>()
            .with_y(0.0)
            .normalize();
        let point = pairs.iter().map(|&(a, b)| (location(a) + location(b)) / 2.0).sum::<Vec3>()
            / pairs.len() as f32;
        let reflect = |p: Vec3| p - normal * (2.0 * (p - point).dot(normal));

        let moves: Vec<(JointKey, Vec3)> = pairs
            .iter()
            .flat_map(|&(a, b)| {
                let mean = (location(a) + reflect(location(b))) / 2.0;
                [(a, mean), (b, reflect(mean))]
            })
            .chain(on_plane.iter().map(|&key| {
                let p = location(key);
                (key, p - normal * (p - point).dot(normal))
            }))
            .collect();
        for (key, to) in moves {
            self.joints[key].location = to;
        }
    }
}
