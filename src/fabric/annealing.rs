use std::collections::{HashMap, HashSet};

use cgmath::{InnerSpace, MetricSpace, Point3, Vector3};
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
            incidents[*alpha_index].add(interval);
            incidents[*omega_index].add(interval);
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

    fn add(&mut self, interval: &Interval) {
        match interval.role {
            Role::Push => self.push = Some(interval.clone()),
            Role::Pull => self.pulls.push(interval.clone()),
            Role::Measure => panic!("Should be no measures yet"),
        }
        self.adjacent_joints.insert(interval.other_joint(self.index));
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
    pairs: HashMap<(usize, usize), MeasurePair>,
}

impl PairGenerator {
    fn new(joints: Vec<JointIncident>, intervals: HashSet<(usize, usize)>) -> Self {
        Self {
            joints,
            intervals,
            pairs: HashMap::new(),
        }
    }

    fn generate_pairs(mut self, pair_strategy: PairStrategy) -> impl Iterator<Item=MeasurePair> {
        for joint in self.joints.clone() {
            match pair_strategy {
                PairStrategy::PushProximity => self.push_proximity(joint),
                PairStrategy::BowTie => self.bow_tie(joint),
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

    fn bow_tie(&mut self, joint: JointIncident) {
        if joint.push.is_none() {
            return;
        };
        for (remote, _) in self.across_pulls(joint.index) {
            for (across_remote, remote_ray) in self.across_pulls(remote) {
                if across_remote == joint.index { // not back to us
                    continue;
                }
                for (local, local_ray) in self.across_pulls(joint.index) {
                    if local == remote { // different from the main loop
                        continue;
                    }
                    if self.interval_exists(local, across_remote) {
                        continue;
                    }
                    let dot = local_ray.dot(remote_ray);
                    if dot < 0.85 { // not similar enough direction
                        continue;
                    } else {
                        dbg!(dot);
                    }
                    let length = self.joints[remote].location.distance(self.joints[across_remote].location);
                    let pair = MeasurePair { alpha_index: remote, omega_index: local, length };
                    self.pairs.insert(pair.key(), pair);
                }
            }
        }
    }

    fn interval_exists(&self, a: usize, b: usize) -> bool {
        if a < b {
            self.intervals.contains(&(a, b))
        } else {
            self.intervals.contains(&(b, a))
        }
    }

    fn across_pulls(&self, index: usize) -> Vec<(usize, Vector3<f32>)> {
        self.joints[index].pulls
            .iter()
            .map(|pull| {
                let other_index = pull.other_joint(index);
                let to_other = (self.joints[other_index].location - self.joints[index].location).normalize();
                (other_index, to_other)
            })
            .collect()
    }
}