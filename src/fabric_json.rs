//! A fabric as built, as JSON: what each strut, cable, rod, plate, hinge,
//! foot and guy line is and where it sits, for whatever reads it. It says
//! what is made, never how to analyse it: an FEA script decides that
//! (`scripts/rfem/build_rfem.py`). Format: `docs/fabric-json.md`.
//!
//! The moment is the standing figure: pretensioned, settled under gravity on
//! its feet and guy lines, with the connectors committed on it. Every member
//! has its length here, its rest length (unloaded, what it is made to) and
//! the force it carries. Metres, kilograms, newtons, Z up, the ground on Z = 0.

use crate::connector::fork::threaded_rod_length;
use crate::connector::strut::{tube_set, INNER_BORE_RADIUS, INNER_RADIUS, OUTER_BORE_RADIUS, OUTER_RADIUS};
use crate::connector::system::{foot_pin_axis, hinge_pin_axis, hinge_struts, is_hanger};
use crate::connector::ConnectorSystem;
use crate::fabric::dimensions::Hardware;
use crate::fabric::interval::{Interval, Role};
use crate::fabric::material::Material;
use crate::fabric::{Fabric, IntervalEnd, IntervalKey, JointKey};
use crate::connector::parts::{ALUMINIUM_DENSITY, STEEL_DENSITY};
use crate::units::{Grams, GramsPerMeter, Meters, Newtons, Pascals, SquareMeters, Unit};
use glam::{Mat3, Vec3};
use serde::Serialize;
use std::f32::consts::FRAC_PI_2;

/// Bumped whenever a field changes meaning or goes; readers check it.
pub const SCHEMA_VERSION: u32 = 2;

const ALUMINIUM_MODULUS: Pascals = Pascals(69e9);
const STEEL_MODULUS: Pascals = Pascals(210e9);
/// Steel's share of a 6 mm 7×19 rope's section works out from its stiffness
/// at this rope modulus.
const ROPE_MODULUS: Pascals = Pascals(100e9);
const CABLE_BREAKING_LOAD: Newtons = Newtons(25.5e3);
const M8_STRESS_AREA: SquareMeters = SquareMeters(36.6e-6);
/// An M8 rod's weight, at its nominal section.
const ROD_DENSITY: GramsPerMeter = GramsPerMeter(395.0);
/// The jaw terminal's share of a cable end's mass; the rest is its ring.
const TERMINAL: Grams = Grams(160.0);

#[derive(Serialize)]
pub struct FabricJson {
    schema_version: u32,
    fabric: String,
    created: String,
    moment: &'static str,
    frame: &'static str,
    /// Everything the simulation weighs, which the ground and anchors carry.
    total_mass: f64,
    materials: Vec<MaterialJson>,
    tubes: Vec<TubeJson>,
    tension_stock: Vec<TensionStock>,
    joints: Vec<JointJson>,
    struts: Vec<StrutJson>,
    tension: Vec<TensionJson>,
    plates: Vec<PlateJson>,
    hinges: Vec<HingeJson>,
    feet: Vec<FootJson>,
    guy_lines: Vec<GuyLineJson>,
}

#[derive(Serialize)]
struct MaterialJson {
    name: &'static str,
    description: &'static str,
    youngs_modulus: f32,
    density: f32,
}

#[derive(Serialize)]
struct TubeJson {
    name: &'static str,
    description: &'static str,
    material: &'static str,
    outside_diameter: f64,
    wall: f64,
}

/// What cables and fork rods are made of: their area carries the load.
#[derive(Serialize)]
struct TensionStock {
    name: &'static str,
    description: &'static str,
    material: &'static str,
    area: f64,
    mass_per_metre: f64,
    breaking_load: Option<f64>,
}

#[derive(Serialize)]
struct JointJson {
    name: String,
    position: [f64; 3],
}

#[derive(Serialize)]
struct StrutJson {
    name: String,
    /// Stack to stack, standing.
    length: f64,
    /// Stack to stack, unloaded: what the tubes are set to.
    rest_length: f64,
    /// Newtons, compression negative.
    force: f64,
    /// The three tubes.
    tube_mass: f64,
    ends: [StrutEnd; 2],
    tube_set: TubeSetJson,
    /// Inner, outer, inner, from the first end to the second.
    segments: [Segment; 3],
}

