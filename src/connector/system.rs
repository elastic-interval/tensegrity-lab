//! Runtime connector state for a fabric: hardware dimensions plus the
//! cable-to-slot assignments per push interval. Present on a `Fabric` only
//! when a physical build is intended; absent, connectors play no role.

use crate::connector::attachment::{
    calculate_interval_attachment_points, PullConnection, PullConnections, PullIntervalData,
    ATTACHMENT_POINTS,
};
use crate::connector::ConnectorDimensions;
use crate::fabric::interval::Role;
use crate::fabric::{Fabric, IntervalEnd, IntervalKey, JointKey};
use glam::Vec3;
use slotmap::SecondaryMap;

#[derive(Clone, Debug)]
pub struct ConnectorSystem {
    pub dimensions: ConnectorDimensions,
    /// Slot assignments per push interval, rebuilt on demand from current
    /// geometry (`update_all_connections`) — never during the physics tick.
    pub connections: SecondaryMap<IntervalKey, PullConnections>,
}

impl ConnectorSystem {
    pub fn new(dimensions: ConnectorDimensions) -> Self {
        Self {
            dimensions,
            connections: SecondaryMap::new(),
        }
    }

    pub fn connections(
        &self,
        push_key: IntervalKey,
        end: IntervalEnd,
    ) -> Option<&[Option<PullConnection>; ATTACHMENT_POINTS]> {
        self.connections.get(push_key).map(|conn| conn.connections(end))
    }

    pub fn connections_mut(&mut self, push_key: IntervalKey) -> Option<&mut PullConnections> {
        self.connections.get_mut(push_key)
    }

    pub fn ring_center(&self, push_end: Vec3, push_axis: Vec3, slot: usize) -> Vec3 {
        self.dimensions.ring_center(push_end, push_axis, slot)
    }

    /// `(pivot_pos, elevation_deg)` — see `ConnectorDimensions::pivot_geometry`.
    pub fn pivot_geometry(
        &self,
        push_end: Vec3,
        push_axis: Vec3,
        slot: usize,
        pull_other_end: Vec3,
    ) -> (Vec3, f32) {
        self.dimensions
            .pivot_geometry(push_end, push_axis, slot, pull_other_end)
    }

    /// Rebuild slot assignments for every push interval from current geometry.
    pub fn update_all_connections(&mut self, fabric: &Fabric) {
        self.connections.clear();
        if fabric.joints.is_empty() {
            return;
        }
        let push_keys: Vec<IntervalKey> = fabric
            .intervals
            .iter()
            .filter_map(|(key, interval)| interval.has_role(Role::Pushing).then_some(key))
            .collect();
        for push_key in push_keys {
            self.update_push_connections(fabric, push_key);
        }
    }

    /// Assign all pull intervals connected to one push interval to their
    /// optimal attachment slots.
    fn update_push_connections(&mut self, fabric: &Fabric, push_key: IntervalKey) {
        let Some(push_interval) = fabric.intervals.get(push_key) else {
            return;
        };
        if !push_interval.has_role(Role::Pushing) {
            return;
        }
        let push_alpha = push_interval.alpha_key;
        let push_omega = push_interval.omega_key;

        let mut connected_pulls = Vec::new();
        let mut pull_data = Vec::new();
        for (key, interval) in fabric.intervals.iter() {
            if !interval.role.is_pull_like() {
                continue;
            }
            if interval.alpha_key == push_alpha
                || interval.alpha_key == push_omega
                || interval.omega_key == push_alpha
                || interval.omega_key == push_omega
            {
                connected_pulls.push((key, interval.alpha_key, interval.omega_key));
                pull_data.push(PullIntervalData {
                    key,
                    alpha_key: interval.alpha_key,
                    omega_key: interval.omega_key,
                    strain: interval.strain,
                    unit: interval.unit,
                });
            }
        }

        let (alpha_location, omega_location) = push_interval.locations(&fabric.joints);
        let (alpha_points, omega_points) = calculate_interval_attachment_points(
            alpha_location,
            omega_location,
            &self.dimensions,
        );

        let mut connections = PullConnections::new();
        connections.reorder_connections(
            &alpha_points,
            &omega_points,
            &fabric.joints,
            &connected_pulls,
            &pull_data,
            push_alpha,
            push_omega,
            &self.dimensions,
        );
        self.connections.insert(push_key, connections);
    }

