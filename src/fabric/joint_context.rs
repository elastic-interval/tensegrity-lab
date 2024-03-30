use std::cmp::Ordering;
use itertools::Itertools;
use crate::fabric::{Fabric, UniqueId};
use crate::fabric::interval::{Interval, Role};

impl Fabric {
    pub fn joint_contexts(&self) -> Vec<JointContext> {
        let mut joint_context: Vec<JointContext> = self
            .joints
            .iter()
            .enumerate()
            .map(|(index, _)| JointContext::new(index)).collect();
        for (id, Interval { alpha_index, omega_index, .. }) in self.intervals.iter() {
            joint_context.get_mut(*alpha_index).unwrap().add(id);
            joint_context.get_mut(*omega_index).unwrap().add(id);
        }
        joint_context
    }

    pub fn angles(&self) -> Vec<(usize, usize, usize)> {
        self
            .joint_contexts()
            .iter()
            .flat_map(|context| context.angles(self))
            .collect()
    }

    pub fn connections(&self) -> Vec<Vec<JointConnection>> {
        self
            .joint_contexts()
            .iter()
            .map(|joint_context| joint_context.connections(self))
            .collect()
    }

    pub fn instructions(&self, scale: f32) -> Vec<String> {
        self
            .connections()
            .iter()
            .flatten()
            .sorted()
            .map(|JointConnection{ alpha_index, length, role, omega_index }| {
                format!("{:?} {} {:.1}mm {}", *role, alpha_index, length * scale, omega_index)
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct JointContext {
    pub index: usize,
    pub intervals: Vec<UniqueId>,
}

#[derive(Clone, Debug)]
pub struct JointConnection {
    alpha_index: usize,
    length: f32,
    role: Role,
    omega_index: usize,
}

impl Eq for JointConnection {}

impl PartialEq<Self> for JointConnection {
    fn eq(&self, other: &Self) -> bool {
        self.alpha_index == other.alpha_index &&
            self.omega_index == other.omega_index
    }
}

impl PartialOrd<Self> for JointConnection {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for JointConnection {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.eq(other) {
            Ordering::Equal
        } else if self.alpha_index > other.alpha_index {
            Ordering::Greater
        } else if self.alpha_index < other.alpha_index {
            Ordering::Less
        } else {
            match self.role {
                Role::Push if other.role == Role::Pull => Ordering::Less,
                Role::Pull if other.role == Role::Push => Ordering::Greater,
                _ => Ordering::Equal
            }
        }
    }
}

impl JointContext {
    pub fn new(index: usize) -> JointContext {
        JointContext { index, intervals: Default::default() }
    }

    pub fn add(&mut self, interval_id: &UniqueId) {
        self.intervals.push(*interval_id);
    }

    pub fn joints(&self, fabric: &Fabric) -> Vec<usize> {
        self
            .intervals
            .iter()
            .map(|id| *fabric.interval(*id))
            .map(|interval| interval.other_joint(self.index))
            .collect()
    }

    pub fn connections(&self, fabric: &Fabric) -> Vec<JointConnection> {
        self
            .intervals
            .iter()
            .map(|id| *fabric.interval(*id))
            .map(|interval| JointConnection {
                alpha_index: self.index,
                length: interval.ideal(),
                role: fabric.materials[interval.material].role,
                omega_index: interval.other_joint(self.index),
            })
            .collect()
    }

    pub fn angles(&self, fabric: &Fabric) -> Vec<(usize, usize, usize)> {
        self
            .joints(fabric)
            .iter()
            .tuple_combinations()
            .map(|(a, b)| (*a, self.index, *b))
            .collect()
    }
}