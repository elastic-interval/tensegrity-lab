use std::collections::{HashMap, HashSet};
use cgmath::{MetricSpace, Point3};

use crate::fabric::{Fabric, UniqueId};
use crate::fabric::interval::{Interval, Material, Role};
use crate::fabric::joint::Joint;
use crate::fabric::Link;

const ROOT3: f32 = 1.732_050_8;

const BOW_TIE_MATERIAL: Material = Material {
    role: Role::Pull,
    stiffness: 0.7,
    mass: 0.1,
};
const BOW_TIE_SHORTEN: f32 = 0.5;

impl Fabric {
    pub(crate) const BOW_TIE_MATERIAL: usize = 2;

    pub fn default_bow_tie() -> Self {
        let mut fabric = Fabric::default();
        fabric.materials.push(BOW_TIE_MATERIAL);
        fabric
    }

    pub fn install_bow_ties(&mut self) {
        for Pair { alpha_index, omega_index, length } in self.pair_generator().bow_tie_pulls(&self.joints, &self.materials) {
            self.create_interval(alpha_index, omega_index, Link { ideal: length, material: Fabric::BOW_TIE_MATERIAL });
        }
    }

    pub fn install_measures(&mut self) {
        for Pair { alpha_index, omega_index, length } in self.pair_generator().proximity_measures() {
            self.create_interval(alpha_index, omega_index, Link::pull(length)); // todo: how to do measuring?
        }
    }