#[derive(Serialize)]
struct StrutEnd {
    joint: String,
    /// Where the tubes end and the stack of rings starts.
    stack: [f64; 3],
    /// Outward along the strut, the way the stack builds.
    axis: [f64; 3],
    /// Cap, bolt, nut and washers.
    hardware_mass: f64,
    /// What closes the stack: a lock nut, or a fitting to a pin.
    closed_by: &'static str,
}

#[derive(Serialize)]
struct TubeSetJson {
    outer_length: f64,
    inner_length: f64,
    /// Each end's pins, as the crew counts: inner hole, then the outer pair.
    pins: [String; 2],
    /// How much longer the tubes set this way are than `rest_length`.
    length_error: f64,
}

#[derive(Serialize)]
struct Segment {
    tube: &'static str,
    from: [f64; 3],
    to: [f64; 3],
}

#[derive(Serialize)]
struct TensionJson {
    name: String,
    /// `cable`, `hanger` (a fork rod hanging a plate) or `fork rod`.
    kind: &'static str,
    stock: &'static str,
    /// Standing.
    pin_to_pin: f64,
    /// Unloaded: what it is made to.
    rest_pin_to_pin: f64,
    /// Newtons; nil when slack.
    force: f64,
    /// For fork rods: the threaded rod to cut, from `rest_pin_to_pin`.
    rod_cut_length: Option<f64>,
    ends: [TensionEnd; 2],
}

#[derive(Serialize)]
#[serde(tag = "on", rename_all = "snake_case")]
enum TensionEnd {
    /// Pinned through a cross-tube on a ring (or custom disc) on a strut's stack.
    Stack {
        strut: String,
        joint: String,
        /// 1 nearest the strut's tubes.
        slot: usize,
        extra_washers: u8,
        /// The ring's centre, on the strut's axis.
        ring_center: [f64; 3],
        /// The cross-tube's pin, where the terminal or fork pivots.
        pivot: [f64; 3],
        /// Terminal or fork, ring and washer.
        hardware_mass: f64,
    },
    /// Pinned through a hole of a node plate.
    Plate {
        plate: String,
        hole: usize,
        position: [f64; 3],
        /// Terminal or fork.
        hardware_mass: f64,
    },
}

#[derive(Serialize)]
struct PlateJson {
    name: String,
    holes: [[f64; 3]; 3],
    normal: [f64; 3],
    thickness: f64,
    /// The plate alone.
    mass: f64,
}

#[derive(Serialize)]
struct HingeJson {
    pin: String,
    position: [f64; 3],
    pin_axis: [f64; 3],
    clevis_strut: String,
    eye_strut: String,
    note: &'static str,
    mass: f64,
}

#[derive(Serialize)]
struct FootJson {
    joint: String,
    strut: String,
    position: [f64; 3],
    pin_axis: [f64; 3],
    part: &'static str,
    note: &'static str,
}

#[derive(Serialize)]
struct GuyLineJson {
    name: String,
    strut: String,
    joint: String,
    /// Where it leaves the strut's stack.
    from: [f64; 3],
    anchor: String,
    anchor_position: [f64; 3],
    length: f64,
    rest_length: f64,
    force: f64,
    note: &'static str,
}

/// How far an interval is from its rest length, and what it pulls with.
struct Loading {
    /// Rest length over length here: scale a member's length by it for its rest length.
    rest_ratio: f32,
    force: Newtons,
}

fn loading(fabric: &Fabric, interval: &Interval, hardware: Hardware) -> Loading {
    let stiffness = match interval.material {
        Material::Push => hardware.strut_stiffness,
        _ => hardware.cable_stiffness,
    };
    Loading {
        rest_ratio: interval.ideal() / interval.length(&fabric.joints),
        force: stiffness * (interval.stiffness.as_factor() * interval.strain),
    }
}

/// To Z up: sim (x, y, z) → (x, −z, y).
fn z_up() -> Mat3 {
    Mat3::from_rotation_x(FRAC_PI_2)
}

/// To the micrometre, so the file doesn't carry f32's noise.
fn round(v: Vec3) -> [f64; 3] {
    v.to_array().map(fine)
}

fn fine(x: f32) -> f64 {
    (x as f64 * 1e6).round() / 1e6
}

fn metres(length: Meters) -> f64 {
    fine(length.f32())
}

fn kg(mass: Grams) -> f64 {
    fine(mass.to_kg())
}

fn newtons(force: Newtons) -> f64 {
    fine(force.f32())
}

