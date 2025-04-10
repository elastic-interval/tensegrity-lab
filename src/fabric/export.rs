use cgmath::Point3;
use itertools::Itertools;
use std::io;
use std::io::{Cursor, Write};
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

use crate::fabric::interval::Interval;
use crate::fabric::joint::Joint;
use crate::fabric::material::interval_material;
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
        for (
            index,
            Joint {
                location, ..
            },
        ) in self
            .joints
            .iter()
            .enumerate()
            .filter(|(_, Joint { fixed, .. })| !fixed)
        {
            let Point3 { x, y, z } = location * self.scale;
            writeln!(zip, "{};{x:.2};{y:.2};{z:.2}", index + 1)?;
        }

        // Create intervals CSV
        zip.start_file("intervals.csv", options)?;
        writeln!(zip, "Joints;Role;Length;Strain")?;
        for interval in self.interval_values() {
            let length = interval.length(&self.joints) * self.scale;
            let strain = interval.strain;
            let role = interval_material(interval.material).role;
            let Interval {
                alpha_index,
                omega_index,
                ..
            } = interval;
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
            .filter(
                |(
                    _,
                    Joint {
                        location: Point3 { y, .. },
                        ..
                    },
                )| *y <= 0.0,
            )
            .map(|(index, _)| (index + 1).to_string())
            .join(",");
        writeln!(zip, "Submerged")?;
        writeln!(zip, "=\"{}\"", submerged)?;

        // Finalize the ZIP file
        let buffer = zip.finish()?.into_inner();
        Ok(buffer)
    }
}
