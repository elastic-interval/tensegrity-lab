use cgmath::{InnerSpace, Point3};
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use crate::fabric::attachment::{ConnectorSpec, IntervalDimensions};
use crate::fabric::interval::Role;
use crate::fabric::{Fabric, IntervalEnd, IntervalKey, JointKey};
use crate::units::{Degrees, MM_PER_METER};

impl Fabric {
    /// Export fabric intervals to CSV with hinge positions and angles.
    pub fn snapshot_csv(&mut self, filename: &str) -> io::Result<()> {
        self.snapshot_csv_with_phase(filename, None)
    }

    /// Export fabric intervals to CSV with phase indicator.
    pub fn snapshot_csv_with_phase(&mut self, filename: &str, phase: Option<&str>) -> io::Result<()> {
        self.update_all_attachment_connections();

        let path = Path::new(filename);
        let mut file = File::create(path)?;

        let connector = ConnectorSpec::for_scale(self.scale);
        let height_mm = self
            .joints
            .values()
            .fold(0.0f32, |h, joint| h.max(joint.location.y))
            * MM_PER_METER;
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();

        // Header comments
        let phase_str = phase.unwrap_or("unknown");
        writeln!(file, "# {}, Phase: {}, Height: {:.1}mm, Created: {}", self.name, phase_str, height_mm, now)?;
        write_dimensions_comments(&mut file, &connector)?;
        writeln!(file, "Index,Role,Length(m),Strain,AlphaX,AlphaY,AlphaZ,AlphaJoint,AlphaSlot,AlphaAngle,OmegaX,OmegaY,OmegaZ,OmegaJoint,OmegaSlot,OmegaAngle")?;

        // Build a map of pull interval connections for each push interval
        // Key: (pull_interval_key, end, slot) -> (pull_end_pos, hinge_pos, joint_key, slot, angle)
        // pull_end_pos is where the pull interval terminates (hinge_pos - hinge_length along pull direction)
        // hinge_pos is the actual hinge location
        let mut pull_hinge_info: std::collections::HashMap<(IntervalKey, IntervalEnd, usize), (Point3<f32>, Point3<f32>, JointKey, usize, Degrees)> =
            std::collections::HashMap::new();

        // First pass: collect hinge info from push intervals
        for (_key, push_interval) in self.intervals.iter() {
            if !push_interval.has_role(Role::Pushing) {
                continue;
            }

            let alpha_pos = self.joints[push_interval.alpha_key].location;
            let omega_pos = self.joints[push_interval.omega_key].location;
            let push_dir = (omega_pos - alpha_pos).normalize();

            // Process alpha end
            if let Some(connections) = push_interval.connections(IntervalEnd::Alpha) {
                for (slot_idx, conn_opt) in connections.iter().enumerate() {
                    if let Some(connection) = conn_opt {
                        if let Some(pull_interval) = self.intervals.get(connection.pull_interval_key) {
                            let pull_other_end = if pull_interval.alpha_key == push_interval.alpha_key {
                                self.joints[pull_interval.omega_key].location
                            } else {
                                self.joints[pull_interval.alpha_key].location
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

                            // Pull end position: pull back from hinge by hinge_length along pull direction
                            let pull_end_pos = hinge_pos + pull_direction * *connector.hinge_length;

                            // Store for the pull interval's alpha or omega end
                            let pull_end = if pull_interval.alpha_key == push_interval.alpha_key {
                                IntervalEnd::Alpha
                            } else {
                                IntervalEnd::Omega
                            };
                            pull_hinge_info.insert(
                                (connection.pull_interval_key, pull_end, slot_idx + 1),
                                (pull_end_pos, hinge_pos, push_interval.alpha_key, slot_idx + 1, angle),
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
                            let pull_other_end = if pull_interval.alpha_key == push_interval.omega_key {
                                self.joints[pull_interval.omega_key].location
                            } else {
                                self.joints[pull_interval.alpha_key].location
                            };

                            let hinge_pos = connector.hinge_position(
                                omega_pos,
                                push_dir,
                                slot_idx,
                                pull_other_end,
                            );

                            let pull_direction = (pull_other_end - hinge_pos).normalize();
                            let angle = ConnectorSpec::hinge_angle(push_dir, pull_direction);

                            // Pull end position: pull back from hinge by hinge_length along pull direction
                            let pull_end_pos = hinge_pos + pull_direction * *connector.hinge_length;

                            let pull_end = if pull_interval.alpha_key == push_interval.omega_key {
                                IntervalEnd::Alpha
                            } else {
                                IntervalEnd::Omega
                            };
                            pull_hinge_info.insert(
                                (connection.pull_interval_key, pull_end, slot_idx + 1),
                                (pull_end_pos, hinge_pos, push_interval.omega_key, slot_idx + 1, angle),
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
                let alpha = self.joints[interval.alpha_key].location;
                let omega = self.joints[interval.omega_key].location;
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
                let alpha_joint = &self.joints[interval.alpha_key];
                let omega_joint = &self.joints[interval.omega_key];
                let alpha = alpha_joint.location * MM_PER_METER;
                let omega = omega_joint.location * MM_PER_METER;
                writeln!(
                    file,
                    "{},{},{:.3},{:.3e},{:.3},{:.3},{:.3},{},0,90.000,{:.3},{:.3},{:.3},{},0,90.000",
                    index + 1,
                    role_str,
                    info.length,
                    info.strain,
                    alpha.x, alpha.y, alpha.z, alpha_joint.id,
                    omega.x, omega.y, omega.z, omega_joint.id,
                )?;
            } else {
                // Find pull_end_pos (shortened by hinge_length) for each end
                let alpha_info = pull_hinge_info.iter()
                    .find(|((pull_id, end, _), _)| *pull_id == info.key && *end == IntervalEnd::Alpha)
                    .map(|(_, data)| data);
                let omega_info = pull_hinge_info.iter()
                    .find(|((pull_id, end, _), _)| *pull_id == info.key && *end == IntervalEnd::Omega)
                    .map(|(_, data)| data);

                // Use pull_end_pos (first element) for the interval position
                let (alpha_pos, alpha_joint_idx, alpha_slot, alpha_angle) = if let Some((pull_end_pos, _, joint_key, slot, angle)) = alpha_info {
                    (Point3::new(pull_end_pos.x, pull_end_pos.y, pull_end_pos.z) * MM_PER_METER, self.joints[*joint_key].id, *slot, *angle)
                } else {
                    let joint = &self.joints[interval.alpha_key];
                    (joint.location * MM_PER_METER, joint.id, 0, Degrees(0.0))
                };

                let (omega_pos, omega_joint_idx, omega_slot, omega_angle) = if let Some((pull_end_pos, _, joint_key, slot, angle)) = omega_info {
                    (Point3::new(pull_end_pos.x, pull_end_pos.y, pull_end_pos.z) * MM_PER_METER, self.joints[*joint_key].id, *slot, *angle)
                } else {
                    let joint = &self.joints[interval.omega_key];
                    (joint.location * MM_PER_METER, joint.id, 0, Degrees(0.0))
                };

                // Calculate shortened length
                let shortened_length = (omega_pos - alpha_pos).magnitude() / MM_PER_METER;

                writeln!(
                    file,
                    "{},{},{:.3},{:.3e},{:.3},{:.3},{:.3},{},{},{:.3},{:.3},{:.3},{:.3},{},{},{:.3}",
                    index + 1,
                    role_str,
                    shortened_length,
                    info.strain,
                    alpha_pos.x, alpha_pos.y, alpha_pos.z, alpha_joint_idx, alpha_slot, *alpha_angle,
                    omega_pos.x, omega_pos.y, omega_pos.z, omega_joint_idx, omega_slot, *omega_angle,
                )?;
            }
        }

        // Build link structure for each push interval end
        // Group connections by push joint to build the axial chain
        // Each entry stores: (slot, pull_end_pos, hinge_pos)
        let mut push_end_connections: std::collections::HashMap<JointKey, Vec<(usize, Point3<f32>, Point3<f32>)>> =
            std::collections::HashMap::new();

        for (_, (pull_end_pos, hinge_pos, joint_key, slot, _)) in &pull_hinge_info {
            push_end_connections
                .entry(*joint_key)
                .or_default()
                .push((*slot, *pull_end_pos, *hinge_pos));
        }

        let mut link_index = interval_infos.len();

        for (joint_key, mut connections) in push_end_connections {
            connections.sort_by_key(|(slot, _, _)| *slot);

            let joint = &self.joints[joint_key];
            let joint_pos = joint.location;

            // Find push axis direction (outward from this joint)
            let push_axis = self.intervals.values()
                .find(|i| i.has_role(Role::Pushing) && (i.alpha_key == joint_key || i.omega_key == joint_key))
                .map(|push_interval| {
                    let alpha_pos = self.joints[push_interval.alpha_key].location;
                    let omega_pos = self.joints[push_interval.omega_key].location;
                    let dir = (omega_pos - alpha_pos).normalize();
                    if push_interval.alpha_key == joint_key { -dir } else { dir }
                })
                .unwrap_or(cgmath::Vector3::new(0.0, 1.0, 0.0));

            let mut prev_pos = joint_pos;
            let mut prev_slot = 0usize;

            for (slot, pull_end_pos, hinge_pos) in &connections {
                // Ring center at this slot (1x, 2x, 3x ring_thickness)
                let ring_center = joint_pos + push_axis * *connector.ring_thickness * *slot as f32;

                // Axial link: previous position → ring center
                link_index += 1;
                let prev_mm = prev_pos * MM_PER_METER;
                let ring_mm = ring_center * MM_PER_METER;
                let axial_length = (ring_center - prev_pos).magnitude();
                writeln!(
                    file,
                    "{},axial,{:.3},0.000e0,{:.3},{:.3},{:.3},{},{},90.000,{:.3},{:.3},{:.3},{},{},90.000",
                    link_index, axial_length,
                    prev_mm.x, prev_mm.y, prev_mm.z, joint.id, prev_slot,
                    ring_mm.x, ring_mm.y, ring_mm.z, joint.id, slot,
                )?;

                // Radial link: ring center → hinge
                link_index += 1;
                let hinge_mm = *hinge_pos * MM_PER_METER;
                let radial_length = (*hinge_pos - ring_center).magnitude();
                writeln!(
                    file,
                    "{},radial,{:.3},0.000e0,{:.3},{:.3},{:.3},{},{},0.000,{:.3},{:.3},{:.3},{},{},0.000",
                    link_index, radial_length,
                    ring_mm.x, ring_mm.y, ring_mm.z, joint.id, slot,
                    hinge_mm.x, hinge_mm.y, hinge_mm.z, joint.id, slot,
                )?;

                // Hinge link: hinge → pull_end (along pull direction)
                link_index += 1;
                let pull_end_mm = *pull_end_pos * MM_PER_METER;
                let hinge_link_length = (*pull_end_pos - *hinge_pos).magnitude();
                writeln!(
                    file,
                    "{},hinge,{:.3},0.000e0,{:.3},{:.3},{:.3},{},{},0.000,{:.3},{:.3},{:.3},{},{},0.000",
                    link_index, hinge_link_length,
                    hinge_mm.x, hinge_mm.y, hinge_mm.z, joint.id, slot,
                    pull_end_mm.x, pull_end_mm.y, pull_end_mm.z, joint.id, slot,
                )?;

                prev_pos = ring_center;
                prev_slot = *slot;
            }
        }

        println!("Exported {} to {}", self.name, filename);
        Ok(())
    }
}

fn write_dimensions_comments(file: &mut File, dims: &IntervalDimensions) -> io::Result<()> {
    writeln!(file, "# push_radius: {:.3}m ({:.1}mm)", *dims.push_radius, dims.push_radius.to_mm())?;
    writeln!(file, "# pull_radius: {:.3}m ({:.1}mm)", *dims.pull_radius, dims.pull_radius.to_mm())?;
    writeln!(file, "# ring_thickness: {:.3}m ({:.1}mm)", *dims.ring_thickness, dims.ring_thickness.to_mm())?;
    writeln!(file, "# hinge_offset: {:.3}m ({:.1}mm)", *dims.hinge_offset, dims.hinge_offset.to_mm())?;
    writeln!(file, "# hinge_length: {:.3}m ({:.1}mm)", *dims.hinge_length, dims.hinge_length.to_mm())?;
    Ok(())
}
