use cgmath::{InnerSpace, Point3};
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use crate::fabric::attachment::ConnectorSpec;
use crate::fabric::interval::Role;
use crate::fabric::{Fabric, IntervalEnd, IntervalKey};
use crate::units::{Degrees, MM_PER_METER};

impl Fabric {
    /// Export fabric intervals to CSV with hinge positions and angles.
    ///
    /// Each row represents an interval with:
    /// - Role (push/pull)
    /// - Alpha position (X, Y, Z in mm), Slot, Angle
    /// - Omega position (X, Y, Z in mm), Slot, Angle
    ///
    /// For push intervals: positions are joint locations, slot=0, angle=90.0 (pushing outward)
    /// For pull intervals: positions are hinge locations, slot=1..n, angle from hinge_angle()
    pub fn snapshot_csv(&mut self, filename: &str) -> io::Result<()> {
        // Recalculate hinge positions before export
        self.update_all_attachment_connections();

        let path = Path::new(filename);
        let mut file = File::create(path)?;

        // Write header with comment showing fabric info
        let height_mm = self
            .joints
            .iter()
            .fold(0.0f32, |h, joint| h.max(joint.location.y))
            * MM_PER_METER;
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
        writeln!(file, "# {}, Height: {:.1}mm, Created: {}", self.name, height_mm, now)?;
        writeln!(file, "Index,Role,Length(m),Strain,AlphaX,AlphaY,AlphaZ,AlphaJoint,AlphaSlot,AlphaAngle,OmegaX,OmegaY,OmegaZ,OmegaJoint,OmegaSlot,OmegaAngle")?;

        let connector = ConnectorSpec::for_scale(self.scale);

        // Build a map of pull interval connections for each push interval
        // Key: (pull_interval_key, end, slot) -> (hinge_pos, joint_index, slot, angle)
        let mut pull_hinge_info: std::collections::HashMap<(IntervalKey, IntervalEnd, usize), (Point3<f32>, usize, usize, Degrees)> =
            std::collections::HashMap::new();

        // First pass: collect hinge info from push intervals
        for (_key, push_interval) in self.intervals.iter() {
            if !push_interval.has_role(Role::Pushing) {
                continue;
            }

            let alpha_pos = self.joints[push_interval.alpha_index].location;
            let omega_pos = self.joints[push_interval.omega_index].location;
            let push_dir = (omega_pos - alpha_pos).normalize();

            // Process alpha end
            if let Some(connections) = push_interval.connections(IntervalEnd::Alpha) {
                for (slot_idx, conn_opt) in connections.iter().enumerate() {
                    if let Some(connection) = conn_opt {
                        if let Some(pull_interval) = self.intervals.get(connection.pull_interval_key) {
                            let pull_other_end = if pull_interval.alpha_index == push_interval.alpha_index {
                                self.joints[pull_interval.omega_index].location
                            } else {
                                self.joints[pull_interval.alpha_index].location
                            };

                            let hinge_pos = connector.hinge_position(
                                alpha_pos,
                                -push_dir,
                                slot_idx,
                                pull_other_end,
                            );

                            // Calculate angle: direction from hinge to other end vs inward push axis
                            let pull_direction = (pull_other_end - hinge_pos).normalize();
                            let angle = ConnectorSpec::hinge_angle(-push_dir, pull_direction);

                            // Store for the pull interval's alpha or omega end
                            let pull_end = if pull_interval.alpha_index == push_interval.alpha_index {
                                IntervalEnd::Alpha
                            } else {
                                IntervalEnd::Omega
                            };
                            pull_hinge_info.insert(
                                (connection.pull_interval_key, pull_end, slot_idx + 1),
                                (hinge_pos, push_interval.alpha_index, slot_idx + 1, angle),
                            );
                        }
                    }
                }
            }

            // Process omega end
            if let Some(connections) = push_interval.connections(IntervalEnd::Omega) {
                for (slot_idx, conn_opt) in connections.iter().enumerate() {
                    if let Some(connection) = conn_opt {
                        if let Some(pull_interval) = self.intervals.get(connection.pull_interval_key) {
                            let pull_other_end = if pull_interval.alpha_index == push_interval.omega_index {
                                self.joints[pull_interval.omega_index].location
                            } else {
                                self.joints[pull_interval.alpha_index].location
                            };

                            let hinge_pos = connector.hinge_position(
                                omega_pos,
                                push_dir,
                                slot_idx,
                                pull_other_end,
                            );

                            let pull_direction = (pull_other_end - hinge_pos).normalize();
                            let angle = ConnectorSpec::hinge_angle(push_dir, pull_direction);

                            let pull_end = if pull_interval.alpha_index == push_interval.omega_index {
                                IntervalEnd::Alpha
                            } else {
                                IntervalEnd::Omega
                            };
                            pull_hinge_info.insert(
                                (connection.pull_interval_key, pull_end, slot_idx + 1),
                                (hinge_pos, push_interval.omega_index, slot_idx + 1, angle),
                            );
                        }
                    }
                }
            }
        }

        // Collect intervals with their lengths for sorting
        struct IntervalInfo {
            key: IntervalKey,
            is_push: bool,
            length: f32,
            strain: f32,
        }

        let mut interval_infos: Vec<IntervalInfo> = self.intervals.iter()
            .filter_map(|(key, interval)| {
                if interval.has_role(Role::Support) {
                    return None;
                }
                let alpha = self.joints[interval.alpha_index].location;
                let omega = self.joints[interval.omega_index].location;
                let length = (omega - alpha).magnitude();
                Some(IntervalInfo {
                    key,
                    is_push: interval.has_role(Role::Pushing),
                    length,
                    strain: interval.strain,
                })
            })
            .collect();

        // Sort: Push first, then Pull; within each group, short to long
        interval_infos.sort_by(|a, b| {
            match (a.is_push, b.is_push) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.length.partial_cmp(&b.length).unwrap_or(std::cmp::Ordering::Equal),
            }
        });

