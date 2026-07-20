//! Runtime connector state for a fabric: hardware dimensions plus the
//! cable-to-slot assignments per push interval. Present on a `Fabric` only
//! when a physical build is intended; absent, connectors play no role.

use crate::connector::attachment::{
    calculate_interval_attachment_points, PullConnection, PullConnections, PullIntervalData,
    TabBend, ATTACHMENT_POINTS,
};
use crate::connector::{bend_optimizer, ConnectorDimensions};
use crate::fabric::interval::Role;
use crate::fabric::{Fabric, FabricDimensions, IntervalEnd, IntervalKey};
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

    /// `(tab_pos, tab_bend, pull_end_pos, ideal_deg)` — see `ConnectorDimensions::tab_geometry`.
    pub fn tab_geometry(
        &self,
        fabric_dimensions: &FabricDimensions,
        push_end: Vec3,
        push_axis: Vec3,
        slot: usize,
        pull_other_end: Vec3,
    ) -> (Vec3, TabBend, Vec3, f32) {
        self.dimensions.tab_geometry(
            fabric_dimensions.push_radius,
            push_end,
            push_axis,
            slot,
            pull_other_end,
        )
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
            fabric.dimensions.push_radius,
        );
        self.connections.insert(push_key, connections);
    }

    /// Update `self.dimensions.bend_magnitudes` with the K-center optimal set
    /// for this fabric's cable ends. No-op when locked, K=0, or no pulls.
    pub fn recompute_bend_magnitudes(&mut self, fabric: &Fabric) {
        if self.dimensions.bend_magnitudes_locked {
            return;
        }
        let k = self.dimensions.bend_count;
        if k == 0 {
            return;
        }
        let ideals = self.collect_ideal_bend_angles(fabric);
        if ideals.is_empty() {
            return;
        }
        self.dimensions.bend_magnitudes = bend_optimizer::optimize_magnitudes(&ideals, k);
    }

    /// Continuous ideal bend angle (degrees) at every cable end.
    pub fn collect_ideal_bend_angles(&self, fabric: &Fabric) -> Vec<f32> {
        let mut ideals = Vec::new();
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
                    let (_, _, _, ideal_deg) = self.tab_geometry(
                        &fabric.dimensions,
                        end_pos,
                        axis_dir,
                        slot_idx,
                        pull_other_end,
                    );
                    ideals.push(ideal_deg);
                }
            }
        }
        ideals
    }
}