fn strut_name(fabric: &Fabric, strut: IntervalKey) -> String {
    let interval = &fabric.intervals[strut];
    format!("{}-{}", fabric.joint_label(interval.alpha_key), fabric.joint_label(interval.omega_key))
}

/// The strut ending at `joint` that carries `cable`'s end there.
fn carrying_strut(fabric: &Fabric, cable: Option<IntervalKey>, joint: JointKey) -> (IntervalKey, IntervalEnd) {
    let strut = cable.and_then(|cable| fabric.hinge_strut.get(&(cable, joint)).copied()).unwrap_or_else(|| {
        fabric
            .intervals
            .iter()
            .find(|(_, interval)| interval.has_role(Role::Pushing) && interval.touches(joint))
            .map(|(key, _)| key)
            .unwrap_or_else(|| panic!("cable end at {} is on no strut", fabric.joint_label(joint)))
    });
    let end = if fabric.intervals[strut].alpha_key == joint { IntervalEnd::Alpha } else { IntervalEnd::Omega };
    (strut, end)
}

fn sorted_by_name(fabric: &Fabric, keys: impl Iterator<Item = IntervalKey>) -> Vec<IntervalKey> {
    let mut keys: Vec<IntervalKey> = keys.collect();
    keys.sort_by_key(|&key| {
        let interval = &fabric.intervals[key];
        (fabric.joint_label(interval.alpha_key), fabric.joint_label(interval.omega_key))
    });
    keys
}

