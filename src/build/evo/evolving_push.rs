use crate::fabric::material::Material;
use crate::fabric::{Fabric, UniqueId};
use cgmath::Point3;

pub struct EvolvingPush {
    interval_id: UniqueId,
    alpha_pulls: Vec<UniqueId>,
    omega_pulls: Vec<UniqueId>,
}

enum End {
    Alpha,
    Omega,
}

impl EvolvingPush {
    pub fn first_push(fabric: &mut Fabric) -> Self {
        let alpha = fabric.create_joint(Point3::new(0.5, 0.0, 0.0));
        let omega = fabric.create_joint(Point3::new(-0.5, 0.0, 0.0));
        let interval_id = fabric.create_interval(alpha, omega, 1.0, Material::PushMaterial, 0);
        Self::new(interval_id)
    }

    pub fn new_push(&mut self, fabric: &mut Fabric, end: End) -> Self {
        let interval = fabric.interval(self.interval_id);
        let locations = interval.locations(&fabric.joints);
        let (here_id, here, pulls) = match end {
            End::Alpha => (interval.alpha_index, locations.0, &mut self.alpha_pulls),
            End::Omega => (interval.omega_index, locations.1, &mut self.omega_pulls),
        };
        let alpha = fabric.create_joint(here - interval.unit / 2.0);
        let omega = fabric.create_joint(here + interval.unit / 2.0);
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

    fn new(interval_id: UniqueId) -> Self {
        Self {
            interval_id,
            alpha_pulls: vec![],
            omega_pulls: vec![],
        }
    }
}