    pub fn replace_faces(&mut self) -> Vec<UniqueId> {
        let joint_incident = self.joint_incident();
        let mut faces_to_remove = vec![];
        for (id, face) in self.faces.clone() {
            let side_length = face.scale * ROOT3;
            let radial_joints = face.radial_joints(self);
            for (alpha, omega) in [(0, 1), (1, 2), (2, 0)] {
                let (alpha_index, omega_index) = (radial_joints[alpha], radial_joints[omega]);
                let overlap = joint_incident[alpha_index].pull_adjacent_joints
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

    pub fn strain_limits(&self, target_material: usize) -> (f32, f32) {
        let choose_target = |&Interval { strain, material, .. }|
            (material == target_material).then_some(strain);
        let max_strain = self.interval_values()
            .filter_map(choose_target)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(1.0);
        let min_strain = self.interval_values()
            .filter_map(choose_target)
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);
        (min_strain, max_strain)
    }

    fn pair_generator(&self) -> PairGenerator {
        PairGenerator::new(self.joint_incident(), self.interval_map())
    }

    fn joint_incident(&self) -> Vec<JointIncident> {
        let mut incidents: Vec<_> = self.joints
            .iter()
            .enumerate()
            .map(|(index, joint)| JointIncident::new(index, joint.location))
            .collect();
        for interval @ Interval { alpha_index, omega_index, .. } in self.interval_values() {
            incidents[*alpha_index].add_interval(interval, &self.materials);
            incidents[*omega_index].add_interval(interval, &self.materials);
        }
        incidents
    }

    fn interval_map(&self) -> HashMap<(usize, usize), Interval> {
        self.interval_values()
            .map(|interval| (interval.key(), *interval))
            .collect()
    }
}

#[derive(Debug, Clone)]
struct JointIncident {
    index: usize,
    location: Point3<f32>,
    push: Option<Interval>,
    pulls: Vec<Interval>,
    pull_adjacent_joints: HashSet<usize>,
    adjacent_joints: HashSet<usize>,
}

impl JointIncident {
    fn new(index: usize, location: Point3<f32>) -> Self {
        Self {
            index,
            location,
            push: None,
            pulls: vec![],
            pull_adjacent_joints: HashSet::new(),
            adjacent_joints: HashSet::new(),
        }
    }

    fn add_interval(&mut self, interval: &Interval, materials: &[Material]) {
        match materials[interval.material].role {
            Role::Push => self.push = Some(*interval),
            Role::Pull => {
                self.pulls.push(*interval);
                self.pull_adjacent_joints.insert(interval.other_joint(self.index));
            }
        }
        self.adjacent_joints.insert(interval.other_joint(self.index));
    }

    fn extended_paths(&self, path: &Path) -> Vec<Path> {
        self.pulls
            .iter()
            .flat_map(|pull| path.add(*pull))
            .collect()
    }

    fn across_push(&self) -> Option<usize> {
        self.push.map(|push| push.other_joint(self.index))
    }
}

#[derive(Debug, Clone)]
struct Path {
    joint_indices: Vec<usize>,
    intervals: Vec<Interval>,
}

impl Path {
    fn new(joint_index: usize, interval: Interval) -> Self {
        Self { joint_indices: vec![joint_index], intervals: vec![interval] }
    }

    fn add(&self, interval: Interval) -> Option<Path> {
        if self.is_cycle() {
            return None;
        }
        let last_joint = self.last_interval().joint_with(&interval)?;
        let mut path = self.clone();
        path.joint_indices.push(last_joint);
        path.intervals.push(interval);
        Some(path)
    }

    fn is_cycle(&self) -> bool {
        self.first_joint() == self.last_joint()
    }

    fn first(&self) -> &Interval {
        self.intervals.first().unwrap()
    }

    fn last_interval(&self) -> &Interval {
        self.intervals.last().unwrap()
    }

    fn first_joint(&self) -> usize {
        self.joint_indices[0]
    }

    fn last_joint(&self) -> usize {
        self.last_interval().other_joint(self.joint_indices[self.joint_indices.len() - 1])
    }

    fn hexagon_key(&self) -> Option<[usize; 6]> {
        if self.joint_indices.len() != 6 || !self.is_cycle() {
            return None;
        }
        let mut key: [usize; 6] = self.joint_indices.clone().try_into().unwrap();
        key.sort();
        Some(key)
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

    fn proximity_measures(mut self) -> impl Iterator<Item=Pair> {
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
        let new_pairs = self.joints
            .iter()
            .filter_map(|other_joint| {
                if joint_index == other_joint.index {
                    return None;
                }
                if self.joints[joint_index].adjacent_joints.contains(&other_joint.index) {
                    return None;
                }
                let length = self.joints[joint_index].location.distance(other_joint.location);
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

    fn bow_tie_pulls(mut self, joints: &[Joint], materials: &[Material]) -> impl Iterator<Item=Pair> {
        for interval in self.intervals.values() {
            if materials[interval.material].role != Role::Push {
                continue;
            }
            let mut meeting_pairs = vec![];
            for alpha_path in self.paths_for(interval.alpha_index, 2) {
                for omega_path in self.paths_for(interval.omega_index, 2) {
                    if alpha_path.last_interval().key() == omega_path.last_interval().key() { // second interval is the bridge
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
                            if self.interval_exists(self.joints[a].across_push()?, self.joints[b].across_push()?) {
                                return None;
                            }
                            Some((a, b))
                        })
                        .collect();
                    if let &[(alpha_index, omega_index)] = cross_twist_diagonals.as_slice() {
                        let distance = joints[alpha_index].location.distance(joints[omega_index].location);
                        let pair = Pair { alpha_index, omega_index, length: distance * BOW_TIE_SHORTEN};
                        self.pairs.insert(pair.key(), pair);
                    } else {
                        let candidate_completions = [
                            (alpha1, alpha2), (alpha2, alpha1),
                            (omega1, omega2), (omega2, omega1),
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
                            let distance = joints[alpha_index].location.distance(joints[omega_index].location);
                            let pair = Pair { alpha_index, omega_index, length: distance * BOW_TIE_SHORTEN };
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
                        let pair = Pair { alpha_index, omega_index, length: interval.ideal() / 4.0 };
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
        let paths: Vec<_> = self.joints[joint_index].pulls
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