use std::collections::{HashMap, HashSet};

use cgmath::{MetricSpace, Point3};
use crate::fabric::{Fabric, Link};
use crate::fabric::interval::{Interval, Role};

impl Fabric {
    pub fn install_bow_ties(&mut self) {
        let pairs = PairGenerator::new(self.joint_incident(), self.interval_map());
        for Pair { alpha_index, omega_index, link } in pairs.bow_tie_pulls() {
            self.create_interval(alpha_index, omega_index, link);
        }
    }

    pub fn install_measures(&mut self) {
        let pairs = PairGenerator::new(self.joint_incident(), self.interval_map());
        for Pair { alpha_index, omega_index, link } in pairs.proximity_measures() {
            self.create_interval(alpha_index, omega_index, link);
        }
    }

    pub fn max_measure_strain(&self) -> f32 {
        self.interval_measures()
            .map(|Interval { strain, .. }| strain)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .cloned()
            .unwrap_or(0.0)
    }

    pub fn measures_to_pulls(&mut self, strain_threshold: f32) -> Vec<(usize, usize, f32)> {
        self.interval_values()
            .filter_map(|interval|
                (interval.role == Role::Measure && interval.strain > strain_threshold)
                    .then_some((
                        interval.alpha_index,
                        interval.omega_index,
                        interval.ideal() - interval.strain,
                    )))
            .collect()
    }

    fn joint_incident(&self) -> Vec<JointIncident> {
        let mut incidents: Vec<JointIncident> = self.joints
            .iter()
            .enumerate()
            .map(|(index, joint)| JointIncident::new(index, joint.location)).collect();
        for interval @ Interval { alpha_index, omega_index, .. } in self.interval_values() {
            incidents[*alpha_index].add_interval(interval);
            incidents[*omega_index].add_interval(interval);
        }
        incidents
    }

    fn interval_map(&self) -> HashMap<(usize, usize), Interval> {
        let mut hashmap = HashMap::new();
        for interval in self.interval_values() {
            hashmap.insert(interval.key(), *interval);
        }
        hashmap
    }
}

#[derive(Debug, Clone)]
struct JointIncident {
    index: usize,
    location: Point3<f32>,
    push: Option<Interval>,
    pulls: Vec<Interval>,
    adjacent_joints: HashSet<usize>,
}

impl JointIncident {
    fn new(index: usize, location: Point3<f32>) -> Self {
        Self {
            index,
            location,
            push: None,
            pulls: vec![],
            adjacent_joints: HashSet::new(),
        }
    }

    fn add_interval(&mut self, interval: &Interval) {
        match interval.role {
            Role::Push => self.push = Some(*interval),
            Role::Pull => self.pulls.push(*interval),
            Role::Measure => panic!("Should be no measures yet"),
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
        let last_joint = self.last().joint_with(&interval)?;
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

    fn last(&self) -> &Interval {
        self.intervals.last().unwrap()
    }

    fn first_joint(&self) -> usize {
        self.joint_indices[0]
    }

    fn last_joint(&self) -> usize {
        self.last().other_joint(self.joint_indices[self.joint_indices.len() - 1])
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
    link: Link,
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
        let two_steps: HashSet<_> = self.joints[joint_index].adjacent_joints
            .iter()
            .flat_map(|&adjacent| self.joints[adjacent].adjacent_joints.iter())
            .collect();
        let new_pairs = self.joints
            .iter()
            .filter_map(|other_joint| {
                if joint_index == other_joint.index {
                    return None;
                }
                if self.joints[joint_index].adjacent_joints.contains(&other_joint.index) {
                    return None;
                }
                if two_steps.contains(&other_joint.index) {
                    return None;
                }
                let length = self.joints[joint_index].location.distance(other_joint.location);
                if length > length_limit {
                    return None;
                }
                Some(Pair {
                    alpha_index: joint_index,
                    omega_index: other_joint.index,
                    link: Link::Measure { length },
                })
            })
            .map(|pair| (pair.key(), pair));
        self.pairs.extend(new_pairs);
    }

    fn bow_tie_pulls(mut self) -> impl Iterator<Item=Pair> {
        for interval in self.intervals.values() {
            if interval.role != Role::Push {
                continue;
            }
            let mut meeting_pairs: Vec<(Path, Path)> = vec![];
            for alpha_path in self.paths_for(interval.alpha_index, 2) {
                for omega_path in self.paths_for(interval.omega_index, 2) {
                    if alpha_path.last() == omega_path.last() { // second one is the bridge
                        meeting_pairs.push((alpha_path.clone(), omega_path.clone()))
                    }
                }
            }
            let [(alpha1, omega1), (alpha2, omega2)] = meeting_pairs.as_slice() else {
                continue;
            };
            let diagonals = [
                (alpha1.last_joint(), omega2.last_joint()),
                (alpha2.last_joint(), omega1.last_joint())
            ];
            let candidates: Vec<(usize, usize)> = diagonals
                .iter()
                .filter_map(|&(a, b)| {
                    match (self.joints[a].across_push(), self.joints[b].across_push()) {
                        (Some(joint_a), Some(joint_b)) => {
                            (!self.interval_exists(joint_a, joint_b)).then_some((a, b))
                        }
                        _ => None
                    }
                })
                .collect();
            if let &[(alpha_index, omega_index)] = candidates.as_slice() {
                let link = Link::Pull {ideal: interval.ideal() / 3.0};
                let pair = Pair { alpha_index, omega_index, link};
                self.pairs.insert(pair.key(), pair);
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

    fn get_interval(&self, a: usize, b: usize) -> Option<&Interval> {
        if a < b {
            self.intervals.get(&(a, b))
        } else {
            self.intervals.get(&(b, a))
        }
    }

    fn paths_for(&self, joint_index: usize, max_size: usize) -> Vec<Path> {
        let paths: Vec<Path> = self.joints[joint_index].pulls
            .iter()
            .map(|pull| Path::new(joint_index, *pull))
            .collect();
        self.paths_via_pulls(&paths, 1, max_size)
    }

    fn paths_via_pulls(&self, paths: &[Path], size: usize, max_size: usize) -> Vec<Path> {
        if size == max_size {
            paths.to_vec()
        } else {
            let bigger: Vec<Path> = paths
                .iter()
                .flat_map(|path| self.joints[path.last_joint()].extended_paths(path))
                .collect();
            self.paths_via_pulls(&bigger, size + 1, max_size)
        }
    }
}