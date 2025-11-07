use std::collections::HashSet;

use cgmath::Point3;

use crate::fabric::interval::{Interval, Role};
use crate::fabric::{Fabric, UniqueId};

impl Fabric {
    pub fn joint_incidents(&self) -> Vec<JointIncident> {
        let mut incidents: Vec<_> = self
            .joints
            .iter()
            .enumerate()
            .map(|(index, joint)| JointIncident::new(index, joint.location))
            .collect();
        for (index, interval_opt) in self.intervals.iter().enumerate() {
            if let Some(interval) = interval_opt {
                let id = UniqueId(index);
                incidents[interval.alpha_index].add_interval(id, interval);
                incidents[interval.omega_index].add_interval(id, interval);
            }
        }
        incidents
    }
}

#[derive(Debug, Clone)]
pub struct JointIncident {
    pub index: usize,
    pub location: Point3<f32>,
    intervals: Vec<(UniqueId, Interval)>,
}

impl JointIncident {
    pub fn new(index: usize, location: Point3<f32>) -> Self {
        Self {
            index,
            location,
            intervals: vec![],
        }
    }

    pub fn add_interval(&mut self, id: UniqueId, interval: &Interval) {
        self.intervals.push((id, interval.clone()));
    }

    pub fn intervals(&self) -> &[(UniqueId, Interval)] {
        &self.intervals
    }

    pub fn push(&self) -> Option<(UniqueId, Interval)> {
        self.intervals
            .iter()
            .find(|(_, interval)| interval.has_role(Role::Pushing))
            .map(|(id, interval)| (*id, interval.clone()))
    }

    pub fn pulls(&self) -> Vec<(UniqueId, Interval)> {
        self.intervals
            .iter()
            .filter(|(_, interval)| interval.role.is_pull_like())
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

    pub fn adjacent_joints(&self) -> HashSet<usize> {
        self.intervals
            .iter()
            .filter(|(_, interval)| interval.role.is_pull_like())
            .map(|(_, interval)| interval.other_joint(self.index))
            .collect()
    }

    pub fn interval_to(&self, joint_index: usize) -> Option<(UniqueId, Interval)> {
        self.intervals
            .iter()
            .find(|(_, interval)| interval.other_joint(self.index) == joint_index)
            .map(|(id, interval)| (*id, interval.clone()))
    }

    pub(crate) fn extended_paths(&self, path: &Path) -> Vec<Path> {
        self.pulls()
            .iter()
            .flat_map(|(_, pull)| path.add(pull.clone()))
            .collect()
    }

    pub fn across_push(&self) -> Option<usize> {
        self.push().map(|(_, push)| push.other_joint(self.index))
    }
}

#[derive(Debug, Clone)]
pub struct Path {
    pub(crate) joint_indices: Vec<usize>,
    intervals: Vec<Interval>,
}

impl Path {
    pub(crate) fn new(joint_index: usize, interval: Interval) -> Self {
        Self {
            joint_indices: vec![joint_index],
            intervals: vec![interval],
        }
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

    fn _first(&self) -> &Interval {
        self.intervals.first().unwrap()
    }

    pub(crate) fn last_interval(&self) -> &Interval {
        self.intervals.last().unwrap()
    }

    fn first_joint(&self) -> usize {
        self.joint_indices[0]
    }

    pub(crate) fn last_joint(&self) -> usize {
        self.last_interval()
            .other_joint(self.joint_indices[self.joint_indices.len() - 1])
    }

    fn _hexagon_key(&self) -> Option<[usize; 6]> {
        if self.joint_indices.len() != 6 || !self.is_cycle() {
            return None;
        }
        let mut key: [usize; 6] = self.joint_indices.clone().try_into().unwrap();
        key.sort();
        Some(key)
    }
}
