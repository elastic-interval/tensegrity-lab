use crate::fabric::interval::{IntervalSnapshot, Role};
use crate::fabric::IntervalEnd;
use crate::fabric::{Fabric, IntervalKey};
use cgmath::{Point3, Vector3};

pub struct EvolvingPush {
    pub interval_key: IntervalKey,
    alpha_pulls: Vec<IntervalKey>,
    omega_pulls: Vec<IntervalKey>,
}

impl EvolvingPush {}

impl EvolvingPush {
    pub fn first_push(fabric: &mut Fabric) -> Self {
        let alpha = fabric.create_joint(Point3::new(0.5, 0.0, 0.0));
        let omega = fabric.create_joint(Point3::new(-0.5, 0.0, 0.0));
        let interval_key = fabric.create_slack_interval(alpha, omega, Role::Pushing);
        Self::new(interval_key)
    }

    pub fn end_push(
        &mut self,
        fabric: &mut Fabric,
        snapshot: IntervalSnapshot,
        end: IntervalEnd,
        project: Vector3<f32>,
    ) -> Self {
        let IntervalSnapshot {
            interval,
            alpha,
            omega,
        } = snapshot;
        let (here_key, here, pulls) = match end {
            IntervalEnd::Alpha => (interval.alpha_key, alpha.location, &mut self.alpha_pulls),
            IntervalEnd::Omega => (interval.omega_key, omega.location, &mut self.omega_pulls),
        };
        let alpha_key = fabric.create_joint(&here - project / 2.0);
        let omega_key = fabric.create_joint(&here + project / 2.0);
        let interval_key = fabric.create_slack_interval(alpha_key, omega_key, Role::Pushing);
        let alpha_pull = fabric.create_slack_interval(here_key, alpha_key, Role::Pulling);
        let omega_pull = fabric.create_slack_interval(here_key, omega_key, Role::Pulling);
        pulls.push(alpha_pull);
        pulls.push(omega_pull);
        Self {
            interval_key,
            alpha_pulls: vec![alpha_pull],
            omega_pulls: vec![omega_pull],
        }
    }

    pub fn add_pull(&mut self, end: &IntervalEnd, pull_key: IntervalKey) {
        match end {
            IntervalEnd::Alpha => {
                self.alpha_pulls.push(pull_key);
            }
            IntervalEnd::Omega => {
                self.omega_pulls.push(pull_key);
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
            (IntervalEnd::Alpha, IntervalEnd::Alpha),
            (IntervalEnd::Alpha, IntervalEnd::Omega),
            (IntervalEnd::Omega, IntervalEnd::Alpha),
            (IntervalEnd::Omega, IntervalEnd::Omega),
        ];
        for (end_a, end_b) in ends {
            let key_a = snapshot_a.1.end_key(&end_a);
            let key_b = snapshot_b.1.end_key(&end_b);
            let pull = fabric.create_slack_interval(key_a, key_b, Role::Pulling);
            evolving_pushes[snapshot_a.0].add_pull(&end_a, pull);
            evolving_pushes[snapshot_b.0].add_pull(&end_b, pull);
        }
    }

    fn new(interval_key: IntervalKey) -> Self {
        Self {
            interval_key,
            alpha_pulls: vec![],
            omega_pulls: vec![],
        }
    }
}
