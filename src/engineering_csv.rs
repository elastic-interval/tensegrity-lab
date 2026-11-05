//! Engineering CSV: a fabric's slack-moment geometry for the engineer's FEA.
//!
//! Geometry only — joint coordinates, element connectivity and lengths, with
//! no connector output (slots, pivot pins, FEA or link rows): for fabrics
//! whose joint hardware isn't designed yet. Open Claw's connector CSV lives in
//! `open_claw_symmetry.rs`. The snapshot is taken at the end of Building,
//! after slackening and before pretensing (see `docs/csv-handoff.md`). Any
//! fabric can be exported; the fabric-specific parts (which plan, symmetry
//! rounding, notes) are left to the caller.

use crate::connector::fork::threaded_rod_length;
use glam::{Mat3, Vec3};
use std::f32::consts::FRAC_PI_2;
use std::fs::File;
use std::io::{self, Write};

use crate::build::dsl::settle_phase::GuyLine;
use crate::fabric::interval::{Role, Span};
use crate::connector::strut::tube_set;
use crate::connector::system::{hinge_struts, is_hanger};
use crate::connector::ConnectorSystem;
use crate::fabric::{Fabric, IntervalKey, JointKey};
use crate::units::{Meters, Unit, MM_PER_METER};

/// Rotation from simulation space (Y-up) to CSV space (Z-up, RFEM/Rhino).
///
/// A +90° rotation about X sends sim +Y → csv +Z (vertical extent lands in
/// the CSV's Z column) and sim +Z → csv −Y, keeping the transform a pure
/// right-handed rotation: sim (x, y, z) → csv (x, −z, y).
pub fn sim_to_csv() -> Mat3 {
    Mat3::from_rotation_x(FRAC_PI_2)
}

/// Write the slack-moment CSV for `fabric` to `filename`. Bridle hardware is
/// written as built, so call `Fabric::update_all_attachment_connections` first.
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

    let mut rows = interval_rows(fabric);
    rows.extend(plate_rows(fabric));
    rows.sort_by(|a, b| (b.role == "push").cmp(&(a.role == "push")).then(a.length.total_cmp(&b.length)));

    if !fabric.bridle_nodes.is_empty() {
        for note in HARDWARE_NOTES {
            writeln!(file, "# {note}")?;
        }
        writeln!(file, "#")?;
        let rods: Vec<&Row> = rows.iter().filter(|row| row.role == "rod").collect();
        if !rods.is_empty() {
            writeln!(file, "# === Threaded rod to cut ===")?;
            for row in rods {
                writeln!(
                    file,
                    "# {} to {}: {:.0} mm rod, {:.0} mm pin to pin",
                    row.alpha.1,
                    row.omega.1,
                    threaded_rod_length(row.length) * MM_PER_METER,
                    row.length * MM_PER_METER,
                )?;
            }
            writeln!(file, "#")?;
        }
    }

    if let Some(connector) = fabric.connector.as_ref() {
        write_strut_settings(&mut file, fabric, connector)?;
    }

    writeln!(
        file,
        "Index,Role,Length(m),Strain,AlphaX,AlphaY,AlphaZ,AlphaJoint,OmegaX,OmegaY,OmegaZ,OmegaJoint"
    )?;
    for (index, row) in rows.iter().enumerate() {
        let (alpha, omega) = (mm(row.alpha.0), mm(row.omega.0));
        writeln!(
            file,
            "{},{},{:.3},{:.3e},{:.3},{:.3},{:.3},{},{:.3},{:.3},{:.3},{}",
            index + 1,
            row.role,
            row.length,
            row.strain,
            alpha.x,
            alpha.y,
            alpha.z,
            row.alpha.1,
            omega.x,
            omega.y,
            omega.z,
            row.omega.1,
        )?;
    }

    println!("Exported {} to {}", fabric.name, filename);
    Ok(())
}

