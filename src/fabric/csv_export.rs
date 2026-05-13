use glam::{Mat3, Vec3};
use std::f32::consts::FRAC_PI_2;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use crate::fabric::attachment::HingeBend;
use crate::fabric::interval::Role;
use crate::fabric::{Fabric, FabricDimensions, IntervalEnd, IntervalKey, JointKey};
use crate::units::{Unit, MM_PER_METER};

/// Rotation from simulation space (Y-up) to CSV space (Z-up, RFEM/Rhino).
///
/// Verified against OpenClaw grav_pretenst data: in sim space the vertical
/// extent is clearly in `.y` (feet at 0, top at ~7 m), while `.x` and `.z`
/// are horizontal. A +90° rotation about X sends sim +Y → csv +Z, so the
/// vertical extent lands in the CSV's Z column. Sim +Z → csv −Y keeps the
/// transform a pure right-handed rotation.
fn sim_to_csv() -> Mat3 {
    Mat3::from_rotation_x(FRAC_PI_2)
}

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
        self.recompute_bend_magnitudes();

        let path = Path::new(filename);
        let mut file = File::create(path)?;

        let dimensions = &self.dimensions;
        let to_csv = sim_to_csv();
        let height_mm = self
            .joints
            .values()
            .fold(0.0f32, |h, joint| h.max(joint.location.y))
            * MM_PER_METER;

        // Collect every joint with its CSV-space (rotated, mm) position and
        // its path identifier so we can record the three lowest points and
        // the single highest one as header comments. Lowest/highest here is
        // measured along the CSV-space vertical axis (Z), which — after the
        // sim→csv rotation — equals simulation Y.
        let joints_csv: Vec<(String, Vec3)> = self
            .joints
            .values()
            .map(|j| (j.path.to_string(), to_csv * (j.location * MM_PER_METER)))
            .collect();
        let mut sorted_by_z = joints_csv.clone();
        sorted_by_z.sort_by(|a, b| a.1.z.partial_cmp(&b.1.z).unwrap_or(std::cmp::Ordering::Equal));
        let lowest_three: Vec<&(String, Vec3)> = sorted_by_z.iter().take(3).collect();
        let highest_one: &(String, Vec3) = sorted_by_z.last().expect("fabric has at least one joint");

        let now = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();

        let phase_str = phase.unwrap_or("unknown");
        writeln!(
            file,
            "# {}, Phase: {}, Height: {:.1}mm, Created: {}",
            self.name, phase_str, height_mm, now
        )?;
        // Hinge parameters first so engineering tools that only display the
        // top of the file (e.g. Grasshopper Panel) see all of A–E + t1, t2.
        write_dimensions_comments(&mut file, &self.dimensions)?;
        writeln!(
            file,
            "# Orientation check (CSV coords, mm, Z-up): ground plane at Z=0, apex at Z={:.1}",
            highest_one.1.z
        )?;
        for (i, (path, pos)) in lowest_three.iter().enumerate() {
            writeln!(
                file,
                "# Lowest[{}]: joint={} X={:.1} Y={:.1} Z={:.1}",
                i + 1, path, pos.x, pos.y, pos.z
            )?;
        }
        writeln!(
            file,
            "# Highest:   joint={} X={:.1} Y={:.1} Z={:.1}",
            highest_one.0, highest_one.1.x, highest_one.1.y, highest_one.1.z
        )?;
        let bend_summary = build_bend_summary(self);
        file.write_all(bend_summary.as_bytes())?;
        writeln!(file, "Index,Role,Length(m),Strain,AlphaX,AlphaY,AlphaZ,AlphaJoint,AlphaSlot,AlphaAngle,OmegaX,OmegaY,OmegaZ,OmegaJoint,OmegaSlot,OmegaAngle")?;

        // Build a map of pull interval connections for each push interval
        // Key: (pull_interval_key, end, slot) -> (pull_end_pos, hinge_pos,
        // joint_key, slot, hinge_bend, ideal_deg)
        let mut pull_hinge_info: std::collections::HashMap<
            (IntervalKey, IntervalEnd, usize),
            (Vec3, Vec3, JointKey, usize, HingeBend, f32),
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

                            let (hinge_pos, hinge_bend, pull_end_pos, ideal_deg) =
                                dimensions.hinge_geometry(
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
                                    ideal_deg,
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

                            let (hinge_pos, hinge_bend, pull_end_pos, ideal_deg) =
                                dimensions.hinge_geometry(
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
                                    ideal_deg,
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
                let length = (omega - alpha).length();
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
        for ((_, _, slot), (_, _, joint_key, _, _, _)) in &pull_hinge_info {
            let entry = highest_slot_per_joint.entry(*joint_key).or_insert(0);
            if *slot > *entry {
                *entry = *slot;
            }
        }

        // Build a map of ring centers for pull-fea (joint_key, slot) -> ring_center
        let mut ring_centers: std::collections::HashMap<(JointKey, usize), Vec3> =
            std::collections::HashMap::new();

        for (_key, push_interval) in self.intervals.iter() {
            if !push_interval.has_role(Role::Pushing) {
                continue;
            }
            let alpha_pos = self.joints[push_interval.alpha_key].location;
            let omega_pos = self.joints[push_interval.omega_key].location;
            let push_dir = (omega_pos - alpha_pos).normalize();

            // For each end, calculate ring centers at all slots (0-indexed internally, 1-indexed in hashmap keys)
            for slot_0 in 0..3 {
                let slot_1 = slot_0 + 1;
                // Alpha end
                let alpha_ring = dimensions.ring_center(alpha_pos, -push_dir, slot_0);
                ring_centers.insert((push_interval.alpha_key, slot_1), alpha_ring);

                // Omega end
                let omega_ring = dimensions.ring_center(omega_pos, push_dir, slot_0);
                ring_centers.insert((push_interval.omega_key, slot_1), omega_ring);
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
                let alpha = to_csv * (alpha_joint.location * MM_PER_METER);
                let omega = to_csv * (omega_joint.location * MM_PER_METER);
                writeln!(
                    file,
                    "{},{},{:.3},{:.3e},{:.3},{:.3},{:.3},{},0,90,{:.3},{:.3},{:.3},{},0,90",
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
                    if let Some((pull_end_pos, _, joint_key, slot, bend, _)) = alpha_info {
                        (
                            to_csv * (*pull_end_pos * MM_PER_METER),
                            self.joints[*joint_key].path.to_string(),
                            *slot,
                            Some(*bend),
                        )
                    } else {
                        let joint = &self.joints[interval.alpha_key];
                        (
                            to_csv * (joint.location * MM_PER_METER),
                            joint.path.to_string(),
                            0,
                            None,
                        )
                    };

                let (omega_pos, omega_joint_path, omega_slot, omega_bend) =
                    if let Some((pull_end_pos, _, joint_key, slot, bend, _)) = omega_info {
                        (
                            to_csv * (*pull_end_pos * MM_PER_METER),
                            self.joints[*joint_key].path.to_string(),
                            *slot,
                            Some(*bend),
                        )
                    } else {
                        let joint = &self.joints[interval.omega_key];
                        (
                            to_csv * (joint.location * MM_PER_METER),
                            joint.path.to_string(),
                            0,
                            None,
                        )
                    };

                // Calculate shortened length
                let shortened_length = (omega_pos - alpha_pos).length() / MM_PER_METER;

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
            alpha_pos: Vec3,
            omega_pos: Vec3,
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

            let fea_length = (omega_fea - alpha_fea).length();

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
        // All pull-fea endpoints connect to the HIGHEST ring center at each joint
        // (same point as push-fea endpoints) so all FEA elements meet at the same node
        for info in interval_infos.iter().filter(|i| !i.is_push) {
            let interval = self.intervals.get(info.key).unwrap();

            // Find connection info for each end (to get the joint_key)
            let alpha_info = pull_hinge_info
                .iter()
                .find(|((pull_id, end, _), _)| *pull_id == info.key && *end == IntervalEnd::Alpha)
                .map(|((_, _, _slot), (_, _, joint_key, _, _, _))| *joint_key);
            let omega_info = pull_hinge_info
                .iter()
                .find(|((pull_id, end, _), _)| *pull_id == info.key && *end == IntervalEnd::Omega)
                .map(|((_, _, _slot), (_, _, joint_key, _, _, _))| *joint_key);

            // Get ring centers at HIGHEST slot (same as push-fea endpoints)
            let (alpha_fea, alpha_joint_path, alpha_slot) = if let Some(joint_key) = alpha_info {
                let highest_slot = highest_slot_per_joint.get(&joint_key).copied().unwrap_or(0);
                let ring = if highest_slot > 0 {
                    ring_centers
                        .get(&(joint_key, highest_slot))
                        .copied()
                        .unwrap_or(self.joints[joint_key].location)
                } else {
                    self.joints[joint_key].location
                };
                (ring, self.joints[joint_key].path.to_string(), highest_slot)
            } else {
                let joint = &self.joints[interval.alpha_key];
                (joint.location, joint.path.to_string(), 0)
            };

            let (omega_fea, omega_joint_path, omega_slot) = if let Some(joint_key) = omega_info {
                let highest_slot = highest_slot_per_joint.get(&joint_key).copied().unwrap_or(0);
                let ring = if highest_slot > 0 {
                    ring_centers
                        .get(&(joint_key, highest_slot))
                        .copied()
                        .unwrap_or(self.joints[joint_key].location)
                } else {
                    self.joints[joint_key].location
                };
                (ring, self.joints[joint_key].path.to_string(), highest_slot)
            } else {
                let joint = &self.joints[interval.omega_key];
                (joint.location, joint.path.to_string(), 0)
            };

            let fea_length = (omega_fea - alpha_fea).length();

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
            let alpha_mm = to_csv * (fea.alpha_pos * MM_PER_METER);
            let omega_mm = to_csv * (fea.omega_pos * MM_PER_METER);

            writeln!(
                file,
                "{},{},{:.3},{:.3e},{:.3},{:.3},{:.3},{},{},90,{:.3},{:.3},{:.3},{},{},90",
                current_index,
                role_str,
                fea.length,
                fea.strain,
                alpha_mm.x,
                alpha_mm.y,
                alpha_mm.z,
                fea.alpha_joint_path,
                fea.alpha_slot,
                omega_mm.x,
                omega_mm.y,
                omega_mm.z,
                fea.omega_joint_path,
                fea.omega_slot,
            )?;
        }

        // Build link structure for each push interval end
        // Group connections by push joint to build the axial chain
        // Each entry stores: (slot, pull_end_pos, hinge_pos)
        let mut push_end_connections: std::collections::HashMap<
            JointKey,
            Vec<(usize, Vec3, Vec3)>,
        > = std::collections::HashMap::new();

        for (_, (pull_end_pos, hinge_pos, joint_key, slot, _, _)) in &pull_hinge_info {
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
                .unwrap_or(Vec3::Y);

            let mut prev_pos = joint_pos;
            let mut prev_slot = 0usize;

            let joint_path = &joint.path;
            for (slot, pull_end_pos, hinge_pos) in &connections {
                // slot is 1-indexed here (from pull_hinge_info), convert to 0-indexed for ring_center
                let ring_center = dimensions.ring_center(joint_pos, push_axis, *slot - 1);

                // Axial link: previous position → ring center
                link_index += 1;
                let prev_mm = to_csv * (prev_pos * MM_PER_METER);
                let ring_mm = to_csv * (ring_center * MM_PER_METER);
                let axial_length = (ring_center - prev_pos).length();
                writeln!(
                    file,
                    "{},axial,{:.3},0.000e0,{:.3},{:.3},{:.3},{},{},90,{:.3},{:.3},{:.3},{},{},90",
                    link_index, axial_length,
                    prev_mm.x, prev_mm.y, prev_mm.z, joint_path, prev_slot,
                    ring_mm.x, ring_mm.y, ring_mm.z, joint_path, slot,
                )?;

                // Radial link: ring center → hinge
                link_index += 1;
                let hinge_mm = to_csv * (*hinge_pos * MM_PER_METER);
                let radial_length = (*hinge_pos - ring_center).length();
                writeln!(
                    file,
                    "{},radial,{:.3},0.000e0,{:.3},{:.3},{:.3},{},{},0.000,{:.3},{:.3},{:.3},{},{},0.000",
                    link_index, radial_length,
                    ring_mm.x, ring_mm.y, ring_mm.z, joint_path, slot,
                    hinge_mm.x, hinge_mm.y, hinge_mm.z, joint_path, slot,
                )?;

                // Hinge link: hinge → pull_end (along pull direction)
                link_index += 1;
                let pull_end_mm = to_csv * (*pull_end_pos * MM_PER_METER);
                let hinge_link_length = (*pull_end_pos - *hinge_pos).length();
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

fn format_unsigned_angle(deg: f32) -> String {
    if (deg - deg.round()).abs() < 0.05 {
        format!("{}°", deg.round() as i32)
    } else {
        format!("{:.1}°", deg)
    }
}

fn format_signed_angle(deg: f32) -> String {
    if deg.abs() < 0.05 {
        "0°".to_string()
    } else if (deg - deg.round()).abs() < 0.05 {
        format!("{:+}°", deg.round() as i32)
    } else {
        format!("{:+.1}°", deg)
    }
}

fn build_bend_summary(fabric: &Fabric) -> String {
    use std::fmt::Write;
    use crate::fabric::bend_optimizer::snap_to_magnitudes;

    let mut s = String::new();
    let h = &fabric.dimensions.hinge;
    let ideals = fabric.collect_ideal_bend_angles();

    writeln!(s, "# === Hinge bend snap quality ===").ok();
    writeln!(s, "# Bend count (K):       {}", h.bend_count).ok();
    if h.bend_magnitudes_locked {
        writeln!(s, "# Magnitude source:     LOCKED to factory inventory (no per-export reoptimisation)").ok();
    } else {
        writeln!(s, "# Magnitude source:     k-center optimiser, recomputed per export").ok();
    }

    if h.bend_count == 0 {
        writeln!(s, "# Snapping disabled (K = 0); CSV uses continuous ideal angles.").ok();
        writeln!(s, "# Cable ends measured: {}", ideals.len()).ok();
        writeln!(s, "#").ok();
        return s;
    }
    if ideals.is_empty() {
        writeln!(s, "# Cable ends measured: 0  (no pull connections)").ok();
        writeln!(s, "#").ok();
        return s;
    }

    let mags = &h.bend_magnitudes;
    let mag_str: Vec<String> = mags.iter().map(|m| format_unsigned_angle(*m)).collect();
    writeln!(s, "# Optimal magnitudes:   [{}]", mag_str.join(", ")).ok();

    let mut signed: Vec<f32> = Vec::with_capacity(mags.len() * 2);
    for &m in mags.iter().rev() {
        if m != 0.0 {
            signed.push(-m);
        }
    }
    for &m in mags {
        signed.push(m);
    }
    let signed_str: Vec<String> = signed.iter().map(|m| format_signed_angle(*m)).collect();
    writeln!(s, "# Effective signed set: [{}]", signed_str.join(", ")).ok();

    let n = ideals.len();
    let snapped: Vec<(f32, f32)> = ideals
        .iter()
        .map(|&x| snap_to_magnitudes(x, mags))
        .collect();

    let signed_counts: Vec<usize> = signed
        .iter()
        .map(|&candidate| {
            snapped
                .iter()
                .filter(|(snap, _)| (snap - candidate).abs() < 0.5)
                .count()
        })
        .collect();
    let mag_counts: Vec<usize> = mags
        .iter()
        .map(|&m| {
            snapped
                .iter()
                .filter(|(snap, _)| (snap.abs() - m).abs() < 0.5)
                .count()
        })
        .collect();

    let counts_line = signed_counts
        .iter()
        .zip(signed.iter())
        .map(|(c, m)| format!("{}×{}", format_signed_angle(*m), c))
        .collect::<Vec<_>>()
        .join("  ");
    writeln!(s, "# Bend counts (signed): {}", counts_line).ok();

    let mag_counts_line = mag_counts
        .iter()
        .zip(mags.iter())
        .map(|(c, m)| format!("{}×{}", format_unsigned_angle(*m), c))
        .collect::<Vec<_>>()
        .join("  ");
    writeln!(s, "# Bend counts (per magnitude): {}", mag_counts_line).ok();

    let errors: Vec<f32> = snapped.iter().map(|(_, e)| *e).collect();
    let mean = errors.iter().sum::<f32>() / n as f32;
    let max = errors.iter().fold(0.0_f32, |a, &b| a.max(b));
    let rms = (errors.iter().map(|e| e * e).sum::<f32>() / n as f32).sqrt();
    writeln!(s, "# Cable ends measured:  {}", n).ok();
    writeln!(
        s,
        "# Snap error:           mean |Δ|={:.2}°  max |Δ|={:.2}°  RMS={:.2}°",
        mean, max, rms
    )
    .ok();
    writeln!(s, "#").ok();
    s
}

fn write_dimensions_comments(file: &mut File, dims: &FabricDimensions) -> io::Result<()> {
    let h = &dims.hinge;
    let mm = |m: f32| m * 1000.0;
    let a = h.push_radius.f32();
    let b = h.push_radius_margin.f32();
    let t1 = h.disc_thickness.f32();
    let t2 = h.disc_separator_thickness.f32();
    let c = t1 / 2.0;
    let d = h.hinge_extension.f32();
    let e = h.hinge_hole_diameter.f32();
    let cap = h.cap_thickness.f32();
    writeln!(file, "#")?;
    writeln!(file, "# === Hinge parameters (see diagram) ===")?;
    writeln!(file, "# A  radius van de buis (push_radius):        {:.1}mm  ({:.5}m)", mm(a), a)?;
    writeln!(file, "# B  marge (push_radius_margin):              {:.1}mm  ({:.5}m)", mm(b), b)?;
    writeln!(file, "# C  offset door radius (= t1/2):             {:.1}mm  ({:.5}m)", mm(c), c)?;
    writeln!(file, "# D  randafstand (hinge_extension):           {:.1}mm  ({:.5}m)", mm(d), d)?;
    writeln!(file, "# E  diameter gat (hinge_hole_diameter):      {:.1}mm  ({:.5}m)", mm(e), e)?;
    writeln!(file, "# t1 dikte staal (disc_thickness):             {:.1}mm  ({:.5}m)", mm(t1), t1)?;
    writeln!(file, "# t2 dikte POM (disc_separator):              {:.1}mm  ({:.5}m)", mm(t2), t2)?;
    writeln!(file, "#    cap_thickness:                           {:.1}mm  ({:.5}m)", mm(cap), cap)?;
    writeln!(file, "#    pull_radius:                             {:.1}mm  ({:.5}m)", mm(dims.pull_radius.f32()), dims.pull_radius.f32())?;
    writeln!(file, "#")?;
    writeln!(file, "# === Afgeleide waarden ===")?;
    writeln!(file, "#    A + B + C  = {:.1}mm  (halve breedte schijf)", mm(a + b + c))?;
    writeln!(file, "#    C + D + E  = {:.1}mm  (scharnier lengte)", mm(c + d + e))?;
    writeln!(file, "#    t1 + t2    = {:.1}mm  (schijf + separator)", mm(t1 + t2))?;
    writeln!(
        file,
        "#    disc_center_offset(0) = {:.1}mm  (as-afstand tot centrum eerste schijf)",
        mm(h.disc_center_offset(0).f32()),
    )?;
    writeln!(file, "#")?;
    Ok(())
}
