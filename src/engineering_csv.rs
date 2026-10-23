//! Engineering CSV: a fabric's slack-moment geometry for the engineer's FEA.
//!
//! Geometry only — joint coordinates, element connectivity and lengths, with
//! no connector output (slots, pivot pins, FEA or link rows): for fabrics
//! whose joint hardware isn't designed yet. Open Claw's connector CSV lives in
//! `open_claw_symmetry.rs`. The snapshot is taken at the end of Building,
//! after slackening and before pretensing (see `docs/csv-handoff.md`). Any
//! fabric can be exported; the fabric-specific parts (which plan, symmetry
//! rounding, notes) are left to the caller.

use glam::{Mat3, Vec3};
use std::f32::consts::FRAC_PI_2;
use std::fs::File;
use std::io::{self, Write};

use crate::build::dsl::settle_phase::GuyLine;
use crate::fabric::interval::{Role, Span};
use crate::fabric::{Fabric, IntervalKey};
use crate::units::{Unit, MM_PER_METER};

/// Rotation from simulation space (Y-up) to CSV space (Z-up, RFEM/Rhino).
///
/// A +90° rotation about X sends sim +Y → csv +Z (vertical extent lands in
/// the CSV's Z column) and sim +Z → csv −Y, keeping the transform a pure
/// right-handed rotation: sim (x, y, z) → csv (x, −z, y).
pub fn sim_to_csv() -> Mat3 {
    Mat3::from_rotation_x(FRAC_PI_2)
}

/// Write the slack-moment CSV for `fabric` to `filename`.
///
/// Every interval except supports gets a row: pushes first, then pulls, each
/// sorted by its joint-to-joint length. `notes` become header comment lines,
/// for things the rows can't carry such as guy lines.
pub fn write_csv(fabric: &Fabric, filename: &str, notes: &[String]) -> io::Result<()> {
    let mut file = File::create(filename)?;
    let to_csv = sim_to_csv();
    // At the slack moment the fabric may still hang at its build altitude;
    // drop it so the lowest joint rests on the ground plane.
    let (lowest_y, highest_y) = fabric.altitude_range();
    let mm = |location: Vec3| to_csv * ((location - Vec3::Y * lowest_y) * MM_PER_METER);

    let height_mm = (highest_y - lowest_y) * MM_PER_METER;
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
    writeln!(
        file,
        "# {}, Phase: slack, Height: {:.1}mm, Created: {}",
        fabric.name, height_mm, now
    )?;

    let dims = &fabric.dimensions;
    writeln!(file, "#")?;
    writeln!(file, "# === Dimensions ===")?;
    writeln!(file, "#         scale:       {:.3}m", dims.scale.f32())?;
    writeln!(
        file,
        "#         push_radius: {:.1}mm",
        dims.push_radius.f32() * MM_PER_METER
    )?;
    writeln!(
        file,
        "#         pull_radius: {:.1}mm",
        dims.pull_radius.f32() * MM_PER_METER
    )?;
    writeln!(file, "#")?;

    let mut joints: Vec<(String, Vec3)> = fabric
        .joints
        .iter()
        .map(|(key, joint)| (fabric.joint_label(key), mm(joint.location)))
        .collect();
    joints.sort_by(|a, b| a.1.z.total_cmp(&b.1.z));
    let (highest_label, highest) = joints.last().expect("fabric has at least one joint");
    writeln!(
        file,
        "# Orientation check (CSV coords, mm, Z-up): ground plane at Z=0, apex at Z={:.1}",
        highest.z
    )?;
    for (i, (label, pos)) in joints.iter().take(3).enumerate() {
        writeln!(
            file,
            "# Lowest[{}]: joint={} X={:.1} Y={:.1} Z={:.1}",
            i + 1,
            label,
            pos.x,
            pos.y,
            pos.z
        )?;
    }
    writeln!(
        file,
        "# Highest:   joint={} X={:.1} Y={:.1} Z={:.1}",
        highest_label, highest.x, highest.y, highest.z
    )?;
    writeln!(file, "#")?;
    for note in notes {
        writeln!(file, "# {note}")?;
    }
    if !notes.is_empty() {
        writeln!(file, "#")?;
    }

    let mut rows: Vec<(IntervalKey, bool, f32)> = fabric
        .intervals
        .iter()
        .filter(|(_, interval)| !interval.has_role(Role::Support))
        .map(|(key, interval)| {
            (
                key,
                interval.has_role(Role::Pushing),
                interval.length(&fabric.joints),
            )
        })
        .collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.2.total_cmp(&b.2)));

    writeln!(
        file,
        "Index,Role,Length(m),Strain,AlphaX,AlphaY,AlphaZ,AlphaJoint,OmegaX,OmegaY,OmegaZ,OmegaJoint"
    )?;
    for (index, (key, is_push, length)) in rows.iter().enumerate() {
        let interval = &fabric.intervals[*key];
        // `interval.strain` is only refreshed by the next physics tick, so
        // right after `slacken` it still holds the pre-slack value. Pushes
        // already approach their pretensed length; strain is measured against
        // the slack length they start from.
        let rest_length = match interval.span {
            Span::Approaching { start_length, .. } => start_length.f32(),
            _ => interval.ideal().f32(),
        };
        let strain = (length - rest_length) / rest_length;
        let alpha = mm(fabric.joints[interval.alpha_key].location);
        let omega = mm(fabric.joints[interval.omega_key].location);
        writeln!(
            file,
            "{},{},{:.3},{:.3e},{:.3},{:.3},{:.3},{},{:.3},{:.3},{:.3},{}",
            index + 1,
            if *is_push { "push" } else { "pull" },
            length,
            strain,
            alpha.x,
            alpha.y,
            alpha.z,
            fabric.joint_label(interval.alpha_key),
            omega.x,
            omega.y,
            omega.z,
            fabric.joint_label(interval.omega_key),
        )?;
    }

    println!("Exported {} to {}", fabric.name, filename);
    Ok(())
}

