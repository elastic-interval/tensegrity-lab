use crate::fabric::interval::{End, IntervalSnapshot};
use crate::fabric::material::Material;
use crate::fabric::{Fabric, UniqueId};
use cgmath::{Point3, Vector3};

pub struct EvolvingPush {
    pub interval_id: UniqueId,
    alpha_pulls: Vec<UniqueId>,
    omega_pulls: Vec<UniqueId>,
}

impl EvolvingPush {}

impl EvolvingPush {
    pub fn first_push(fabric: &mut Fabric) -> Self {
        let alpha = fabric.create_joint(Point3::new(0.5, 0.0, 0.0));
        let omega = fabric.create_joint(Point3::new(-0.5, 0.0, 0.0));
        let interval_id = fabric.create_interval(alpha, omega, 1.0, Material::PushMaterial, 0);
        Self::new(interval_id)
    }

    pub fn end_push(&mut self, fabric: &mut Fabric, snapshot: IntervalSnapshot, end: End, project: Vector3<f32>) -> Self {
        let IntervalSnapshot {
            interval,
            alpha,
            omega,
        } = snapshot;
        let (here_id, here, pulls) = match end {
            End::Alpha => (interval.alpha_index, alpha.location, &mut self.alpha_pulls),
            End::Omega => (interval.omega_index, omega.location, &mut self.omega_pulls),
        };
        let alpha = fabric.create_joint(here - project / 2.0);
        let omega = fabric.create_joint(here + project / 2.0);
        let interval_id = fabric.create_interval(alpha, omega, 1.0, Material::PushMaterial, 0);
        let alpha_pull = fabric.create_interval(here_id, alpha, 0.5, Material::PullMaterial, 0);
        let omega_pull = fabric.create_interval(here_id, omega, 0.5, Material::PullMaterial, 0);
        pulls.push(alpha_pull);
        pulls.push(omega_pull);
        Self {
            interval_id,
            alpha_pulls: vec![alpha_pull],
            omega_pulls: vec![omega_pull],
        }
    }

    pub fn add_pull(&mut self, end: &End, pull_id: UniqueId) {
        match end {
            End::Alpha => {
                self.alpha_pulls.push(pull_id);
            }
            End::Omega => {
                self.omega_pulls.push(pull_id);
            }
        }
    }

    pub fn join_pushes(
        fabric: &mut Fabric,
        evolving_pushes: &mut Vec<EvolvingPush>,
        snapshot_a: (usize, IntervalSnapshot),
        snapshot_b: (usize, IntervalSnapshot),
    ) {
        let ends = [
            (End::Alpha, End::Alpha),
            (End::Alpha, End::Omega),
            (End::Omega, End::Alpha),
            (End::Omega, End::Omega),
        ];
        for (end_a, end_b) in ends {
            let index_a = snapshot_a.1.end_index(&end_a);
            let index_b = snapshot_b.1.end_index(&end_b);
            let pull = fabric.create_interval(index_a, index_b, 0.5, Material::PullMaterial, 0);
            evolving_pushes[snapshot_a.0].add_pull(&end_a, pull);
            evolving_pushes[snapshot_b.0].add_pull(&end_b, pull);
        }
    }

    fn new(interval_id: UniqueId) -> Self {
        Self {
            interval_id,
            alpha_pulls: vec![],
            omega_pulls: vec![],
        }
    }
}
