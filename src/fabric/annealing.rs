use std::collections::{HashMap, HashSet};

use cgmath::{MetricSpace, Point3};

use crate::fabric::Fabric;
use crate::fabric::interval::{Interval, Role};
use crate::fabric::interval::Role::Measure;

impl Fabric {
    pub fn install_measures(&mut self) {
        let measures = PairGenerator::new(self.joint_incident());
        for MeasurePair { alpha_index, omega_index, length } in measures.generate_pairs() {
            self.create_interval(alpha_index, omega_index, Measure, length);
        }
    }

    pub fn measure_limits(&self) -> Option<MeasureLimits> {
        let mut limits: MeasureLimits = MeasureLimits { low: f32::MAX, high: f32::MIN };
        let mut measures_present = false;
        for Interval { strain, .. } in self.interval_measures() {
            measures_present = true;
            if *strain > limits.high {
                limits.high = *strain;
            }
            if *strain < limits.low {
                limits.low = *strain;
            }
        }
        measures_present.then_some(limits)
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
}

#[derive(Debug, Clone)]
pub struct MeasureLimits {
    low: f32,
    high: f32,
}

impl MeasureLimits {
    pub fn interpolate(&self, nuance: f32) -> f32 {
        self.low * (1.0 - nuance) + self.high * nuance
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
            Measure => panic!("Should be no measures yet"),
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
    pairs: HashMap<(usize, usize), MeasurePair>,
}

impl PairGenerator {
    fn new(joints: Vec<JointIncident>) -> Self {
        Self {
            joints,
            pairs: HashMap::new(),
        }
    }

    fn generate_pairs(mut self) -> impl Iterator<Item=MeasurePair> {
        for joint in self.joints.clone() {
            self.add_pairs_for(joint)
        }
        self.pairs.into_values()
    }

    fn add_pairs_for(&mut self, joint: JointIncident) {
        let Some(push) = &joint.push else {
            return;
        };
        let length_limit = push.ideal_length();
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
}