/// Header notes describing guy lines, which are only created on landing and
/// so never appear among the slack-moment rows.
pub fn guy_line_notes(guy_lines: &[GuyLine]) -> Vec<String> {
    if guy_lines.is_empty() {
        return Vec::new();
    }
    let mut notes = vec![
        "=== Guy lines (created on landing; not among the rows) ===".to_string(),
        "Each runs from a joint to a ground anchor offset horizontally from where".to_string(),
        "that joint lands, with a length that percentage of the joint-to-anchor distance."
            .to_string(),
    ];
    let to_csv = sim_to_csv();
    for guy_line in guy_lines {
        let (dx, dz) = guy_line.ground_offset;
        let offset = to_csv * (Vec3::new(dx.f32(), 0.0, dz.f32()) * MM_PER_METER);
        notes.push(format!(
            "Guy line: joint={} anchor offset X={:.1} Y={:.1} (mm), length {:.1}%",
            guy_line.joint, offset.x, offset.y, guy_line.length.0
        ));
    }
    notes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::dsl::fabric_library::{self, FabricName};
    use crate::build::dsl::fabric_plan_executor::{ExecutorStage, FabricPlanExecutor};

    /// Build a fabric through Building (so `slacken` has run) and stop —
    /// no pretensing ticks. This is the moment the CSV captures.
    fn build_to_slack(name: FabricName) -> FabricPlanExecutor {
        let plan = fabric_library::get_fabric_plan(name);
        let mut executor = FabricPlanExecutor::new(plan);
        while *executor.stage() == ExecutorStage::Building {
            let _ = executor.iterate();
        }
        executor
    }

    fn headless_hug_notes() -> Vec<String> {
        let plan = fabric_library::get_fabric_plan(FabricName::HeadlessHug);
        let guy_lines = plan
            .settle_phase
            .map(|settle| settle.guy_lines)
            .unwrap_or_default();
        guy_line_notes(&guy_lines)
    }

    /// One row per non-support interval, under the geometry-only header,
    /// with each push's length equal to its joint-to-joint distance.
    #[test]
    fn test_headless_hug_csv_rows() {
        let executor = build_to_slack(FabricName::HeadlessHug);
        let fabric = &executor.fabric;
        let path = std::env::temp_dir().join("tensegrity-lab-headless-hug-test.csv");
        let path = path.to_str().expect("temp path is UTF-8");
        write_csv(fabric, path, &headless_hug_notes()).expect("CSV write failed");
        let text = std::fs::read_to_string(path).expect("CSV read failed");
        std::fs::remove_file(path).ok();

        let mut lines = text.lines().skip_while(|line| line.starts_with('#'));
        assert_eq!(
            lines.next(),
            Some("Index,Role,Length(m),Strain,AlphaX,AlphaY,AlphaZ,AlphaJoint,OmegaX,OmegaY,OmegaZ,OmegaJoint")
        );
        let rows: Vec<Vec<&str>> = lines.map(|line| line.split(',').collect()).collect();
        let expected = fabric
            .intervals
            .values()
            .filter(|interval| !interval.has_role(Role::Support))
            .count();
        assert_eq!(rows.len(), expected);
        assert!(
            text.contains("# Guy line: joint=Z2"),
            "guy lines missing from the header"
        );
        for row in rows.iter().filter(|row| row[1] == "push") {
            let coord = |i: usize| row[i].parse::<f32>().unwrap();
            let alpha = Vec3::new(coord(4), coord(5), coord(6));
            let omega = Vec3::new(coord(8), coord(9), coord(10));
            let length_mm = row[2].parse::<f32>().unwrap() * MM_PER_METER;
            assert!(
                ((omega - alpha).length() - length_mm).abs() < 1.0,
                "push row {} length",
                row[0]
            );
        }
    }

    /// Writes `HeadlessHug-<date>.csv` to the working directory. Marked
    /// `#[ignore]` so it runs only on demand:
    ///
    /// ```
    /// cargo test --release --lib export_headless_hug_csv -- --ignored
    /// ```
    #[test]
    #[ignore]
    fn export_headless_hug_csv() {
        let executor = build_to_slack(FabricName::HeadlessHug);
        let date = chrono::Local::now().format("%Y-%m-%d");
        let filename = format!("HeadlessHug-{}.csv", date);
        write_csv(&executor.fabric, &filename, &headless_hug_notes()).expect("CSV write failed");
    }
}
