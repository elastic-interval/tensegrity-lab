use cgmath::{InnerSpace, Point3};
use std::io;
use std::io::{Cursor, Write};
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

use crate::fabric::interval::{Interval, Role};
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
        let scale_mm = self.scale.to_mm();
        let height_mm = self
            .joints
            .iter()
            .fold(0.0f32, |h, joint| h.max(joint.location.y))
            * scale_mm;
        let now = chrono::Local::now()
            .format("%Y-%m-%d %H-%M")
            .to_string();
        writeln!(zip, "Name: {}", self.name)?;
        writeln!(zip, "Created: {now}")?;
        writeln!(zip, "Height: {height_mm:.1} mm")?;
        writeln!(zip, "Scale: {scale_mm:.1} mm/unit")?;

        // Create joints CSV
        zip.start_file("joints.csv".to_string(), options)?;
        writeln!(zip, "Index;X;Y;Z")?;
        for (index, Joint { location, .. }) in self.joints.iter().enumerate() {
            let Point3 { x, y, z } = location * scale_mm;
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

    /// Export fabric to USDA (ASCII USD) format with optional camera
    /// This creates a single .usda file content with the fabric geometry and camera
    pub fn to_usda_with_camera(
        &self,
        camera_pos: Option<Point3<f32>>,
        camera_target: Option<Point3<f32>>,
    ) -> io::Result<String> {
        let mut output = String::new();

        // USD header - use meters as base unit for better Blender compatibility
        output.push_str("#usda 1.0\n");
        output.push_str("(\n");
        output.push_str("    defaultPrim = \"Fabric\"\n");
        output.push_str("    metersPerUnit = 1.0\n");
        output.push_str("    upAxis = \"Y\"\n");
        output.push_str(")\n\n");

        // Scale is now in Meters, use directly
        let export_scale = *self.scale;

        // Export camera if provided
        if let (Some(pos), Some(target)) = (camera_pos, camera_target) {
            output.push_str("def Camera \"Camera\"\n");
            output.push_str("{\n");
            output.push_str(&format!(
                "    double3 xformOp:translate = ({:.6}, {:.6}, {:.6})\n",
                pos.x, pos.y, pos.z
            ));

            // Calculate look-at rotation
            let forward = (target - pos).normalize();
            let up = cgmath::Vector3::unit_y();
            let right = forward.cross(up).normalize();
            let _true_up = right.cross(forward);

            // Calculate rotation angles from direction vector
            let yaw = forward.z.atan2(forward.x).to_degrees();
            let pitch = (-forward.y).asin().to_degrees();

            output.push_str(&format!(
                "    double3 xformOp:rotateXYZ = ({:.6}, {:.6}, 0.0)\n",
                pitch, yaw
            ));
            output.push_str(
                "    uniform token[] xformOpOrder = [\"xformOp:translate\", \"xformOp:rotateXYZ\"]\n",
            );
            output.push_str("    float focalLength = 50.0\n");
            output.push_str("    float horizontalAperture = 36.0\n");
            output.push_str("    float verticalAperture = 24.0\n");
            output.push_str("}\n\n");
        }

        // Root transform
        output.push_str("def Xform \"Fabric\" (\n");
        output.push_str("    kind = \"component\"\n");
        output.push_str(")\n");
        output.push_str("{\n");

        // Export joints as spheres (radius in meters)
        let joint_radius = 0.01; // 1cm radius
        output.push_str("    def Scope \"Joints\"\n");
        output.push_str("    {\n");
        for (index, Joint { location, .. }) in self.joints.iter().enumerate() {
            let Point3 { x, y, z } = location * export_scale;
            output.push_str(&format!("        def Sphere \"Joint_{:04}\"\n", index));
            output.push_str("        {\n");
            output.push_str(&format!("            double radius = {:.6}\n", joint_radius));
            output.push_str(&format!(
                "            double3 xformOp:translate = ({:.6}, {:.6}, {:.6})\n",
                x, y, z
            ));
            output.push_str(
                "            uniform token[] xformOpOrder = [\"xformOp:translate\"]\n",
            );
            output.push_str("        }\n");
        }
        output.push_str("    }\n\n");

        // Export intervals as cylinders
        output.push_str("    def Scope \"Intervals\"\n");
        output.push_str("    {\n");

        // Group by role
        let mut push_intervals = Vec::new();
        let mut pull_intervals = Vec::new();
        let mut springy_intervals = Vec::new();

        for interval in self.interval_values() {
            // Skip support intervals
            if interval.has_role(Role::Support) {
                continue;
            }
            match interval.role {
                Role::Pushing => push_intervals.push(interval),
                Role::Springy => springy_intervals.push(interval),
                _ if interval.role.is_pull_like() => pull_intervals.push(interval),
                _ => {}
            }
        }

        // Export push intervals with unique prefix
        if !push_intervals.is_empty() {
            output.push_str("        def Scope \"Push\"\n");
            output.push_str("        {\n");
            for (idx, interval) in push_intervals.iter().enumerate() {
                self.write_interval_cylinder(&mut output, "Push", idx, interval, "            ", export_scale);
            }
            output.push_str("        }\n\n");
        }

        // Export pull intervals with unique prefix
        if !pull_intervals.is_empty() {
            output.push_str("        def Scope \"Pull\"\n");
            output.push_str("        {\n");
            for (idx, interval) in pull_intervals.iter().enumerate() {
                self.write_interval_cylinder(&mut output, "Pull", idx, interval, "            ", export_scale);
            }
            output.push_str("        }\n\n");
        }

        output.push_str("    }\n");
        output.push_str("}\n");

        Ok(output)
    }

    /// Export fabric to USDA (ASCII USD) format without camera
    pub fn to_usda(&self) -> io::Result<String> {
        self.to_usda_with_camera(None, None)
    }

    /// Helper to write a cylinder representing an interval
    /// Uses a unit cylinder with scale, rotate, translate transforms
    fn write_interval_cylinder(
        &self,
        output: &mut String,
        prefix: &str,
        idx: usize,
        interval: &Interval,
        indent: &str,
        export_scale: f32,
    ) {
        let alpha_loc = self.joints[interval.alpha_index].location * export_scale;
        let omega_loc = self.joints[interval.omega_index].location * export_scale;

        // Calculate midpoint (translation)
        let mid_x = (alpha_loc.x + omega_loc.x) / 2.0;
        let mid_y = (alpha_loc.y + omega_loc.y) / 2.0;
        let mid_z = (alpha_loc.z + omega_loc.z) / 2.0;

        // Calculate length and direction
        let dx = omega_loc.x - alpha_loc.x;
        let dy = omega_loc.y - alpha_loc.y;
        let dz = omega_loc.z - alpha_loc.z;
        let length = (dx * dx + dy * dy + dz * dz).sqrt();

        // Radius based on role (in meters - push ~1.2cm, pull ~5mm)
        // Note: Pull radius increased from 2mm to 5mm for better Blender compatibility
        let radius = match interval.role {
            Role::Pushing => 0.012,
            _ => 0.005,
        };

        // Build a 4x4 transformation matrix that:
        // 1. Scales: X and Z by radius, Y by length (unit cylinder is radius=1, height=1)
        // 2. Rotates: Y-axis to align with interval direction
        // 3. Translates: to midpoint
        //
        // Combined matrix = T * R * S where:
        // - S scales the unit cylinder
        // - R rotates Y-axis to direction
        // - T translates to midpoint

        let y_axis = cgmath::Vector3::new(dx / length, dy / length, dz / length);

        // Choose an arbitrary vector not parallel to y_axis to compute x_axis
        let arbitrary = if y_axis.y.abs() < 0.9 {
            cgmath::Vector3::new(0.0, 1.0, 0.0)
        } else {
            cgmath::Vector3::new(1.0, 0.0, 0.0)
        };

        let x_axis = y_axis.cross(arbitrary).normalize();
        let z_axis = x_axis.cross(y_axis).normalize();

        // USD default cylinder has height=2 (from -1 to +1 on axis), radius=1
        // We scale Y by length/2 so total height becomes length
        let c0 = x_axis * radius;
        let c1 = y_axis * (length / 2.0);  // Divide by 2 because default height is 2
        let c2 = z_axis * radius;

        // USD uses row-vector convention with translation in the LAST ROW:
        // row0 = (Xx, Xy, Xz, 0)  - X basis (scaled by radius)
        // row1 = (Yx, Yy, Yz, 0)  - Y basis (scaled by length/2, since default height=2)
        // row2 = (Zx, Zy, Zz, 0)  - Z basis (scaled by radius)
        // row3 = (Tx, Ty, Tz, 1)  - Translation
        output.push_str(&format!("{}def Cylinder \"{}_{:04}\"\n", indent, prefix, idx));
        output.push_str(&format!("{}{{\n", indent));
        // Use USD defaults (radius=1, height=2) - scaling handled in matrix
        output.push_str(&format!(
            "{}    matrix4d xformOp:transform = ( ({:.6}, {:.6}, {:.6}, 0), ({:.6}, {:.6}, {:.6}, 0), ({:.6}, {:.6}, {:.6}, 0), ({:.6}, {:.6}, {:.6}, 1) )\n",
            indent,
            c0.x, c0.y, c0.z,
            c1.x, c1.y, c1.z,
            c2.x, c2.y, c2.z,
            mid_x, mid_y, mid_z
        ));
        output.push_str(&format!(
            "{}    uniform token[] xformOpOrder = [\"xformOp:transform\"]\n",
            indent
        ));
        output.push_str(&format!("{}}}\n", indent));
    }
}
