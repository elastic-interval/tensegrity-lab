use cgmath::Point3;
use std::io;
use std::io::{Cursor, Write};
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

use crate::fabric::interval::Interval;
use crate::fabric::joint::Joint;
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
        zip.start_file("info.txt".to_string(), options)?;
        let height = self
            .joints
            .iter()
            .fold(0.0f32, |h, joint| h.max(joint.location.y))
            * self.scale;
        let scale = self.scale;
        let now = chrono::Local::now()
            .format("%Y-%m-%d %H-%M")
            .to_string();
        writeln!(zip, "Name: {}", self.name)?;
        writeln!(zip, "Created: {now}")?;
        writeln!(zip, "Height: {height:.1}")?;
        writeln!(zip, "Scale: {scale:.1}")?;

        // Create joints CSV
        zip.start_file("joints.csv".to_string(), options)?;
        writeln!(zip, "Index;X;Y;Z")?;
        for (index, Joint { location, .. }) in self.joints.iter().enumerate() {
            let Point3 { x, y, z } = location * self.scale;
            writeln!(zip, "{};{x:.2};{z:.2};{y:.2}", index + 1)?;
        }

        // Create intervals CSV
        zip.start_file("intervals.csv".to_string(), options)?;
        writeln!(zip, "Alpha;Omega;Role")?;
        for interval in self.interval_values() {
            let role = interval.role;
            let Interval {
                alpha_index,
                omega_index,
                ..
            } = interval;
            writeln!(
                zip,
                "{},{};{}",
                alpha_index + 1,
                omega_index + 1,
                role as u8
            )?;
        }

        // Finalize the ZIP file
        let buffer = zip.finish()?.into_inner();
        Ok(buffer)
    }
}