    /// Pivot pin position for the given pull's end at joint `near`, aimed at
    /// the pull's far joint. `None` when that end isn't attached to any push.
    /// Aiming at the far pivot instead of the far joint is a second pass over
    /// this: see the renderers, which use it to keep forks collinear with
    /// their cables on short spans.
    pub fn pull_end_pivot(
        &self,
        fabric: &Fabric,
        pull_key: IntervalKey,
        near: JointKey,
    ) -> Option<Vec3> {
        for (push_key, push) in fabric.intervals.iter() {
            if !push.has_role(Role::Pushing) {
                continue;
            }
            for end in [IntervalEnd::Alpha, IntervalEnd::Omega] {
                let end_joint = match end {
                    IntervalEnd::Alpha => push.alpha_key,
                    IntervalEnd::Omega => push.omega_key,
                };
                if end_joint != near {
                    continue;
                }
                let Some(conns) = self.connections(push_key, end) else {
                    continue;
                };
                for (slot_idx, conn_opt) in conns.iter().enumerate() {
                    let Some(conn) = conn_opt else { continue };
                    if conn.pull_interval_key != pull_key {
                        continue;
                    }
                    let alpha_pos = fabric.joints[push.alpha_key].location;
                    let omega_pos = fabric.joints[push.omega_key].location;
                    let push_dir = (omega_pos - alpha_pos).normalize();
                    let (end_pos, axis_dir) = match end {
                        IntervalEnd::Alpha => (alpha_pos, -push_dir),
                        IntervalEnd::Omega => (omega_pos, push_dir),
                    };
                    let pull = &fabric.intervals[pull_key];
                    let other = if pull.alpha_key == near {
                        fabric.joints[pull.omega_key].location
                    } else {
                        fabric.joints[pull.alpha_key].location
                    };
                    let (pivot_pos, _elevation) =
                        self.pivot_geometry(end_pos, axis_dir, slot_idx, other);
                    return Some(pivot_pos);
                }
            }
        }
        None
    }

    /// Free pivot elevation angle (degrees) at every cable end.
    pub fn collect_pivot_angles(&self, fabric: &Fabric) -> Vec<f32> {
        let mut angles = Vec::new();
        for (push_key, push_interval) in fabric.intervals.iter() {
            if !push_interval.has_role(Role::Pushing) {
                continue;
            }
            let alpha_pos = fabric.joints[push_interval.alpha_key].location;
            let omega_pos = fabric.joints[push_interval.omega_key].location;
            let push_dir = (omega_pos - alpha_pos).normalize();

            for interval_end in [IntervalEnd::Alpha, IntervalEnd::Omega] {
                let (end_pos, axis_dir, end_key) = match interval_end {
                    IntervalEnd::Alpha => (alpha_pos, -push_dir, push_interval.alpha_key),
                    IntervalEnd::Omega => (omega_pos, push_dir, push_interval.omega_key),
                };
                let Some(connections) = self.connections(push_key, interval_end) else {
                    continue;
                };
                for (slot_idx, conn_opt) in connections.iter().enumerate() {
                    let Some(connection) = conn_opt else { continue };
                    let Some(pull_interval) = fabric.intervals.get(connection.pull_interval_key)
                    else {
                        continue;
                    };
                    let pull_other_end = if pull_interval.alpha_key == end_key {
                        fabric.joints[pull_interval.omega_key].location
                    } else {
                        fabric.joints[pull_interval.alpha_key].location
                    };
                    let (_, elevation_deg) =
                        self.pivot_geometry(end_pos, axis_dir, slot_idx, pull_other_end);
                    angles.push(elevation_deg);
                }
            }
        }
        angles
    }
}
