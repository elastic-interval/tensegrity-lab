use std::collections::{HashMap, HashSet};

use cgmath::Point3;

use crate::fabric::interval::{Interval, Role};
use crate::fabric::{Fabric, IntervalKey, JointKey};

impl Fabric {
    pub fn joint_incidents(&self) -> HashMap<JointKey, JointIncident> {
        let mut incidents: HashMap<JointKey, JointIncident> = self
            .joints
            .iter()
            .map(|(key, joint)| (key, JointIncident::new(key, joint.location)))
            .collect();
        for (key, interval) in self.intervals.iter() {
            if let Some(incident) = incidents.get_mut(&interval.alpha_key) {
                incident.add_interval(key, interval);
            }
            if let Some(incident) = incidents.get_mut(&interval.omega_key) {
                incident.add_interval(key, interval);
            }
        }
        incidents
    }
}

#[derive(Debug, Clone)]
pub struct JointIncident {
    pub key: JointKey,
    pub location: Point3<f32>,
    intervals: Vec<(IntervalKey, Interval)>,
}

impl JointIncident {
    pub fn new(key: JointKey, location: Point3<f32>) -> Self {
        Self {
            key,
            location,
            intervals: vec![],
        }
    }

    pub fn add_interval(&mut self, id: IntervalKey, interval: &Interval) {
        self.intervals.push((id, interval.clone()));
    }

    pub fn intervals(&self) -> &[(IntervalKey, Interval)] {
        &self.intervals
    }

    pub fn push(&self) -> Option<(IntervalKey, Interval)> {
        self.intervals
            .iter()
            .find(|(_, interval)| interval.has_role(Role::Pushing))
            .map(|(id, interval)| (*id, interval.clone()))
    }

    pub fn pulls(&self) -> Vec<(IntervalKey, Interval)> {
        self.intervals
            .iter()
            .filter(|(_, interval)| {
                interval.role.is_pull_like() && !interval.has_role(Role::PrismPull)
            })
            .map(|(id, interval)| (*id, interval.clone()))
            .collect()
    }

    pub fn springs(&self) -> Vec<Interval> {
        self.intervals
            .iter()
            .filter(|(_, interval)| interval.has_role(Role::Springy))
            .map(|(_, interval)| interval.clone())
            .collect()
    }

    pub fn adjacent_joints(&self) -> HashSet<JointKey> {
        self.intervals
            .iter()
            .filter(|(_, interval)| interval.role.is_pull_like())
            .map(|(_, interval)| interval.other_joint(self.key))
            .collect()
    }

    pub fn interval_to(&self, joint_key: JointKey) -> Option<(IntervalKey, Interval)> {
        self.intervals
            .iter()
            .find(|(_, interval)| interval.other_joint(self.key) == joint_key)
            .map(|(id, interval)| (*id, interval.clone()))
    }

    pub(crate) fn extended_paths(&self, path: &Path) -> Vec<Path> {
        self.pulls()
            .iter()
            .flat_map(|(_, pull)| path.add(pull.clone()))
            .collect()
    }

    pub fn across_push(&self) -> Option<JointKey> {
        self.push().map(|(_, push)| push.other_joint(self.key))
    }
}

#[derive(Debug, Clone)]
pub struct Path {
    pub(crate) joint_keys: Vec<JointKey>,
    intervals: Vec<Interval>,
}

impl Path {
    pub(crate) fn new(joint_key: JointKey, interval: Interval) -> Self {
        Self {
            joint_keys: vec![joint_key],
            intervals: vec![interval],
        }
    }

    fn add(&self, interval: Interval) -> Option<Path> {
        if self.is_cycle() {
            return None;
        }
        let last_joint = self.last_interval().joint_with(&interval)?;
        let mut path = self.clone();
        path.joint_keys.push(last_joint);
        path.intervals.push(interval);
        Some(path)
    }

    fn is_cycle(&self) -> bool {
        self.first_joint() == self.last_joint()
    }

    fn _first(&self) -> &Interval {
        self.intervals.first().unwrap()
    }

    pub(crate) fn last_interval(&self) -> &Interval {
        self.intervals.last().unwrap()
    }

    fn first_joint(&self) -> JointKey {
        self.joint_keys[0]
    }

    pub(crate) fn last_joint(&self) -> JointKey {
        self.last_interval()
            .other_joint(self.joint_keys[self.joint_keys.len() - 1])
    }

    fn _hexagon_key(&self) -> Option<[JointKey; 6]> {
        if self.joint_keys.len() != 6 || !self.is_cycle() {
            return None;
        }
        let mut key: [JointKey; 6] = self.joint_keys.clone().try_into().unwrap();
        key.sort();
        Some(key)
    }
}