/// `fabric` as built. Commit the connectors first
/// (`Fabric::update_all_attachment_connections`).
pub fn fabric_json(fabric: &Fabric) -> FabricJson {
    let connector = fabric.connector.as_ref().expect("fabric has connectors");
    let hardware = fabric.dimensions.hardware.unwrap_or(Hardware::OPEN_CLAW);
    let rotation = z_up();
    let (lowest_y, _) = fabric.altitude_range();
    let at = |location: Vec3| round(rotation * (location - Vec3::Y * lowest_y));
    let along = |direction: Vec3| round(rotation * direction);
    let joint_at = |joint: JointKey| fabric.joints[joint].location;

    let mut joints: Vec<JointJson> = fabric
        .joints
        .keys()
        .map(|joint| JointJson { name: fabric.joint_label(joint), position: at(joint_at(joint)) })
        .collect();
    joints.sort_by(|a, b| a.name.cmp(&b.name));

    let struts = sorted_by_name(fabric, fabric.intervals.iter().filter(|(_, i)| i.has_role(Role::Pushing)).map(|(k, _)| k));
    let struts: Vec<StrutJson> = struts.into_iter().map(|key| strut(fabric, connector, key, hardware, &at, &along)).collect();

    let cables = fabric.intervals.iter().filter(|(_, i)| is_tension(i));
    let tension: Vec<TensionJson> = sorted_by_name(fabric, cables.map(|(k, _)| k))
        .into_iter()
        .map(|key| tension(fabric, connector, key, hardware, &at))
        .collect();

    let mut nodes: Vec<JointKey> = fabric.bridle_nodes.keys().copied().collect();
    nodes.sort_by_key(|&node| fabric.joint_label(node));
    let plates: Vec<PlateJson> = nodes
        .into_iter()
        .map(|node| {
            let plate = connector.node_plate(node).expect("every bridle node has a plate");
            PlateJson {
                name: fabric.joint_label(node),
                holes: plate.holes.map(|(_, hole)| at(hole)),
                normal: along(plate.normal),
                thickness: metres(crate::connector::fork::PLATE_THICKNESS),
                // The simulation counts the plate less the rings its cables' ends are counted with.
                mass: kg(hardware.plate + (hardware.cable_end - TERMINAL) * 3.0),
            }
        })
        .collect();

    let mut hinges: Vec<HingeJson> = fabric
        .joints
        .keys()
        .filter_map(|pin| {
            let [clevis, eye] = hinge_struts(fabric, pin)?;
            Some(HingeJson {
                pin: fabric.joint_label(pin),
                position: at(joint_at(pin)),
                pin_axis: along(hinge_pin_axis(fabric, pin)),
                clevis_strut: strut_name(fabric, clevis),
                eye_strut: strut_name(fabric, eye),
                note: "clevis GN 751-12-24-M12 and eye GN 752-12-M12, screwed onto the struts' bolts; \
                       both struts spin freely on their bolts",
                mass: kg(hardware.hinge),
            })
        })
        .collect();
    hinges.sort_by(|a, b| a.pin.cmp(&b.pin));

    let mut feet: Vec<FootJson> = connector
        .feet
        .iter()
        .map(|&foot| {
            let (strut, _) = carrying_strut(fabric, None, foot);
            FootJson {
                joint: fabric.joint_label(foot),
                strut: strut_name(fabric, strut),
                position: at(joint_at(foot)),
                pin_axis: along(foot_pin_axis(fabric, foot, strut)),
                part: "INA GIKR12-PW rod end, 13° tilt",
                note: "what it pins to on the ground is not designed",
            }
        })
        .collect();
    feet.sort_by(|a, b| a.joint.cmp(&b.joint));

    let guys = sorted_by_name(fabric, fabric.intervals.iter().filter(|(_, i)| i.has_role(Role::GuyLine)).map(|(k, _)| k));
    let guy_lines = guys
        .into_iter()
        .enumerate()
        .map(|(i, key)| {
            let guy = &fabric.intervals[key];
            let on_strut = |joint| fabric.intervals.values().any(|i| i.has_role(Role::Pushing) && i.touches(joint));
            let (joint, anchor) =
                if on_strut(guy.alpha_key) { (guy.alpha_key, guy.omega_key) } else { (guy.omega_key, guy.alpha_key) };
            let (strut, end) = carrying_strut(fabric, None, joint);
            let [alpha, omega] = connector.stack_ends(fabric, strut);
            let from = if end == IntervalEnd::Alpha { alpha.0 } else { omega.0 };
            let length = Meters(from.distance(joint_at(anchor)));
            let load = loading(fabric, guy, hardware);
            GuyLineJson {
                name: format!("guy{}", i + 1),
                strut: strut_name(fabric, strut),
                joint: fabric.joint_label(joint),
                from: at(from),
                anchor: fabric.joint_label(anchor),
                anchor_position: at(joint_at(anchor)),
                length: metres(length),
                rest_length: metres(length * load.rest_ratio),
                force: newtons(load.force),
                note: "6 mm cable to a ground anchor, not designed",
            }
        })
        .collect();

    FabricJson {
        schema_version: SCHEMA_VERSION,
        fabric: fabric.name.clone(),
        created: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
        moment: "standing: pretensioned and settled under gravity, on its feet and guy lines",
        frame: "metres, kilograms and newtons, Z up, the ground on Z = 0",
        total_mass: kg(fabric.joints.values().fold(Grams(0.0), |total, joint| total + joint.accumulated_mass)),
        materials: vec![
            MaterialJson {
                name: "aluminium",
                description: "EN AW-6060 T66",
                youngs_modulus: ALUMINIUM_MODULUS.f32(),
                density: ALUMINIUM_DENSITY.to_kg_per_cubic_meter(),
            },
            MaterialJson {
                name: "rope",
                description: "6 mm 7x19 galvanized steel rope; modulus an ESTIMATE until a datasheet gives it",
                youngs_modulus: ROPE_MODULUS.f32(),
                density: STEEL_DENSITY.to_kg_per_cubic_meter(),
            },
            MaterialJson {
                name: "steel",
                description: "steel, threaded rod",
                youngs_modulus: STEEL_MODULUS.f32(),
                density: STEEL_DENSITY.to_kg_per_cubic_meter(),
            },
        ],
        tubes: vec![
            TubeJson {
                name: "outer",
                description: "outer tube 40x3",
                material: "aluminium",
                outside_diameter: metres(OUTER_RADIUS * 2.0),
                wall: metres(Meters(OUTER_RADIUS.f32() - OUTER_BORE_RADIUS.f32())),
            },
            TubeJson {
                name: "inner",
                description: "inner tube 33x3, telescoping out of the outer",
                material: "aluminium",
                outside_diameter: metres(INNER_RADIUS * 2.0),
                wall: metres(Meters(INNER_RADIUS.f32() - INNER_BORE_RADIUS.f32())),
            },
        ],
        tension_stock: vec![
            TensionStock {
                name: "cable",
                description: "6 mm 7x19 cable, jaw terminals swaged on",
                material: "rope",
                area: fine((hardware.cable_stiffness / ROPE_MODULUS).f32()),
                // The simulation's own cable weight.
                mass_per_metre: kg(Material::Pull.base_linear_density() * Meters(1.0)),
                breaking_load: Some(newtons(CABLE_BREAKING_LOAD)),
            },
            TensionStock {
                name: "rod",
                description: "M8 threaded rod between two rod forks; area is the thread's stress area",
                material: "steel",
                area: fine(M8_STRESS_AREA.f32()),
                mass_per_metre: kg(ROD_DENSITY * Meters(1.0)),
                breaking_load: None,
            },
        ],
        joints,
        struts,
        tension,
        plates,
        hinges,
        feet,
        guy_lines,
    }
}

