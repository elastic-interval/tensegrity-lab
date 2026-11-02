//! The parts list: every distinct physical part the connector hardware is
//! made of, and where each copy of it sits in the structure. A part has one
//! shape, defined once in its own frame; the structure is nothing but
//! placements of those shapes. Rendering draws each part once, replicated;
//! counting placements gives the bill of materials. A newly discovered part
//! is a new `Part` variant with its mesh, and the compiler finds every place
//! that must learn about it.

use crate::connector::fork::{PLATE_SIDE, HANGER_LENGTH};
use crate::connector::system::{hinge_pin_axis, hinge_struts, is_hanger};
use crate::fabric::interval::Role;
use crate::fabric::{Fabric, IntervalEnd, IntervalKey};
use crate::units::Unit;
use glam::Vec3;
use std::cell::Cell;
use std::collections::BTreeMap;

/// A part's frame: `x` and `y` are orthogonal unit vectors, `z = x × y`.
/// Each variant says how its shape sits in that frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Part {
    /// Flat ring turning on the bolt, centred at the origin with `y` along
    /// the bolt; its boss reaches out along `x` toward the cable, ending in
    /// the cross-tube welded across it at the pivot.
    Ring,
    /// Laser-cut disc made to suit one joint, carrying several cables at one
    /// height: centred at the origin, `y` along the bolt, its bosses at the
    /// angles and reaches of its `DiscShape`, measured from `x`, each
    /// ending in its welded cross-tube. Each distinct shape is a part of its
    /// own.
    CustomDisc(DiscShape),
    /// POM washers under a ring, keeping steel off steel: `washers` thick,
    /// from what the ring stacks on (origin) up along `y`. One is the normal
    /// divider; more make a spacer, lifting the ring to clear the forks.
    Spacer { washers: u8 },
    /// Jaw terminal with pin and split ring: pin centre at the origin, barrel
    /// along `x`, pin along `y` (see `connector::fork::ClevisDimensions`).
    Clevis,
    /// A fork at each end of a fork rod: a hanger on a node plate or anything
    /// from a plate too short to be a cable (see `connector::fork`). Pin at the
    /// origin along `y`, the threaded rod through its web along `x`. The rod is
    /// stock cut to length, drawn and reported like a cable.
    RodFork,
    /// A rod fork's load nut and lock nut, in the fork's frame. Stock, so not
    /// on the parts list.
    ForkNuts,
    /// A hinge (see `connector::fork::HINGE_LENGTH`): the clevis on one
    /// strut with the pin, its centre at the origin along `y`, the clevis
    /// running down its strut along `x`. It carries only its two struts: no
    /// cable may end on it (`Fabric::split_hinge_cables`).
    Hinge,
    /// A hinge's rod end on the other strut, its eye round the pin at the
    /// origin along `y`, running down its strut along `x`. It turns on the
    /// pin, so it is placed apart, but it is part of its `Hinge` and not on
    /// the parts list.
    HingeEye,
    /// Triangular node plate at a bridle node (see `connector::node_plate`):
    /// centred at the origin, `y` square to the plate, its holes at the
    /// angles of its `PlateShape`, measured from `x`.
    NodePlate(PlateShape),
}

/// A node plate's three holes as (angle from `x` in degrees, distance from
/// the centre in millimetres), ascending by angle, the first at 0. Rounded,
/// so plates that match to the degree and millimetre are the same part.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlateShape {
    pub holes: [(i16, u16); 3],
}

impl PlateShape {
    /// Every plate: `PLATE_SIDE` on a side.
    pub fn equilateral() -> PlateShape {
        let mm = (PLATE_SIDE.f32() / 3f32.sqrt() * 1000.0).round() as u16;
        PlateShape { holes: [(0, mm), (120, mm), (240, mm)] }
    }
}

/// Most bosses one custom disc carries.
pub const MAX_BOSSES: usize = 4;

/// A custom disc's bosses as `(angle from x in degrees, reach from the axis
/// to the pivot in millimetres)`, ascending by angle; the first `count`
/// are used. Rounded, so discs that match to the degree and millimetre are
/// the same part.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DiscShape {
    pub bosses: [(i16, u16); MAX_BOSSES],
    pub count: u8,
}