/// Each strut's tubes and pins, shortest first, for the build crew.
fn write_strut_settings(file: &mut File, fabric: &Fabric, connector: &ConnectorSystem) -> io::Result<()> {
    writeln!(file, "# === Strut tubes and pins ===")?;
    writeln!(file, "# Tubes run from stack to stack. Pins at each end as <inner hole>:<outer holes>, each tube's")?;
    writeln!(file, "# holes counted from 1 at that end: 12:3-6 is inner hole 12 in outer hole 3, inner hole 17 in outer hole 6")?;
    let mut struts: Vec<(f32, String)> = fabric
        .intervals
        .iter()
        .filter(|(_, interval)| interval.has_role(Role::Pushing))
        .map(|(key, interval)| {
            let [(alpha, _), (omega, _)] = connector.stack_ends(fabric, key);
            let length = alpha.distance(omega);
            let (set, setting, fits) = tube_set(connector.dimensions.tube_sets, Meters(length));
            let [alpha_pins, omega_pins] = setting.ends;
            let line = format!(
                "# {} to {}: {:.0} mm, outer {:.0} / inner {:.0}, pins {} at {}, {} at {}, {:+.0} mm{}",
                fabric.joint_label(interval.alpha_key),
                fabric.joint_label(interval.omega_key),
                length * MM_PER_METER,
                set.outer.f32() * MM_PER_METER,
                set.inner.f32() * MM_PER_METER,
                alpha_pins,
                fabric.joint_label(interval.alpha_key),
                omega_pins,
                fabric.joint_label(interval.omega_key),
                setting.error.f32() * MM_PER_METER,
                if fits { "" } else { "  NO TUBE SET FITS" },
            );
            (length, line)
        })
        .collect();
    struts.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (_, line) in struts {
        writeln!(file, "{line}")?;
    }
    writeln!(file, "#")
}

const HARDWARE_NOTES: [&str; 7] = [
    "=== Bridle hardware (rows run pin to pin as built, at real lengths) ===",
    "rod:    fork rod, hanging a node plate from a strut end's stack, or where a plate's cable is too short to make",
    "plate:  an edge of a stiff triangular node plate, hole to hole",
    "pull:   a cable; one ending at a plate's hole runs pin to pin too",
    "<node>.<n> is hole n of the plate at that node, e.g. L48b.1",
    "<pin>@<joint> is on the stack of the strut toward <joint> at hinge <pin>, 30 mm off its axis",
    "No cable or rod ends on a hinge pin: the pin carries only its two struts",
];

/// One row of the CSV: its ends as (location in sim space, name).
struct Row {
    role: &'static str,
    length: f32,
    strain: f32,
    alpha: (Vec3, String),
    omega: (Vec3, String),
}

/// A row per non-support interval. Struts and ordinary cables run joint to
/// joint. A member of a node plate or a cable at a hinge pin runs pin to pin,
/// as built, since the joints there are only the simulation's.
fn interval_rows(fabric: &Fabric) -> Vec<Row> {
    fabric
        .intervals
        .iter()
        .filter(|(_, interval)| !interval.has_role(Role::Support))
        .map(|(key, interval)| {
            let (alpha_key, omega_key) = (interval.alpha_key, interval.omega_key);
            let plate_or_hinge = |joint| fabric.bridle_nodes.contains_key(&joint) || hinge_struts(fabric, joint).is_some();
            let is_push = interval.has_role(Role::Pushing);
            if !is_push && (plate_or_hinge(alpha_key) || plate_or_hinge(omega_key)) {
                let (alpha, omega) = (built_end(fabric, key, alpha_key), built_end(fabric, key, omega_key));
                let link = fabric.connector.as_ref().is_some_and(|c| c.link_length(key).is_some());
                let role = if is_hanger(fabric, key) || link {
                    "rod"
                } else {
                    "pull"
                };
                return Row { role, length: alpha.0.distance(omega.0), strain: 0.0, alpha, omega };
            }
            let length = interval.length(&fabric.joints);
            // `interval.strain` is only refreshed by the next physics tick, so
            // right after `slacken` it still holds the pre-slack value. Pushes
            // already approach their pretensed length; strain is measured against
            // the slack length they start from.
            let rest_length = match interval.span {
                Span::Approaching { start_length, .. } => start_length.f32(),
                _ => interval.ideal().f32(),
            };
            let at = |joint: JointKey| (fabric.joints[joint].location, fabric.joint_label(joint));
            Row {
                role: if is_push { "push" } else { "pull" },
                length,
                strain: (length - rest_length) / rest_length,
                alpha: at(alpha_key),
                omega: at(omega_key),
            }
        })
        .collect()
}