/// A cable or fork rod: anything pulling but supports and guy lines.
fn is_tension(interval: &Interval) -> bool {
    !interval.has_role(Role::Pushing) && !interval.has_role(Role::Support) && !interval.has_role(Role::GuyLine)
}

fn strut(
    fabric: &Fabric,
    connector: &ConnectorSystem,
    key: IntervalKey,
    hardware: Hardware,
    at: &impl Fn(Vec3) -> [f64; 3],
    along: &impl Fn(Vec3) -> [f64; 3],
) -> StrutJson {
    let interval = &fabric.intervals[key];
    let [(alpha, alpha_axis), (omega, omega_axis)] = connector.stack_ends(fabric, key);
    let end = |joint: JointKey, stack: Vec3, axis: Vec3| StrutEnd {
        joint: fabric.joint_label(joint),
        stack: at(stack),
        axis: along(axis),
        hardware_mass: kg(hardware.strut_end),
        closed_by: if connector.feet.contains(&joint) {
            "foot rod end"
        } else {
            match hinge_struts(fabric, joint) {
                Some([clevis, _]) if clevis == key => "hinge clevis",
                Some(_) => "hinge eye",
                None => "washer and lock nut",
            }
        },
    };
    let length = Meters(alpha.distance(omega));
    let load = loading(fabric, interval, hardware);
    let rest_length = length * load.rest_ratio;
    let (set, setting, _) = tube_set(connector.dimensions.tube_sets, rest_length);
    let [alpha_out, omega_out] = setting.ends.map(|end| set.extension(end).f32());
    let direction = (omega - alpha).normalize();
    let (first, second) = (alpha + direction * alpha_out, omega - direction * omega_out);
    StrutJson {
        name: strut_name(fabric, key),
        length: metres(length),
        rest_length: metres(rest_length),
        force: newtons(load.force),
        tube_mass: kg(hardware.strut_density * interval.length(&fabric.joints)),
        ends: [end(interval.alpha_key, alpha, alpha_axis), end(interval.omega_key, omega, omega_axis)],
        tube_set: TubeSetJson {
            outer_length: metres(set.outer),
            inner_length: metres(set.inner),
            pins: setting.ends.map(|pins| pins.to_string()),
            length_error: metres(setting.error),
        },
        segments: [
            Segment { tube: "inner", from: at(alpha), to: at(first) },
            Segment { tube: "outer", from: at(first), to: at(second) },
            Segment { tube: "inner", from: at(second), to: at(omega) },
        ],
    }
}

