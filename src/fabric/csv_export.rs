use cgmath::{InnerSpace, Point3};
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use crate::fabric::attachment::HingeBend;
use crate::fabric::interval::Role;
use crate::fabric::{Fabric, FabricDimensions, IntervalEnd, IntervalKey, JointKey};
use crate::units::MM_PER_METER;

impl Fabric {
    /// Export fabric intervals to CSV with hinge positions and angles.
    pub fn snapshot_csv(&mut self, filename: &str) -> io::Result<()> {
        self.snapshot_csv_with_phase(filename, None)
    }

    /// Export fabric intervals to CSV with phase indicator.
    pub fn snapshot_csv_with_phase(
        &mut self,
        filename: &str,
        phase: Option<&str>,
    ) -> io::Result<()> {
        self.update_all_attachment_connections();

        let path = Path::new(filename);
        let mut file = File::create(path)?;

        let dimensions = &self.dimensions;
        let height_mm = self
            .joints
            .values()
            .fold(0.0f32, |h, joint| h.max(joint.location.y))
            * MM_PER_METER;
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();

        // Header comments
        let phase_str = phase.unwrap_or("unknown");
        writeln!(
            file,
            "# {}, Phase: {}, Height: {:.1}mm, Created: {}",
            self.name, phase_str, height_mm, now
        )?;
        write_dimensions_comments(&mut file, &self.dimensions)?;
        writeln!(file, "Index,Role,Length(m),Strain,AlphaX,AlphaY,AlphaZ,AlphaJoint,AlphaSlot,AlphaAngle,OmegaX,OmegaY,OmegaZ,OmegaJoint,OmegaSlot,OmegaAngle")?;

        // Build a map of pull interval connections for each push interval
        // Key: (pull_interval_key, end, slot) -> (pull_end_pos, hinge_pos, joint_key, slot, hinge_bend)
        let mut pull_hinge_info: std::collections::HashMap<
            (IntervalKey, IntervalEnd, usize),
            (Point3<f32>, Point3<f32>, JointKey, usize, HingeBend),
        > = std::collections::HashMap::new();

        // First pass: collect hinge info from push intervals using hinge_geometry
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
                        if let Some(pull_interval) =
                            self.intervals.get(connection.pull_interval_key)
                        {
                            let pull_other_end =
                                if pull_interval.alpha_key == push_interval.alpha_key {
                                    self.joints[pull_interval.omega_key].location
                                } else {
                                    self.joints[pull_interval.alpha_key].location
                                };

                            let (hinge_pos, hinge_bend, pull_end_pos) = dimensions.hinge_geometry(
                                alpha_pos,
                                -push_dir,
                                slot_idx,
                                pull_other_end,
                            );

                            let pull_end = if pull_interval.alpha_key == push_interval.alpha_key {
                                IntervalEnd::Alpha
                            } else {
                                IntervalEnd::Omega
                            };
                            pull_hinge_info.insert(
                                (connection.pull_interval_key, pull_end, slot_idx + 1),
                                (
                                    pull_end_pos,
                                    hinge_pos,
                                    push_interval.alpha_key,
                                    slot_idx + 1,
                                    hinge_bend,
                                ),
                            );
                        }
                    }
                }
            }

            // Process omega end
            if let Some(connections) = push_interval.connections(IntervalEnd::Omega) {
                for (slot_idx, conn_opt) in connections.iter().enumerate() {
                    if let Some(connection) = conn_opt {
                        if let Some(pull_interval) =
                            self.intervals.get(connection.pull_interval_key)
                        {
                            let pull_other_end =
                                if pull_interval.alpha_key == push_interval.omega_key {
                                    self.joints[pull_interval.omega_key].location
                                } else {
                                    self.joints[pull_interval.alpha_key].location
                                };

                            let (hinge_pos, hinge_bend, pull_end_pos) = dimensions.hinge_geometry(
                                omega_pos,
                                push_dir,
                                slot_idx,
                                pull_other_end,
                            );

                            let pull_end = if pull_interval.alpha_key == push_interval.omega_key {
                                IntervalEnd::Alpha
                            } else {
                                IntervalEnd::Omega
                            };
                            pull_hinge_info.insert(
                                (connection.pull_interval_key, pull_end, slot_idx + 1),
                                (
                                    pull_end_pos,
                                    hinge_pos,
                                    push_interval.omega_key,
                                    slot_idx + 1,
                                    hinge_bend,
                                ),
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

        let mut interval_infos: Vec<IntervalInfo> = self
            .intervals
            .iter()
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
        interval_infos.sort_by(|a, b| match (a.is_push, b.is_push) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a
                .length
                .partial_cmp(&b.length)
                .unwrap_or(std::cmp::Ordering::Equal),
        });

        // Build a map of highest slot per joint (for FEA push endpoints)
        let mut highest_slot_per_joint: std::collections::HashMap<JointKey, usize> =
            std::collections::HashMap::new();
        for ((_, _, slot), (_, _, joint_key, _, _)) in &pull_hinge_info {
            let entry = highest_slot_per_joint.entry(*joint_key).or_insert(0);
            if *slot > *entry {
                *entry = *slot;
            }
        }

        // Build a map of ring centers for pull-fea (joint_key, slot) -> ring_center
        let mut ring_centers: std::collections::HashMap<(JointKey, usize), Point3<f32>> =
            std::collections::HashMap::new();

        for (_key, push_interval) in self.intervals.iter() {
            if !push_interval.has_role(Role::Pushing) {
                continue;
            }
            let alpha_pos = self.joints[push_interval.alpha_key].location;
            let omega_pos = self.joints[push_interval.omega_key].location;
            let push_dir = (omega_pos - alpha_pos).normalize();

            // For each end, calculate ring centers at all slots
            for slot in 1..=3 {
                // Alpha end
                let alpha_ring = alpha_pos + (-push_dir) * *dimensions.hinge.disc_thickness * slot as f32;
                ring_centers.insert((push_interval.alpha_key, slot), alpha_ring);

                // Omega end
                let omega_ring = omega_pos + push_dir * *dimensions.hinge.disc_thickness * slot as f32;
                ring_centers.insert((push_interval.omega_key, slot), omega_ring);
            }
        }

        // Write sorted intervals
        let mut current_index = 0usize;
        for info in interval_infos.iter() {
            current_index += 1;
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
                    current_index,
                    role_str,
                    info.length,
                    info.strain,
                    alpha.x, alpha.y, alpha.z, alpha_joint.path,
                    omega.x, omega.y, omega.z, omega_joint.path,
                )?;
            } else {
                // Find pull_end_pos (shortened by hinge_length) for each end
                let alpha_info = pull_hinge_info
                    .iter()
                    .find(|((pull_id, end, _), _)| {
                        *pull_id == info.key && *end == IntervalEnd::Alpha
                    })
                    .map(|(_, data)| data);
                let omega_info = pull_hinge_info
                    .iter()
                    .find(|((pull_id, end, _), _)| {
                        *pull_id == info.key && *end == IntervalEnd::Omega
                    })
                    .map(|(_, data)| data);

                // Use pull_end_pos (first element) for the interval position
                let (alpha_pos, alpha_joint_path, alpha_slot, alpha_bend) =
                    if let Some((pull_end_pos, _, joint_key, slot, bend)) = alpha_info {
                        (
                            Point3::new(pull_end_pos.x, pull_end_pos.y, pull_end_pos.z)
                                * MM_PER_METER,
                            self.joints[*joint_key].path.to_string(),
                            *slot,
                            Some(*bend),
                        )
                    } else {
                        let joint = &self.joints[interval.alpha_key];
                        (joint.location * MM_PER_METER, joint.path.to_string(), 0, None)
                    };

                let (omega_pos, omega_joint_path, omega_slot, omega_bend) =
                    if let Some((pull_end_pos, _, joint_key, slot, bend)) = omega_info {
                        (
                            Point3::new(pull_end_pos.x, pull_end_pos.y, pull_end_pos.z)
                                * MM_PER_METER,
                            self.joints[*joint_key].path.to_string(),
                            *slot,
                            Some(*bend),
                        )
                    } else {
                        let joint = &self.joints[interval.omega_key];
                        (joint.location * MM_PER_METER, joint.path.to_string(), 0, None)
                    };

                // Calculate shortened length
                let shortened_length = (omega_pos - alpha_pos).magnitude() / MM_PER_METER;

                // Format hinge bend as string (empty if not attached)
                let alpha_bend_str = alpha_bend.map_or(String::new(), |b| b.to_string());
                let omega_bend_str = omega_bend.map_or(String::new(), |b| b.to_string());

                writeln!(
                    file,
                    "{},{},{:.3},{:.3e},{:.3},{:.3},{:.3},{},{},{},{:.3},{:.3},{:.3},{},{},{}",
                    current_index,
                    role_str,
                    shortened_length,
                    info.strain,
                    alpha_pos.x,
                    alpha_pos.y,
                    alpha_pos.z,
                    alpha_joint_path,
                    alpha_slot,
                    alpha_bend_str,
                    omega_pos.x,
                    omega_pos.y,
                    omega_pos.z,
                    omega_joint_path,
                    omega_slot,
                    omega_bend_str,
                )?;
            }
        }

        // Write FEA intervals (push-fea first, then pull-fea, sorted by length)
        // push-fea: extends to ring center at highest slot at each end
        // pull-fea: connects at ring centers instead of hinge endpoints

        // Collect FEA interval data
        struct FeaIntervalInfo {
            is_push: bool,
            length: f32,
            strain: f32,
            alpha_pos: Point3<f32>,
            omega_pos: Point3<f32>,
            alpha_joint_path: String,
            omega_joint_path: String,
            alpha_slot: usize,
            omega_slot: usize,
        }

        let mut fea_infos: Vec<FeaIntervalInfo> = Vec::new();

        // Generate push-fea intervals
        for info in interval_infos.iter().filter(|i| i.is_push) {
            let interval = self.intervals.get(info.key).unwrap();
            let alpha_joint = &self.joints[interval.alpha_key];
            let omega_joint = &self.joints[interval.omega_key];

            // Get highest slot at each end
            let alpha_highest = highest_slot_per_joint
                .get(&interval.alpha_key)
                .copied()
                .unwrap_or(0);
            let omega_highest = highest_slot_per_joint
                .get(&interval.omega_key)
                .copied()
                .unwrap_or(0);

            // Get ring centers at highest slots
            let alpha_fea = if alpha_highest > 0 {
                ring_centers
                    .get(&(interval.alpha_key, alpha_highest))
                    .copied()
                    .unwrap_or(alpha_joint.location)
            } else {
                alpha_joint.location
            };
            let omega_fea = if omega_highest > 0 {
                ring_centers
                    .get(&(interval.omega_key, omega_highest))
                    .copied()
                    .unwrap_or(omega_joint.location)
            } else {
                omega_joint.location
            };

            let fea_length = (omega_fea - alpha_fea).magnitude();

            fea_infos.push(FeaIntervalInfo {
                is_push: true,
                length: fea_length,
                strain: info.strain,
                alpha_pos: alpha_fea,
                omega_pos: omega_fea,
                alpha_joint_path: alpha_joint.path.to_string(),
                omega_joint_path: omega_joint.path.to_string(),
                alpha_slot: alpha_highest,
                omega_slot: omega_highest,
            });
        }

        // Generate pull-fea intervals
        for info in interval_infos.iter().filter(|i| !i.is_push) {
            let interval = self.intervals.get(info.key).unwrap();

            // Find connection info for each end
            let alpha_info = pull_hinge_info
                .iter()
                .find(|((pull_id, end, _), _)| *pull_id == info.key && *end == IntervalEnd::Alpha)
                .map(|((_, _, slot), (_, _, joint_key, _, _))| (*joint_key, *slot));
            let omega_info = pull_hinge_info
                .iter()
                .find(|((pull_id, end, _), _)| *pull_id == info.key && *end == IntervalEnd::Omega)
                .map(|((_, _, slot), (_, _, joint_key, _, _))| (*joint_key, *slot));

            // Get ring centers at connection slots
            let (alpha_fea, alpha_joint_path, alpha_slot) =
                if let Some((joint_key, slot)) = alpha_info {
                    let ring = ring_centers
                        .get(&(joint_key, slot))
                        .copied()
                        .unwrap_or(self.joints[joint_key].location);
                    (ring, self.joints[joint_key].path.to_string(), slot)
                } else {
                    let joint = &self.joints[interval.alpha_key];
                    (joint.location, joint.path.to_string(), 0)
                };

            let (omega_fea, omega_joint_path, omega_slot) =
                if let Some((joint_key, slot)) = omega_info {
                    let ring = ring_centers
                        .get(&(joint_key, slot))
                        .copied()
                        .unwrap_or(self.joints[joint_key].location);
                    (ring, self.joints[joint_key].path.to_string(), slot)
                } else {
                    let joint = &self.joints[interval.omega_key];
                    (joint.location, joint.path.to_string(), 0)
                };

            let fea_length = (omega_fea - alpha_fea).magnitude();

            fea_infos.push(FeaIntervalInfo {
                is_push: false,
                length: fea_length,
                strain: info.strain,
                alpha_pos: alpha_fea,
                omega_pos: omega_fea,
                alpha_joint_path,
                omega_joint_path,
                alpha_slot,
                omega_slot,
            });
        }

        // Sort FEA intervals: push-fea first, then pull-fea; within each group, short to long
        fea_infos.sort_by(|a, b| match (a.is_push, b.is_push) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a
                .length
                .partial_cmp(&b.length)
                .unwrap_or(std::cmp::Ordering::Equal),
        });

        // Write FEA intervals
        for fea in &fea_infos {
            current_index += 1;
            let role_str = if fea.is_push { "push-fea" } else { "pull-fea" };
            let alpha_mm = fea.alpha_pos * MM_PER_METER;
            let omega_mm = fea.omega_pos * MM_PER_METER;

            writeln!(
                file,
                "{},{},{:.3},{:.3e},{:.3},{:.3},{:.3},{},{},90.000,{:.3},{:.3},{:.3},{},{},90.000",
                current_index,
                role_str,
                fea.length,
                fea.strain,
                alpha_mm.x, alpha_mm.y, alpha_mm.z, fea.alpha_joint_path, fea.alpha_slot,
                omega_mm.x, omega_mm.y, omega_mm.z, fea.omega_joint_path, fea.omega_slot,
            )?;
        }

        // Build link structure for each push interval end
        // Group connections by push joint to build the axial chain
        // Each entry stores: (slot, pull_end_pos, hinge_pos)
        let mut push_end_connections: std::collections::HashMap<
            JointKey,
            Vec<(usize, Point3<f32>, Point3<f32>)>,
        > = std::collections::HashMap::new();

        for (_, (pull_end_pos, hinge_pos, joint_key, slot, _)) in &pull_hinge_info {
            push_end_connections.entry(*joint_key).or_default().push((
                *slot,
                *pull_end_pos,
                *hinge_pos,
            ));
        }

        let mut link_index = current_index;

        for (joint_key, mut connections) in push_end_connections {
            connections.sort_by_key(|(slot, _, _)| *slot);

            let joint = &self.joints[joint_key];
            let joint_pos = joint.location;

            // Find push axis direction (outward from this joint)
            let push_axis = self
                .intervals
                .values()
                .find(|i| {
                    i.has_role(Role::Pushing)
                        && (i.alpha_key == joint_key || i.omega_key == joint_key)
                })
                .map(|push_interval| {
                    let alpha_pos = self.joints[push_interval.alpha_key].location;
                    let omega_pos = self.joints[push_interval.omega_key].location;
                    let dir = (omega_pos - alpha_pos).normalize();
                    if push_interval.alpha_key == joint_key {
                        -dir
                    } else {
                        dir
                    }
                })
                .unwrap_or(cgmath::Vector3::new(0.0, 1.0, 0.0));

            let mut prev_pos = joint_pos;
            let mut prev_slot = 0usize;

            let joint_path = &joint.path;
            for (slot, pull_end_pos, hinge_pos) in &connections {
                // Ring center at this slot (1x, 2x, 3x ring_thickness)
                let ring_center = joint_pos + push_axis * *dimensions.hinge.disc_thickness * *slot as f32;

                // Axial link: previous position → ring center
                link_index += 1;
                let prev_mm = prev_pos * MM_PER_METER;
                let ring_mm = ring_center * MM_PER_METER;
                let axial_length = (ring_center - prev_pos).magnitude();
                writeln!(
                    file,
                    "{},axial,{:.3},0.000e0,{:.3},{:.3},{:.3},{},{},90.000,{:.3},{:.3},{:.3},{},{},90.000",
                    link_index, axial_length,
                    prev_mm.x, prev_mm.y, prev_mm.z, joint_path, prev_slot,
                    ring_mm.x, ring_mm.y, ring_mm.z, joint_path, slot,
                )?;

                // Radial link: ring center → hinge
                link_index += 1;
                let hinge_mm = *hinge_pos * MM_PER_METER;
                let radial_length = (*hinge_pos - ring_center).magnitude();
                writeln!(
                    file,
                    "{},radial,{:.3},0.000e0,{:.3},{:.3},{:.3},{},{},0.000,{:.3},{:.3},{:.3},{},{},0.000",
                    link_index, radial_length,
                    ring_mm.x, ring_mm.y, ring_mm.z, joint_path, slot,
                    hinge_mm.x, hinge_mm.y, hinge_mm.z, joint_path, slot,
                )?;

                // Hinge link: hinge → pull_end (along pull direction)
                link_index += 1;
                let pull_end_mm = *pull_end_pos * MM_PER_METER;
                let hinge_link_length = (*pull_end_pos - *hinge_pos).magnitude();
                writeln!(
                    file,
                    "{},hinge,{:.3},0.000e0,{:.3},{:.3},{:.3},{},{},0.000,{:.3},{:.3},{:.3},{},{},0.000",
                    link_index, hinge_link_length,
                    hinge_mm.x, hinge_mm.y, hinge_mm.z, joint_path, slot,
                    pull_end_mm.x, pull_end_mm.y, pull_end_mm.z, joint_path, slot,
                )?;

                prev_pos = ring_center;
                prev_slot = *slot;
            }
        }

        println!("Exported {} to {}", self.name, filename);
        Ok(())
    }
}

fn write_dimensions_comments(file: &mut File, dims: &FabricDimensions) -> io::Result<()> {
    let h = &dims.hinge;
    writeln!(file, "# push_radius: {:.5}m", *h.push_radius)?;
    writeln!(file, "# push_radius_margin: {:.5}m", *h.push_radius_margin)?;
    writeln!(file, "# disc_thickness: {:.5}m", *h.disc_thickness)?;
    writeln!(file, "# disc_separator_thickness: {:.5}m", *h.disc_separator_thickness)?;
    writeln!(file, "# hinge_extension: {:.5}m", *h.hinge_extension)?;
    writeln!(file, "# hinge_hole_diameter: {:.5}m", *h.hinge_hole_diameter)?;
    writeln!(file, "# pull_radius: {:.5}m", *dims.pull_radius)?;
    writeln!(file, "#")?;
    // Derived values
    writeln!(file, "# hinge_offset: {:.5}m", *dims.hinge.offset())?;
    writeln!(file, "# hinge_length: {:.5}m", *dims.hinge.length())?;
    Ok(())
}