impl DiscShape {
    pub fn bosses(&self) -> &[(i16, u16)] {
        &self.bosses[..self.count as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Material {
    /// Fabricated steel, hot-dip galvanized.
    Galvanized,
    /// Bought AISI-316 stainless.
    Stainless,
    /// POM washers between the rings.
    Pom,
}

impl Part {
    pub fn name(self) -> String {
        match self {
            Part::CustomDisc(shape) => {
                let bosses: Vec<String> =
                    shape.bosses().iter().map(|(degrees, reach)| format!("{degrees}°/{reach}mm")).collect();
                format!("custom disc {}", bosses.join(" "))
            }
            Part::Spacer { washers: 1 } => "POM washer".to_string(),
            Part::Spacer { washers } => format!("spacer of {washers} POM washers"),
            Part::NodePlate(shape) => {
                let holes: Vec<String> = shape.holes.iter().map(|(degrees, mm)| format!("{degrees}°/{mm}mm")).collect();
                format!("node plate {}", holes.join(" "))
            }
            part => part.base_name().to_string(),
        }
    }

    fn base_name(self) -> &'static str {
        match self {
            Part::Hinge => "hinge (clevis, rod end, pin)",
            Part::HingeEye => "hinge's rod end",
            Part::Ring => "ring with boss and cross-tube",
            Part::CustomDisc(_) => "custom disc",
            Part::Spacer { .. } => "POM washer",
            Part::Clevis => "jaw terminal",
            Part::RodFork => "rod fork",
            Part::ForkNuts => "rod fork's nuts",
            Part::NodePlate(_) => "node plate",
        }
    }

    /// Whether this is a part of its own on the parts list, not a half of
    /// a hinge placed apart because it turns, or a fork's stock nuts.
    pub fn listed(self) -> bool {
        !matches!(self, Part::ForkNuts | Part::HingeEye)
    }

    pub fn material(self) -> Material {
        match self {
            Part::Ring | Part::CustomDisc(_) | Part::NodePlate(_) | Part::RodFork => {
                Material::Galvanized
            }
            Part::Clevis
            | Part::ForkNuts
            | Part::Hinge
            | Part::HingeEye => Material::Stainless,
            Part::Spacer { .. } => Material::Pom,
        }
    }
}

/// One copy of a part, in its frame (see `Part`). `culprit` marks a copy
/// belonging to an assembly that collides, or a clamp bending past its
/// saddle — a construction problem to resolve.
#[derive(Clone, Copy, Debug)]
pub struct Placement {
    pub part: Part,
    pub origin: Vec3,
    pub x: Vec3,
    pub y: Vec3,
    pub culprit: bool,
}

/// Every part of the fabric's connector hardware, placed. Empty without a
/// connector system.
pub fn placements(fabric: &Fabric) -> Vec<Placement> {
    let mut placed = Vec::new();
    let Some(connector) = fabric.connector.as_ref() else {
        return placed;
    };
    // Placements come out in the same order every time, so the part at this
    // index when the connectors were rebuilt is this placement's part. Shaped
    // parts keep that shape while the joints move: their rounded angles must
    // not flicker as the physics jitters.
    let index = Cell::new(0);
    let frozen = |index: usize| connector.frozen_parts.get(index).copied();
    let mut place = |part: Part, origin: Vec3, x: Vec3, y: Vec3, culprit: bool| {
        placed.push(Placement { part, origin, x, y, culprit });
        index.set(index.get() + 1);
    };

    for (key, push) in fabric.intervals.iter() {
        if !push.has_role(Role::Pushing) {
            continue;
        }
        let [(alpha_end, alpha_axis), (omega_end, omega_axis)] = connector.stack_ends(fabric, key);
        for (end, end_pos, axis, joint) in [
            (IntervalEnd::Alpha, alpha_end, alpha_axis, push.alpha_key),
            (IntervalEnd::Omega, omega_end, omega_axis, push.omega_key),
        ] {
            let Some(connections) = connector.connections(key, end) else {
                continue;
            };
            // A ring or custom disc per height: centre, each cable's radial
            // and custom reach, culprit.
            let mut heights: BTreeMap<usize, (Vec3, Vec<(Vec3, Option<f32>)>, bool, u8)> = BTreeMap::new();
            for connection in connections.iter().flatten() {
                let (cable, slot) = (connection.pull_interval_key, connection.attachment_index);
                let Some(interval) = fabric.intervals.get(cable) else {
                    continue;
                };
                let far = interval.other_joint(joint);
                // Aim at the far end's pivot, not the far joint: on short
                // cables the two differ by several degrees, and the fork
                // must stay collinear with the cable it holds.
                let aim = connector
                    .pull_end_pivot(fabric, cable, far)
                    .unwrap_or(fabric.joints[far].location);
                let (pivot, _) = connector.connection_pivot(end_pos, axis, connection, aim);
                let ring_center = connector.connection_ring_center(end_pos, axis, connection);
                let radial = (pivot - ring_center).normalize();
                let tangent = axis.cross(radial).normalize();
                let culprit = connector.culprits.contains(&(cable, joint));

                let disc = heights.entry(slot).or_insert((ring_center, Vec::new(), false, connection.lift));
                disc.1.push((radial, connection.reach.map(|reach| reach.f32())));
                disc.2 |= culprit;
                let cable_dir = (aim - pivot).normalize();
                if is_hanger(fabric, cable) {
                    let far_pin = connector.node_plate(far).map_or(tangent, |plate| plate.normal);
                    let near_pin = tangent.reject_from(cable_dir).normalize();
                    let far_pin = far_pin.reject_from(cable_dir).normalize_or(near_pin);
                    let far_end = pivot + cable_dir * HANGER_LENGTH.f32();
                    place(Part::RodFork, pivot, cable_dir, near_pin, culprit);
                    place(Part::ForkNuts, pivot, cable_dir, near_pin, culprit);
                    place(Part::RodFork, far_end, -cable_dir, far_pin, culprit);
                    place(Part::ForkNuts, far_end, -cable_dir, far_pin, culprit);
                } else if connector.link_length(cable).is_some() {
                    let pin_axis = tangent.reject_from(cable_dir).normalize();
                    place(Part::RodFork, pivot, cable_dir, pin_axis, culprit);
                    place(Part::ForkNuts, pivot, cable_dir, pin_axis, culprit);
                } else {
                    // Pin square to the cable, so the frame is rigid.
                    let pin_axis = tangent.reject_from(cable_dir).normalize();
                    place(Part::Clevis, pivot, cable_dir, pin_axis, culprit);
                }
            }
            // Under each ring, its washers, on the strut end or the ring below.
            let half_ring = axis * (connector.dimensions.ring_thickness.f32() / 2.0);
            let washer = connector.dimensions.washer_thickness.f32();
            let mut under = end_pos + axis * connector.dimensions.cap_thickness.f32();
            for (center, ..) in heights.values() {
                let gap = (*center - half_ring - under).dot(axis);
                let washers = (gap / washer).round() as u8;
                place(Part::Spacer { washers }, under, axis.any_orthonormal_vector(), axis, false);
                under = *center + half_ring;
            }
            for (center, bosses, culprit, _) in heights.into_values() {
                match bosses[..] {
                    [(radial, None)] => place(Part::Ring, center, radial, axis, culprit),
                    _ => {
                        let (x, shape) = disc_shape(&bosses, axis, connector.dimensions.pivot_radius.f32());
                        let shape = match frozen(index.get()) {
                            Some(Part::CustomDisc(kept)) => kept,
                            _ => shape,
                        };
                        place(Part::CustomDisc(shape), center, x, axis, culprit);
                    }
                }
            }
        }
    }

    for (pin, joint) in fabric.joints.iter() {
        let Some([clevis, rod_end]) = hinge_struts(fabric, pin) else {
            continue;
        };
        let along = |strut: IntervalKey| (fabric.joints[fabric.intervals[strut].other_joint(pin)].location - joint.location).normalize();
        let pin_axis = hinge_pin_axis(fabric, pin);
        for (part, strut) in [(Part::Hinge, clevis), (Part::HingeEye, rod_end)] {
            place(part, joint.location, along(strut), pin_axis, connector.culprits.contains(&(strut, pin)));
        }
    }

    for (&node, &link) in &fabric.bridle_nodes {
        let Some(plate) = connector.node_plate(node) else {
            continue;
        };
        for (cable, hole) in plate.holes {
            if is_hanger(fabric, cable) {
                continue;
            }
            let far = fabric.intervals[cable].other_joint(node);
            let aim = connector.pull_end_pivot(fabric, cable, far).unwrap_or(fabric.joints[far].location);
            let cable_dir = (aim - hole).normalize();
            let pin_axis = plate.normal.reject_from(cable_dir).normalize_or(plate.normal);
            let culprit = connector.culprits.contains(&(cable, node));
            if connector.link_length(cable).is_some() {
                place(Part::RodFork, hole, cable_dir, pin_axis, culprit);
                place(Part::ForkNuts, hole, cable_dir, pin_axis, culprit);
            } else {
                place(Part::Clevis, hole, cable_dir, pin_axis, culprit);
            }
        }
        let middle = plate.holes.iter().map(|(_, hole)| *hole).sum::<Vec3>() / 3.0;
        let x = (plate.holes[0].1 - middle).normalize();
        let culprit = connector.culprits.contains(&(link, node));
        place(Part::NodePlate(PlateShape::equilateral()), middle, x, plate.normal, culprit);
    }
    placed
}

/// A custom disc's frame `x` and its shape: `x` along the bosses' mean
/// direction (on the mirror plane for a symmetric disc), each boss's angle
/// measured from it about `axis`, made exactly symmetric about `x`.
fn disc_shape(bosses: &[(Vec3, Option<f32>)], axis: Vec3, standard_reach: f32) -> (Vec3, DiscShape) {
    let first = bosses[0].0;
    let x = bosses.iter().map(|(radial, _)| *radial).sum::<Vec3>().normalize_or(first);
    let z = x.cross(axis);
    let mut measured: Vec<(i16, u16)> = bosses
        .iter()
        .take(MAX_BOSSES)
        .map(|&(radial, reach)| {
            let degrees = radial.dot(z).atan2(radial.dot(x)).to_degrees().round() as i16;
            let millimetres = (reach.unwrap_or(standard_reach) * 1000.0).round() as u16;
            (degrees, millimetres)
        })
        .collect();
    measured.sort();
    // The part is cut perfectly symmetric about x: mirror bosses (first with
    // last, and so on inward) take their mean angle and the longer reach.
    let n = measured.len();
    for i in 0..n / 2 {
        let (low, high) = (measured[i], measured[n - 1 - i]);
        if low.0 < 0 && high.0 > 0 {
            let degrees = ((high.0 - low.0) as f32 / 2.0).round() as i16;
            let reach = low.1.max(high.1);
            measured[i] = (-degrees, reach);
            measured[n - 1 - i] = (degrees, reach);
        }
    }
    let mut shape = DiscShape { bosses: [(0, 0); MAX_BOSSES], count: measured.len() as u8 };
    shape.bosses[..measured.len()].copy_from_slice(&measured);
    (x, shape)
}

/// How many of each part the placements use.
pub fn parts_list(placements: &[Placement]) -> BTreeMap<Part, usize> {
    let mut counts = BTreeMap::new();
    for placement in placements.iter().filter(|placement| placement.part.listed()) {
        *counts.entry(placement.part).or_insert(0) += 1;
    }
    counts
}

/// The list's parts with short ids for the command line: the kind's name in
/// dashes, numbered where the kind has several shapes (`pom-washer-2`).
pub fn part_ids(list: &BTreeMap<Part, usize>) -> Vec<(String, Part, usize)> {
    let slug = |part: Part| part.base_name().split(" (").next().unwrap_or_default().replace(' ', "-").to_lowercase();
    let kind_size = |part: Part| list.keys().filter(|other| other.base_name() == part.base_name()).count();
    let mut ids: Vec<(String, Part, usize)> = Vec::new();
    for (&part, &count) in list {
        let earlier = ids.iter().filter(|(_, other, _)| other.base_name() == part.base_name()).count();
        let id = if kind_size(part) > 1 { format!("{}-{}", slug(part), earlier + 1) } else { slug(part) };
        ids.push((id, part, count));
    }
    ids
}

/// One kind of part as the parts panel lists it: how many copies in all,
/// in how many distinct shapes, and its most numerous shape, to show.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KindSummary {
    pub name: &'static str,
    pub count: usize,
    pub shapes: usize,
    pub sample: Part,
}

/// The parts list by kind of part, in parts-list order. Each distinct shape
/// stays its own line of the parts list, as a cutting order needs; this
/// only groups them for display. The list is sorted by part, so each kind's
/// shapes lie together.
pub fn kinds(list: &BTreeMap<Part, usize>) -> Vec<KindSummary> {
    let mut kinds: Vec<KindSummary> = Vec::new();
    for (&part, &count) in list {
        match kinds.last_mut() {
            Some(kind) if kind.name == part.base_name() => {
                kind.count += count;
                kind.shapes += 1;
                if count > list[&kind.sample] {
                    kind.sample = part;
                }
            }
            _ => kinds.push(KindSummary { name: part.base_name(), count, shapes: 1, sample: part }),
        }
    }
    kinds
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::dsl::fabric_library::{self, FabricName};
    use crate::build::dsl::fabric_plan_executor::FabricPlanExecutor;
    use crate::connector::fork::FORK_ROD_MIN_LENGTH;

    /// Headless Hug's parts list adds up: every boss (one per ring, several
    /// per custom disc) carries one cross-tube and a jaw terminal or a
    /// rod fork, and every bridle node has a plate with a terminal or rod fork
    /// for each of its cables but its hanger, or a clamp.
    #[test]
    fn headless_hug_parts_list() {
        let mut executor = FabricPlanExecutor::new(fabric_library::get_fabric_plan(FabricName::HeadlessHug));
        while !executor.is_complete() {
            let _ = executor.iterate();
        }
        executor.fabric.update_all_attachment_connections();
        let placed = placements(&executor.fabric);
        let list = parts_list(&placed);
        for (part, count) in &list {
            eprintln!("{:>5} × {}", count, part.name());
        }
        let count = |part| list.get(&part).copied().unwrap_or(0);
        let disc_bosses: usize = list
            .iter()
            .filter_map(|(part, n)| match part {
                Part::CustomDisc(shape) => Some(shape.count as usize * n),
                _ => None,
            })
            .sum();
        let plates: usize = list
            .iter()
            .filter_map(|(part, n)| matches!(part, Part::NodePlate(_)).then_some(*n))
            .sum();
        let bosses = count(Part::Ring) + disc_bosses;
        let connector = executor.fabric.connector.as_ref().expect("fabric has connectors");
        let rods: Vec<f32> = executor.fabric.intervals.keys().filter_map(|key| connector.link_length(key)).collect();
        let hangers = executor.fabric.bridle_hangers.len();
        assert_eq!((rods.len() + hangers) * 2, count(Part::RodFork));
        assert!(rods.iter().all(|&length| length >= FORK_ROD_MIN_LENGTH.f32()));
        assert_eq!(plates, executor.fabric.bridle_nodes.len());
        assert_eq!(bosses + 3 * plates, count(Part::Clevis) + count(Part::RodFork));
        let kinds = kinds(&list);
        let listed = placed.iter().filter(|placement| placement.part.listed()).count();
        assert_eq!(kinds.iter().map(|kind| kind.count).sum::<usize>(), listed);
        assert_eq!(kinds.iter().map(|kind| kind.shapes).sum::<usize>(), list.len());
        for kind in &kinds {
            eprintln!("{:>5} × {} in {} shapes, showing {}", kind.count, kind.name, kind.shapes, kind.sample.name());
        }
    }

    /// A hinge carries only its two struts: every cable at a hinge pin is on
    /// one of their stacks, none on the hinge itself.
    #[test]
    fn no_cable_ends_on_a_hinge() {
        let mut executor = FabricPlanExecutor::new(fabric_library::get_fabric_plan(FabricName::HeadlessHug));
        while !executor.is_complete() {
            let _ = executor.iterate();
        }
        let fabric = &executor.fabric;
        let mut hinges = 0;
        for pin in fabric.joints.keys() {
            let Some(struts) = hinge_struts(fabric, pin) else {
                continue;
            };
            hinges += 1;
            for (key, interval) in fabric.intervals.iter() {
                if interval.role.is_pull_like() && interval.touches(pin) {
                    let strut = fabric.hinge_strut.get(&(key, pin));
                    assert!(strut.is_some_and(|strut| struts.contains(strut)), "a cable ends on hinge {}", fabric.joint_label(pin));
                }
            }
        }
        assert_eq!(hinges, 8);
    }
}