        // Write sorted intervals
        for (index, info) in interval_infos.iter().enumerate() {
            let interval = self.intervals.get(info.key).unwrap();
            let role_str = if info.is_push { "push" } else { "pull" };

            if info.is_push {
                let alpha = self.joints[interval.alpha_index].location * MM_PER_METER;
                let omega = self.joints[interval.omega_index].location * MM_PER_METER;
                writeln!(
                    file,
                    "{},{},{:.3},{:.3e},{:.3},{:.3},{:.3},{},0,90.000,{:.3},{:.3},{:.3},{},0,90.000",
                    index + 1,
                    role_str,
                    info.length,
                    info.strain,
                    alpha.x, alpha.y, alpha.z, interval.alpha_index,
                    omega.x, omega.y, omega.z, interval.omega_index,
                )?;
            } else {
                let alpha_info = pull_hinge_info.iter()
                    .find(|((pull_id, end, _), _)| *pull_id == info.key && *end == IntervalEnd::Alpha)
                    .map(|(_, data)| data);
                let omega_info = pull_hinge_info.iter()
                    .find(|((pull_id, end, _), _)| *pull_id == info.key && *end == IntervalEnd::Omega)
                    .map(|(_, data)| data);

                let (alpha_pos, alpha_joint, alpha_slot, alpha_angle) = if let Some((pos, joint, slot, angle)) = alpha_info {
                    (Point3::new(pos.x, pos.y, pos.z) * MM_PER_METER, *joint, *slot, *angle)
                } else {
                    let loc = self.joints[interval.alpha_index].location * MM_PER_METER;
                    (loc, interval.alpha_index, 0, Degrees(0.0))
                };

                let (omega_pos, omega_joint, omega_slot, omega_angle) = if let Some((pos, joint, slot, angle)) = omega_info {
                    (Point3::new(pos.x, pos.y, pos.z) * MM_PER_METER, *joint, *slot, *angle)
                } else {
                    let loc = self.joints[interval.omega_index].location * MM_PER_METER;
                    (loc, interval.omega_index, 0, Degrees(0.0))
                };

                writeln!(
                    file,
                    "{},{},{:.3},{:.3e},{:.3},{:.3},{:.3},{},{},{:.3},{:.3},{:.3},{:.3},{},{},{:.3}",
                    index + 1,
                    role_str,
                    info.length,
                    info.strain,
                    alpha_pos.x, alpha_pos.y, alpha_pos.z, alpha_joint, alpha_slot, *alpha_angle,
                    omega_pos.x, omega_pos.y, omega_pos.z, omega_joint, omega_slot, *omega_angle,
                )?;
            }
        }

        println!("Exported {} to {}", self.name, filename);
        Ok(())
    }
}