/// Where a plate member or hinge cable really ends at `joint`: a plate's hole,
/// or the pivot on a stack.
fn built_end(fabric: &Fabric, cable: IntervalKey, joint: JointKey) -> (Vec3, String) {
    let label = fabric.joint_label(joint);
    let location = fabric.joints[joint].location;
    let Some(connector) = fabric.connector.as_ref() else {
        return (location, label);
    };
    if let Some(plate) = connector.node_plate(joint) {
        if let Some(hole) = plate.holes.iter().position(|(key, _)| *key == cable) {
            return (plate.holes[hole].1, format!("{label}.{}", hole + 1));
        }
    }
    let pivot = connector.pull_end_pivot(fabric, cable, joint).unwrap_or(location);
    match fabric.hinge_strut.get(&(cable, joint)) {
        Some(&strut) => (pivot, format!("{label}@{}", fabric.joint_label(fabric.intervals[strut].other_joint(joint)))),
        None => (pivot, label),
    }
}

/// Three rows per node plate, its edges hole to hole.
fn plate_rows(fabric: &Fabric) -> Vec<Row> {
    let Some(connector) = fabric.connector.as_ref() else {
        return Vec::new();
    };
    let mut nodes: Vec<JointKey> = fabric.bridle_nodes.keys().copied().collect();
    nodes.sort_by_key(|node| fabric.joint_label(*node));
    let mut rows = Vec::new();
    for node in nodes {
        let Some(plate) = connector.node_plate(node) else {
            continue;
        };
        let label = fabric.joint_label(node);
        let hole = |i: usize| (plate.holes[i].1, format!("{label}.{}", i + 1));
        for (a, b) in [(0, 1), (1, 2), (2, 0)] {
            let (alpha, omega) = (hole(a), hole(b));
            rows.push(Row { role: "plate", length: alpha.0.distance(omega.0), strain: 0.0, alpha, omega });
        }
    }
    rows
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
        let mut executor = build_to_slack(FabricName::HeadlessHug);
        executor.fabric.update_all_attachment_connections();
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
            .count()
            + 3 * fabric.bridle_nodes.len();
        assert_eq!(rows.len(), expected);
        // Nothing but struts ends on a hinge pin, and plates are three stiff edges.
        let hinge_pins: Vec<String> = fabric
            .joints
            .keys()
            .filter(|pin| hinge_struts(fabric, *pin).is_some())
            .map(|pin| fabric.joint_label(pin))
            .collect();
        assert_eq!(hinge_pins.len(), 8);
        for row in rows.iter().filter(|row| row[1] != "push") {
            assert!(!hinge_pins.iter().any(|pin| pin == row[7] || pin == row[11]), "row {} ends on a hinge", row[0]);
        }
        assert_eq!(rows.iter().filter(|row| row[1] == "plate").count(), 3 * fabric.bridle_nodes.len());
        let hanger = |row: &&Vec<&str>| (row[2].parse::<f32>().unwrap() - 0.082).abs() < 0.001;
        assert_eq!(rows.iter().filter(|row| row[1] == "rod").filter(hanger).count(), 16);
        let struts = fabric.intervals.values().filter(|interval| interval.has_role(Role::Pushing)).count();
        assert_eq!(text.lines().filter(|line| line.contains(", pins ")).count(), struts);
        assert!(!text.contains("NO TUBE SET FITS"));
        assert!(
            text.contains("# Guy line: joint=L14"),
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
        let mut executor = build_to_slack(FabricName::HeadlessHug);
        executor.fabric.update_all_attachment_connections();
        let date = chrono::Local::now().format("%Y-%m-%d");
        let filename = format!("HeadlessHug-{}.csv", date);
        write_csv(&executor.fabric, &filename, &headless_hug_notes()).expect("CSV write failed");
    }
}