fn tension(
    fabric: &Fabric,
    connector: &ConnectorSystem,
    key: IntervalKey,
    hardware: Hardware,
    at: &impl Fn(Vec3) -> [f64; 3],
) -> TensionJson {
    let cable = &fabric.intervals[key];
    let mut points = Vec::new();
    let mut names = Vec::new();
    let ends = [cable.alpha_key, cable.omega_key].map(|joint| {
        let label = fabric.joint_label(joint);
        if let Some(plate) = connector.node_plate(joint) {
            let hole = plate.holes.iter().position(|(hole_cable, _)| *hole_cable == key).expect("cable has a hole");
            points.push(plate.holes[hole].1);
            names.push(label.clone());
            return TensionEnd::Plate {
                plate: label,
                hole: hole + 1,
                position: at(plate.holes[hole].1),
                hardware_mass: kg(TERMINAL),
            };
        }
        let (strut, end) = carrying_strut(fabric, Some(key), joint);
        // At a hinge each strut has a stack of its own: `<pin>@<far joint>`.
        names.push(if fabric.hinge_strut.contains_key(&(key, joint)) {
            format!("{label}@{}", fabric.joint_label(fabric.intervals[strut].other_joint(joint)))
        } else {
            label.clone()
        });
        let [alpha, omega] = connector.stack_ends(fabric, strut);
        let (stack, axis) = if end == IntervalEnd::Alpha { alpha } else { omega };
        let connection = connector
            .connections(strut, end)
            .into_iter()
            .flatten()
            .flatten()
            .find(|connection| connection.pull_interval_key == key)
            .expect("cable end has a slot");
        let pivot = connector.pull_end_pivot(fabric, key, joint).expect("cable end has a pivot");
        points.push(pivot);
        TensionEnd::Stack {
            strut: strut_name(fabric, strut),
            joint: label,
            slot: connection.attachment_index + 1,
            extra_washers: connection.lift,
            ring_center: at(connector.connection_ring_center(stack, axis, connection)),
            pivot: at(pivot),
            hardware_mass: kg(hardware.cable_end),
        }
    });
    let pin_to_pin = Meters(points[0].distance(points[1]));
    let load = loading(fabric, cable, hardware);
    let rest_pin_to_pin = pin_to_pin * load.rest_ratio;
    let (kind, stock) = if is_hanger(fabric, key) {
        ("hanger", "rod")
    } else if connector.link_length(key).is_some() {
        ("fork rod", "rod")
    } else {
        ("cable", "cable")
    };
    TensionJson {
        name: format!("{}-{}", names[0], names[1]),
        kind,
        stock,
        pin_to_pin: metres(pin_to_pin),
        rest_pin_to_pin: metres(rest_pin_to_pin),
        force: newtons(load.force),
        rod_cut_length: (stock == "rod").then(|| metres(threaded_rod_length(rest_pin_to_pin))),
        ends,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::dsl::fabric_library::{self, FabricName};
    use crate::build::dsl::fabric_plan_executor::FabricPlanExecutor;
    use std::collections::HashSet;

    /// Headless Hug standing, its hardware committed.
    pub fn headless_hug_standing() -> FabricPlanExecutor {
        let mut executor = FabricPlanExecutor::new(fabric_library::get_fabric_plan(FabricName::HeadlessHug));
        while !executor.is_complete() {
            let _ = executor.iterate();
        }
        executor.fabric.update_all_attachment_connections();
        executor
    }

    /// Everything in the fabric is there once: every strut, every cable or
    /// rod with both ends placed, a plate per bridle node, eight hinges, two
    /// feet, four guy lines, and names unique. Struts push, everything else
    /// pulls, and the tubes are set to the struts' rest lengths.
    #[test]
    fn headless_hug_json() {
        let executor = headless_hug_standing();
        let fabric = &executor.fabric;
        let json = fabric_json(fabric);
        let count = |role: fn(&Interval) -> bool| fabric.intervals.values().filter(|i| role(i)).count();
        assert_eq!(json.struts.len(), count(|i| i.has_role(Role::Pushing)));
        assert_eq!(json.tension.len(), count(is_tension));
        assert_eq!(json.plates.len(), fabric.bridle_nodes.len());
        assert_eq!(json.hinges.len(), 8);
        assert_eq!(json.feet.len(), 2);
        assert_eq!(json.guy_lines.len(), 4);
        let names: HashSet<&String> = json.tension.iter().map(|t| &t.name).collect();
        assert_eq!(names.len(), json.tension.len(), "tension names unique");
        assert_eq!(json.tension.iter().filter(|t| t.kind == "hanger").count(), fabric.bridle_hangers.len());
        for strut in &json.struts {
            assert!(strut.force < 0.0 && strut.rest_length > strut.length, "{} pushes", strut.name);
            assert!(strut.tube_set.length_error.abs() <= 0.005, "{} fits its tubes", strut.name);
        }
        assert!(json.tension.iter().all(|t| t.force >= 0.0));
        assert!(json.guy_lines.iter().all(|g| g.force > 0.0 && g.anchor_position[2].abs() < 0.01));
        let text = serde_json::to_string(&json).expect("serialize");
        assert!(!text.to_lowercase().contains("rfem"));
    }

    /// Writes `HeadlessHug-<date>.json` to the working directory:
    ///
    /// ```
    /// cargo test --release --lib export_headless_hug_json -- --ignored
    /// ```
    #[test]
    #[ignore]
    fn export_headless_hug_json() {
        let executor = headless_hug_standing();
        let filename = format!("HeadlessHug-{}.json", chrono::Local::now().format("%Y-%m-%d"));
        let json = serde_json::to_string_pretty(&fabric_json(&executor.fabric)).expect("serialize");
        std::fs::write(&filename, json).expect("write JSON");
        println!("{filename}");
    }
}
