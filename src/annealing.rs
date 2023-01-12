use std::collections::HashMap;

use cgmath::{MetricSpace, Point3};

use crate::fabric::Fabric;
use crate::interval::{Interval, Role};
use crate::interval::Role::{Measure};

impl Fabric {
    pub fn install_measures(&mut self) {
        let mut measures = PairGenerator::new(self.joint_incident());
        measures.generate_pairs();
        for MeasurePair { alpha_index, omega_index, length } in measures.measure_pairs.values() {
            self.create_interval(*alpha_index, *omega_index, Measure, *length);
        }
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
struct JointIncident {
    index: usize,
    location: Point3<f32>,
    push: Option<Interval>,
    pulls: Vec<Interval>,
}

impl JointIncident {
    fn new(index: usize, location: Point3<f32>) -> Self {
        Self { index, location, push: None, pulls: vec![] }
    }

    fn add(&mut self, interval: &Interval) {
        match interval.role {
            Role::Push => self.push = Some(interval.clone()),
            Role::Pull => self.pulls.push(interval.clone()),
            Measure => panic!("Should be no measures yet"),
        }
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
    joint_incident: Vec<JointIncident>,
    measure_pairs: HashMap<(usize, usize), MeasurePair>,
}

impl PairGenerator {
    fn new(joint_incident: Vec<JointIncident>) -> Self {
        Self {
            joint_incident,
            measure_pairs: HashMap::new(),
        }
    }

    fn generate_pairs(&mut self) {
        for joint_incident in self.joint_incident.clone() {
            self.add_pairs_for(joint_incident)
        }
    }

    fn add_pairs_for(&mut self, JointIncident{index, location, push, pulls}: JointIncident) {
        let Some(push) = push else {
            return;
        };
        let length_limit = push.ideal_length();
        for other_joint in self.joint_incident.clone() {
            if index == other_joint.index {
                continue;
            }
            if push.other_joint(index) == other_joint.index {
                continue;
            }
            if pulls.iter().any(|pull| pull.other_joint(index) == other_joint.index) {
                continue;
            }
            let length = location.distance(other_joint.location);
            if length > length_limit {
                continue;
            }
            self.add_pair(MeasurePair {
                alpha_index: index,
                omega_index: other_joint.index,
                length,
            });
        }
    }

    fn add_pair(&mut self, pair: MeasurePair) {
        self.measure_pairs.insert(pair.key(), pair);
    }
}