use std::collections::HashMap;

use cgmath::MetricSpace;

use crate::fabric::{Fabric, UniqueId};
use crate::fabric::interval::{Interval, Material, Role, Span};
use crate::fabric::joint::Joint;
use crate::fabric::joint_incident::{JointIncident, Path};
use crate::fabric::Link;

const ROOT3: f32 = 1.732_050_8;

const BOW_TIE_SHORTEN: f32 = 0.5;

impl Fabric {
    pub fn install_bow_ties(&mut self) {
        for Pair {
            alpha_index,
            omega_index,
            length,
        } in self
            .pair_generator()
            .bow_tie_pulls(&self.joints, &self.materials)
        {
            self.create_interval(
                alpha_index,
                omega_index,
                Link {
                    ideal: length,
                    material_name: ":bow-tie".to_string(),
                },
            );
        }
    }

    pub fn shorten_pulls(&mut self, strain_threshold: f32, shortening: f32) {
        for interval in self.intervals.values_mut() {
            if self.materials[interval.material].name != ":bow-tie" {
                continue;
            }
            if interval.strain > strain_threshold {
                interval.span = match interval.span {
                    Span::Fixed { length } => Span::Fixed {
                        length: length * shortening,
                    },
                    _ => unreachable!(),
                }
            }
        }
    }

    pub fn install_measures(&mut self) {
        for Pair {
            alpha_index,
            omega_index,
            length,
        } in self.pair_generator().proximity_measures()
        {
            self.create_interval(alpha_index, omega_index, Link::pull(length)); // todo: how to do measuring?
        }
    }

    pub fn faces_to_triangles(&mut self) -> Vec<UniqueId> {
        let joint_incident = self.joint_incidents();
        let mut faces_to_remove = vec![];
        for (id, face) in self.faces.clone() {
            let side_length = face.scale * ROOT3;
            let radial_joints = face.radial_joints(self);
            for (alpha, omega) in [(0, 1), (1, 2), (2, 0)] {
                let (alpha_index, omega_index) = (radial_joints[alpha], radial_joints[omega]);
                let overlap = joint_incident[alpha_index]
                    .pull_adjacent_joints
                    .intersection(&joint_incident[omega_index].pull_adjacent_joints)
                    .count();
                if overlap == 2 {
                    continue;
                }
                self.create_interval(alpha_index, omega_index, Link::pull(side_length));
            }
            faces_to_remove.push(id);
        }
        faces_to_remove
    }

    pub fn strain_limits(&self, material_name: String) -> (f32, f32) {
        let target_material = self.material(material_name);
        let choose_target =
            |&Interval {
                 strain, material, ..
             }| (material == target_material).then_some(strain);
        let max_strain = self
            .interval_values()
            .filter_map(choose_target)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(1.0);
        let min_strain = self
            .interval_values()
            .filter_map(choose_target)
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);
        (min_strain, max_strain)
    }

    fn pair_generator(&self) -> PairGenerator {
        PairGenerator::new(self.joint_incidents(), self.interval_map())
    }

    fn interval_map(&self) -> HashMap<(usize, usize), Interval> {
        self.interval_values()
            .map(|interval| (interval.key(), *interval))
            .collect()
    }
}





#[derive(Debug)]
struct Pair {
    alpha_index: usize,
    omega_index: usize,
    length: f32,
}

impl Pair {
    fn key(&self) -> (usize, usize) {
        if self.alpha_index < self.omega_index {
            (self.alpha_index, self.omega_index)
        } else {
            (self.omega_index, self.alpha_index)
        }
    }
}

struct PairGenerator {
    joints: Vec<JointIncident>,
    intervals: HashMap<(usize, usize), Interval>,
    pairs: HashMap<(usize, usize), Pair>,
}

impl PairGenerator {
    fn new(joints: Vec<JointIncident>, intervals: HashMap<(usize, usize), Interval>) -> Self {
        Self {
            joints,
            intervals,
            pairs: HashMap::new(),
        }
    }

    fn proximity_measures(mut self) -> impl Iterator<Item = Pair> {
        for joint in 0..self.joints.len() {
            self.push_proximity(joint);
        }
        self.pairs.into_values()
    }

    fn push_proximity(&mut self, joint_index: usize) {
        let Some(push) = &self.joints[joint_index].push else {
            return;
        };
        let length_limit = push.ideal();
        let new_pairs = self
            .joints
            .iter()
            .filter_map(|other_joint| {
                if joint_index == other_joint.index {
                    return None;
                }
                if self.joints[joint_index]
                    .adjacent_joints
                    .contains(&other_joint.index)
                {
                    return None;
                }
                let length = self.joints[joint_index]
                    .location
                    .distance(other_joint.location);
                if length > length_limit {
                    return None;
                }
                Some(Pair {
                    alpha_index: joint_index,
                    omega_index: other_joint.index,
                    length,
                })
            })
            .map(|pair| (pair.key(), pair));
        self.pairs.extend(new_pairs);
    }

