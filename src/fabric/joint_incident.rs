use std::collections::HashSet;
use cgmath::Point3;
use crate::fabric::Fabric;
use crate::fabric::interval::{Interval, Material, Role};

impl Fabric {
    pub fn joint_incidents(&self) -> Vec<JointIncident> {
        let mut incidents: Vec<_> = self
            .joints
            .iter()
            .enumerate()
            .map(|(index, joint)| JointIncident::new(index, joint.location))
            .collect();
        for interval @ Interval {
            alpha_index,
            omega_index,
            ..
        } in self.interval_values()
        {
            incidents[*alpha_index].add_interval(interval, &self.materials);
            incidents[*omega_index].add_interval(interval, &self.materials);
        }
        incidents
    }
}

#[derive(Debug, Clone)]
pub struct JointIncident {
    pub index: usize,
    pub location: Point3<f32>,
    pub push: Option<Interval>,
    pub pulls: Vec<Interval>,
    pub springs: Vec<Interval>,
    pub pull_adjacent_joints: HashSet<usize>,
    pub adjacent_joints: HashSet<usize>,
}

impl JointIncident {
    pub(crate) fn new(index: usize, location: Point3<f32>) -> Self {
        Self {
            index,
            location,
            push: None,
            pulls: vec![],
            springs: vec![],
            pull_adjacent_joints: HashSet::new(),
            adjacent_joints: HashSet::new(),
        }
    }

    pub fn add_interval(&mut self, interval: &Interval, materials: &[Material]) {
        match materials[interval.material].role {
            Role::Push => self.push = Some(*interval),
            Role::Pull => {
                self.pulls.push(*interval);
                self.pull_adjacent_joints
                    .insert(interval.other_joint(self.index));
            }
            Role::Spring => {
                self.springs.push(*interval);
            }
        }
        self.adjacent_joints
            .insert(interval.other_joint(self.index));
    }

    pub(crate) fn extended_paths(&self, path: &Path) -> Vec<Path> {
        self.pulls.iter().flat_map(|pull| path.add(*pull)).collect()
    }

    pub(crate) fn across_push(&self) -> Option<usize> {
        self.push.map(|push| push.other_joint(self.index))
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