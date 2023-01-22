use std::collections::{HashMap, HashSet};

use cgmath::{MetricSpace, Point3};
use crate::fabric::{Fabric, Link};
use crate::fabric::interval::{Interval, Role};

pub enum PairStrategy {
    PushProximity,
    BowTie,
}

impl Fabric {
    pub fn install_measures(&mut self, pair_strategy: PairStrategy) {
        let measures = PairGenerator::new(self.joint_incident(), self.interval_keys());
        for MeasurePair { alpha_index, omega_index, length } in measures.generate_pairs(pair_strategy) {
            self.create_interval(alpha_index, omega_index, Link::Measure { length });
        }
    }

    pub fn max_measure_strain(&self) -> f32 {
        self.interval_measures()
            .map(|Interval { strain, .. }| {
                if strain > &1.0 {
                    panic!("Strain {strain}")
                }
                strain
            })
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

    fn interval_keys(&self) -> HashSet<(usize, usize)> {
        let mut set = HashSet::new();
        for interval in self.interval_values() {
            set.insert(interval.key());
        }
        set
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

#[derive(Debug, Clone)]
struct Path {
    joint_indices: Vec<usize>,
    cycle: bool,
}

impl Path {
    fn add(&self, joint_index: usize) -> Option<Path> {
        if self.cycle {
            return None;
        }
        if self.joint_indices.contains(&joint_index) {
            if self.joint_indices.first().unwrap() == &joint_index {
                let mut fresh = self.clone();
                fresh.cycle = true;
                Some(fresh)
            } else {
                None
            }
        } else {
            let mut fresh = self.clone();
            fresh.joint_indices.push(joint_index);
            Some(fresh)
        }
    }

    fn to_hexagon(&self) -> Option<[usize; 6]> {
        if self.joint_indices.len() == 6 && self.cycle {
            let hexagon: [usize; 6] = self.joint_indices.clone().try_into().unwrap();
            Some(hexagon)
        } else {
            None
        }
    }
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
            Role::Push => self.push = Some(interval.clone()),
            Role::Pull => self.pulls.push(interval.clone()),
            Role::Measure => panic!("Should be no measures yet"),
        }
        self.adjacent_joints.insert(interval.other_joint(self.index));
    }

    fn pull_steps(&self, path: &Path) -> Vec<Path> {
        self.pulls
            .iter()
            .flat_map(|pull| path.add(pull.other_joint(self.index)))
            .collect()
    }
}

#[derive(Debug)]
struct MeasurePair {
    alpha_index: usize,
    omega_index: usize,
    length: f32,
}

impl MeasurePair {
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
    intervals: HashSet<(usize, usize)>,
    hexagons: HashMap<[usize; 6], [usize; 6]>,
    pairs: HashMap<(usize, usize), MeasurePair>,
}

impl PairGenerator {
    fn new(joints: Vec<JointIncident>, intervals: HashSet<(usize, usize)>) -> Self {
        Self {
            joints,
            intervals,
            hexagons: HashMap::new(),
            pairs: HashMap::new(),
        }
    }

    fn generate_pairs(mut self, pair_strategy: PairStrategy) -> impl Iterator<Item=MeasurePair> {
        match pair_strategy {
            PairStrategy::PushProximity => {
                for joint in self.joints.clone() {
                    self.push_proximity(joint);
                }
            }
            PairStrategy::BowTie => {
                for joint in self.joints.clone() {
                    self.add_hexagons_for(joint);
                }
                for hexagon in self.hexagons.values() {
                    let mut diagonals: Vec<(usize, usize, f32)> = (0..3)
                        .map(|index| {
                            let other_index = (index + 3) % hexagon.len();
                            let (alpha_index, omega_index) = (hexagon[index], hexagon[other_index]);
                            let distance = self.joints[alpha_index].location.distance(self.joints[omega_index].location);
                            (alpha_index, omega_index, distance)
                        })
                        .collect();
                    diagonals.sort_by(|(_, _, a), (_, _, b)| a.partial_cmp(b).unwrap());
                    let (short, long1, long2) = (diagonals[0].2, diagonals[1].2, diagonals[2].2);
                    if (long1 + long2) / 2.0 > short * 1.3 {
                        let (alpha_index, omega_index, length) = diagonals[0];
                        let pair = MeasurePair { alpha_index, omega_index, length };
                        self.pairs.insert(pair.key(), pair);
                    }
                }
            }
        }
        self.pairs.into_values()
    }

    fn push_proximity(&mut self, joint: JointIncident) {
        let Some(push) = &joint.push else {
            return;
        };
        let length_limit = push.ideal();
        let two_steps: HashSet<_> = joint.adjacent_joints
            .iter()
            .flat_map(|&adjacent| self.joints[adjacent].adjacent_joints.iter())
            .collect();
        let new_pairs = self.joints
            .iter()
            .filter_map(|other_joint| {
                if joint.index == other_joint.index {
                    return None;
                }
                if joint.adjacent_joints.contains(&other_joint.index) {
                    return None;
                }
                if two_steps.contains(&other_joint.index) {
                    return None;
                }
                let length = joint.location.distance(other_joint.location);
                if length > length_limit {
                    return None;
                }
                Some(MeasurePair {
                    alpha_index: joint.index,
                    omega_index: other_joint.index,
                    length,
                })
            })
            .map(|pair| (pair.key(), pair));
        self.pairs.extend(new_pairs);
    }

    fn add_hexagons_for(&mut self, joint: JointIncident) {
        if joint.push.is_none() {
            return;
        }
        for hexagon in self.hexagons(joint.index) {
            let mut key = hexagon;
            key.sort();
            self.hexagons.insert(key, hexagon);
        }
    }

    fn interval_exists(&self, a: usize, b: usize) -> bool {
        if a < b {
            self.intervals.contains(&(a, b))
        } else {
            self.intervals.contains(&(b, a))
        }
    }

    fn hexagons(&self, joint_index: usize) -> Vec<[usize; 6]> {
        let collection = vec![Path { joint_indices: vec![joint_index], cycle: false }];
        self.paths_via_pulls(&collection, 1, 7)
            .iter()
            .flat_map(|path| path.to_hexagon())
            .collect()
    }

    fn paths_via_pulls(&self, collection: &[Path], size: usize, max_size: usize) -> Vec<Path> {
        if size == max_size {
            collection.to_vec()
        } else {
            let bigger: Vec<Path> = collection
                .iter()
                .flat_map(|path| {
                    let last = path.joint_indices.last().unwrap();
                    self.joints[*last].pull_steps(path)
                })
                .collect();
            self.paths_via_pulls(&bigger, size + 1, max_size)
        }
    }
}