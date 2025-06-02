use cgmath::Point3;
use itertools::Itertools;
use std::io;
use std::io::{Cursor, Write};
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

use crate::fabric::interval::Interval;
use crate::fabric::joint::Joint;
use crate::fabric::material::Material;
use crate::fabric::Fabric;

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

    /// Export attachment points in a format suitable for construction
    pub fn export_attachment_points(&self) -> String {
        let mut output = String::new();

        // Get all pull intervals (tension members)
        let pull_intervals: Vec<_> = self.interval_values()
            .enumerate()
            .filter_map(|(idx, interval)| {
                if !interval.is_push_interval() {
                    Some((crate::fabric::UniqueId(idx), interval))
                } else {
                    None
                }
            })
            .collect();

        // Process each pull interval as a single line
        for (pull_id, pull_interval) in &pull_intervals {
            let length_mm = pull_interval.ideal() * self.scale;
            
            // Find connections by searching through all push intervals
            let mut connections = Vec::new();
            
            for (_push_idx, push_interval) in self.interval_values().enumerate() {
                if !push_interval.is_push_interval() {
                    continue;
                }
                
                if let Some(push_connections) = &push_interval.connections {
                    // Check alpha end connections
                    for (hole_idx, conn_opt) in push_connections.alpha.iter().enumerate() {
                        if let Some(conn) = conn_opt {
                            if conn.pull_interval_id.0 == pull_id.0 {
                                connections.push((push_interval.alpha_index, hole_idx));
                            }
                        }
                    }
                    
                    // Check omega end connections
                    for (hole_idx, conn_opt) in push_connections.omega.iter().enumerate() {
                        if let Some(conn) = conn_opt {
                            if conn.pull_interval_id.0 == pull_id.0 {
                                connections.push((push_interval.omega_index, hole_idx));
                            }
                        }
                    }
                }
            }
            
            // Format as single line with exactly 2 connections
            if connections.len() >= 2 {
                let conn1 = connections[0];
                let conn2 = connections[1];
                output.push_str(&format!("C{} {:.1}mm J{}-H{} J{}-H{}\n", 
                    pull_id.0, length_mm, conn1.0, conn1.1, conn2.0, conn2.1));
            } else {
                // Fallback: use joint indices without hole numbers
                output.push_str(&format!("C{} {:.1}mm J{} J{}\n", 
                    pull_id.0, length_mm, pull_interval.alpha_index, pull_interval.omega_index));
            }
        }

        output
    }
}
