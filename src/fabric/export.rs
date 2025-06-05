use cgmath::Point3;
use itertools::Itertools;
use std::io;
use std::io::{Cursor, Write};
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

use crate::fabric::interval::Interval;
use crate::fabric::joint::Joint;
use crate::fabric::material::Material;
use crate::fabric::{Fabric, UniqueId};

impl Fabric {
    /// Generate a ZIP file containing three CSV files with fabric data
    pub fn to_zip(&self) -> io::Result<Vec<u8>> {
        // Create in-memory buffer for the ZIP file
        let buffer = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(buffer);

        let options: FileOptions<'_, ()> =
            FileOptions::default().compression_method(CompressionMethod::Deflated);

        // Create info file
        zip.start_file("info.txt", options)?;
        let height = self
            .joints
            .iter()
            .fold(0.0f32, |h, joint| h.max(joint.location.y))
            * self.scale;
        let scale = self.scale;
        writeln!(zip, "Height: {height:.1}")?;
        writeln!(zip, "Scale: {scale:.1}")?;

        // Create joints CSV
        zip.start_file("joints.csv", options)?;
        writeln!(zip, "Index;X;Y;Z")?;
        for (index, Joint { location, .. }) in self.joints.iter().enumerate() {
            let Point3 { x, y, z } = location * self.scale;
            writeln!(zip, "{};{x:.2};{y:.2};{z:.2}", index + 1)?;
        }

        // Create intervals CSV
        zip.start_file("intervals.csv", options)?;
        writeln!(zip, "Joints;Role;Length;Strain")?;
        for interval in self.interval_values() {
            let length = interval.length(&self.joints) * self.scale;
            let strain = interval.strain;
            let role = interval.material.properties().role;
            let Interval {
                alpha_index,
                omega_index,
                material,
                ..
            } = interval;
            if matches!(material, Material::North | Material::South) {
                continue;
            }
            writeln!(
                zip,
                "=\"{},{}\";{role:?};{length:.2};{strain:.10}",
                alpha_index + 1,
                omega_index + 1
            )?;
        }

        // Create submerged CSV
        zip.start_file("submerged.csv", options)?;
        let submerged = self
            .joints
            .iter()
            .enumerate()
            .filter(|(_, joint)| joint.location.y <= 0.0)
            .map(|(index, _)| (index + 1).to_string())
            .join(",");
        writeln!(zip, "Submerged")?;
        writeln!(zip, "=\"{}\"", submerged)?;

        // Finalize the ZIP file
        let buffer = zip.finish()?.into_inner();
        Ok(buffer)
    }

    /// Export pull intervals (tension members) with their connection points
    pub fn export_pulls(&self) -> String {
        let mut output = String::new();

        // Add CSV header
        output.push_str("Length,Connection\n");

        // Get all pull intervals (tension members)
        let pull_intervals_with_ids: Vec<_> = self.intervals
            .iter()
            .enumerate()
            .filter_map(|(idx, interval_opt)| {
                if let Some(interval) = interval_opt {
                    if !interval.is_push_interval() {
                        Some((UniqueId(idx), interval))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        for (pull_id, pull_interval) in &pull_intervals_with_ids {
            let length = pull_interval.ideal() * self.scale;

            let joint1_idx = pull_interval.alpha_index;
            let joint2_idx = pull_interval.omega_index;

            let mut hole1_idx_opt: Option<usize> = None;
            let mut hole2_idx_opt: Option<usize> = None;

            // Find hole for End 1 (connected to joint1_idx)
            for push_candidate in self.interval_values() {
                if !push_candidate.is_push_interval() { continue; }
                if let Some(conns) = &push_candidate.connections {
                    if push_candidate.alpha_index == joint1_idx {
                        for (h_idx, conn_opt) in conns.alpha.iter().enumerate() {
                            if let Some(conn) = conn_opt {
                                if conn.pull_interval_id == *pull_id {
                                    hole1_idx_opt = Some(h_idx);
                                    break;
                                }
                            }
                        }
                    }
                    if hole1_idx_opt.is_some() { break; }
                    if push_candidate.omega_index == joint1_idx {
                         for (h_idx, conn_opt) in conns.omega.iter().enumerate() {
                            if let Some(conn) = conn_opt {
                                if conn.pull_interval_id == *pull_id {
                                    hole1_idx_opt = Some(h_idx);
                                    break;
                                }
                            }
                        }
                    }
                    if hole1_idx_opt.is_some() { break; }
                }
            }

            // Find hole for End 2 (connected to joint2_idx)
            for push_candidate in self.interval_values() {
                if !push_candidate.is_push_interval() { continue; }
                 if let Some(conns) = &push_candidate.connections {
                    if push_candidate.alpha_index == joint2_idx {
                        for (h_idx, conn_opt) in conns.alpha.iter().enumerate() {
                            if let Some(conn) = conn_opt {
                                if conn.pull_interval_id == *pull_id {
                                    hole2_idx_opt = Some(h_idx);
                                    break;
                                }
                            }
                        }
                    }
                    if hole2_idx_opt.is_some() { break; }
                    if push_candidate.omega_index == joint2_idx {
                         for (h_idx, conn_opt) in conns.omega.iter().enumerate() {
                            if let Some(conn) = conn_opt {
                                if conn.pull_interval_id == *pull_id {
                                    hole2_idx_opt = Some(h_idx);
                                    break;
                                }
                            }
                        }
                    }
                    if hole2_idx_opt.is_some() { break; }
                }
            }

            let hole1_str = hole1_idx_opt.map_or_else(|| "?".to_string(), |h| h.to_string());
            let hole2_str = hole2_idx_opt.map_or_else(|| "?".to_string(), |h| h.to_string());

            // Format as "J10/H2 - J25/H0"
            let connection_str = format!("J{:?}/H{} - J{:?}/H{}", 
                joint1_idx, hole1_str, joint2_idx, hole2_str);

            output.push_str(&format!(
                "{:.2},{}\n",
                length,
                connection_str
            ));
        }

        output
    }
}