    fn bow_tie_pulls(
        mut self,
        joints: &[Joint],
        materials: &[Material],
    ) -> impl Iterator<Item = Pair> {
        for interval in self.intervals.values() {
            if materials[interval.material].role != Role::Push {
                continue;
            }
            let mut meeting_pairs = vec![];
            for alpha_path in self.paths_for(interval.alpha_index, 2) {
                for omega_path in self.paths_for(interval.omega_index, 2) {
                    if alpha_path.last_interval().key() == omega_path.last_interval().key() {
                        // second interval is the bridge
                        meeting_pairs.push((6, alpha_path.clone(), omega_path.clone()));
                    }
                    if alpha_path.last_joint() == omega_path.last_joint() {
                        meeting_pairs.push((8, alpha_path.clone(), omega_path.clone()));
                    }
                }
            }
            meeting_pairs.sort_by_key(|(size, _, _)| *size);
            match meeting_pairs.as_slice() {
                [(6, alpha1, omega1), (6, alpha2, omega2), ..] => {
                    let diagonals = [
                        (alpha1.last_joint(), omega2.last_joint()),
                        (alpha2.last_joint(), omega1.last_joint()),
                    ];
                    let cross_twist_diagonals: Vec<_> = diagonals
                        .iter()
                        .filter_map(|&(a, b)| {
                            if self.interval_exists(
                                self.joints[a].across_push()?,
                                self.joints[b].across_push()?,
                            ) {
                                return None;
                            }
                            Some((a, b))
                        })
                        .collect();
                    if let &[(alpha_index, omega_index)] = cross_twist_diagonals.as_slice() {
                        let distance = joints[alpha_index]
                            .location
                            .distance(joints[omega_index].location);
                        let pair = Pair {
                            alpha_index,
                            omega_index,
                            length: distance * BOW_TIE_SHORTEN,
                        };
                        self.pairs.insert(pair.key(), pair);
                    } else {
                        let candidate_completions = [
                            (alpha1, alpha2),
                            (alpha2, alpha1),
                            (omega1, omega2),
                            (omega2, omega1),
                        ];
                        let triangle_completions: Vec<_> = candidate_completions
                            .iter()
                            .filter_map(|&(path, other_path)| {
                                if self.joints[other_path.joint_indices[1]].push.is_some() {
                                    return None;
                                }
                                Some((path.joint_indices[0], path.last_joint()))
                            })
                            .collect();
                        if let &[(alpha_index, omega_index)] = triangle_completions.as_slice() {
                            let distance = joints[alpha_index]
                                .location
                                .distance(joints[omega_index].location);
                            let pair = Pair {
                                alpha_index,
                                omega_index,
                                length: distance * BOW_TIE_SHORTEN,
                            };
                            self.pairs.insert(pair.key(), pair);
                        }
                    }
                }
                [(8, alpha1, omega1), (8, alpha2, omega2)] => {
                    let candidates = [
                        (alpha1, alpha2.last_joint()),
                        (alpha2, alpha1.last_joint()),
                        (omega1, omega2.last_joint()),
                        (omega2, omega1.last_joint()),
                    ];
                    for (path, omega_index) in candidates {
                        let alpha_index = path.joint_indices[1];
                        if self.joints[alpha_index].push.is_none() {
                            continue;
                        }
                        let pair = Pair {
                            alpha_index,
                            omega_index,
                            length: interval.ideal() / 4.0,
                        };
                        self.pairs.insert(pair.key(), pair);
                    }
                }
                _ => {}
            }
        }

        self.pairs.into_values()
    }

    fn interval_exists(&self, a: usize, b: usize) -> bool {
        if a < b {
            self.intervals.contains_key(&(a, b))
        } else {
            self.intervals.contains_key(&(b, a))
        }
    }

    fn paths_for(&self, joint_index: usize, max_size: usize) -> Vec<Path> {
        let paths: Vec<_> = self.joints[joint_index]
            .pulls
            .iter()
            .map(|pull| Path::new(joint_index, *pull))
            .collect();
        self.paths_via_pulls(&paths, 1, max_size)
    }

    fn paths_via_pulls(&self, paths: &[Path], size: usize, max_size: usize) -> Vec<Path> {
        if size == max_size {
            paths.to_vec()
        } else {
            let bigger: Vec<_> = paths
                .iter()
                .flat_map(|path| self.joints[path.last_joint()].extended_paths(path))
                .collect();
            self.paths_via_pulls(&bigger, size + 1, max_size)
        }
    }
}
