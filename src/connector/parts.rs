//! The parts list: every distinct physical part the connector hardware is
//! made of, and where each copy of it sits in the structure. A part has one
//! shape, defined once in its own frame; the structure is nothing but
//! placements of those shapes. Rendering draws each part once, replicated;
//! counting placements gives the bill of materials. A newly discovered part
//! is a new `Part` variant with its mesh, and the compiler finds every place
//! that must learn about it.

use crate::connector::node_plate::NodePlate;
use crate::connector::system::is_shackle;
use crate::fabric::interval::Role;
use crate::fabric::{Fabric, IntervalEnd};
use crate::units::Unit;
use glam::Vec3;
use std::collections::BTreeMap;

/// A part's frame: `x` and `y` are orthogonal unit vectors, `z = x × y`.
/// Each variant says how its shape sits in that frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Part {
    /// Strut end-cap the stack bolts onto: a disc from the strut end
    /// (origin) outward along `y`.
    Cap,
    /// Flat ring turning on the bolt, centred at the origin with `y` along
    /// the bolt; its boss reaches out along `x` toward the cable.
    Ring,
    /// Laser-cut disc made to suit one joint, carrying several cables at one
    /// height: centred at the origin, `y` along the bolt, its bosses at the
    /// angles and reaches of its `DiscShape`, measured from `x`. Each
    /// distinct shape is a part of its own.
    CustomDisc(DiscShape),
    /// Washers filling a gap that was lifted to clear the forks: `washers`
    /// thick, from the top of the ring below (origin) up along `y`.
    Spacer { washers: u8 },
    /// Cross-tube at the end of the boss: centred on the pivot (origin),
    /// along `y`.
    CrossTube,
    /// Jaw terminal with pin and split ring: pin centre at the origin, barrel
    /// along `x`, pin along `y` (see `connector::fork::ClevisDimensions`).
    Clevis,
    /// Shackle hanging a node plate or clamp from a cross-tube: from the
    /// pivot (origin) along `x` to the plate's hole or the clamp's eye.
    Shackle,
    /// Bridle clamp, a cheek block round a grooved saddle the cable wraps
    /// (see `connector::bridle_clamp`): the saddle's centre at the origin,
    /// `x` toward the eye and the strut it hangs from, `y` through the
    /// cheeks.
    Clamp,
    /// Triangular node plate at a bridle node, cut to suit its three cables
    /// (see `connector::node_plate`): centred at the origin, `y` square to
    /// the plate, its holes at the angles of its `PlateShape`, measured from
    /// `x`. Each distinct shape is a part of its own.
    NodePlate(PlateShape),
}

/// A node plate's three holes as (angle from `x` in degrees, distance from
/// the centre in millimetres), ascending by angle, the first at 0. Rounded,
/// so plates that match to the degree and millimetre are the same part.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlateShape {
    pub holes: [(i16, u16); 3],
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
            Part::Spacer { washers } => format!("spacer of {washers} washers"),
            Part::NodePlate(shape) => {
                let holes: Vec<String> = shape.holes.iter().map(|(degrees, mm)| format!("{degrees}°/{mm}mm")).collect();
                format!("node plate {}", holes.join(" "))
            }
            part => part.base_name().to_string(),
        }
    }

    fn base_name(self) -> &'static str {
        match self {
            Part::Cap => "strut cap",
            Part::Ring => "ring with boss",
            Part::CustomDisc(_) => "custom disc",
            Part::Spacer { .. } => "spacer",
            Part::CrossTube => "cross-tube",
            Part::Clevis => "jaw terminal",
            Part::Shackle => "bridle shackle",
            Part::Clamp => "bridle clamp",
            Part::NodePlate(_) => "node plate",
        }
    }

    pub fn material(self) -> Material {
        match self {
            Part::Cap | Part::Ring | Part::CustomDisc(_) | Part::CrossTube | Part::NodePlate(_) => {
                Material::Galvanized
            }
            Part::Clevis | Part::Shackle | Part::Clamp => Material::Stainless,
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
    let mut place = |part: Part, origin: Vec3, x: Vec3, y: Vec3, culprit: bool| {
        placed.push(Placement { part, origin, x, y, culprit });
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
            let mut occupied = false;
            // A ring or custom disc per height: centre, each cable's radial
            // and custom reach, culprit.
            let mut heights: BTreeMap<usize, (Vec3, Vec<(Vec3, Option<f32>)>, bool, u8)> = BTreeMap::new();
            for connection in connections.iter().flatten() {
                let (cable, slot) = (connection.pull_interval_key, connection.attachment_index);
                let Some(interval) = fabric.intervals.get(cable) else {
                    continue;
                };
                occupied = true;
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
                place(Part::CrossTube, pivot, radial, tangent, culprit);
                let cable_dir = (aim - pivot).normalize();
                if is_shackle(fabric, cable) {
                    place(Part::Shackle, pivot, cable_dir, cable_dir.any_orthonormal_vector(), culprit);
                } else {
                    // Pin square to the cable, so the frame is rigid.
                    let pin_axis = tangent.reject_from(cable_dir).normalize();
                    place(Part::Clevis, pivot, cable_dir, pin_axis, culprit);
                }
            }
            // Lifted gaps are filled with spacers from the ring below.
            let half_ring = axis * (connector.dimensions.ring_thickness.f32() / 2.0);
            let mut below: Option<(Vec3, u8)> = None;
            for (center, _, _, lift) in heights.values() {
                if let Some((under, under_lift)) = below {
                    if *lift > under_lift {
                        let washers = lift - under_lift + 1;
                        place(Part::Spacer { washers }, under + half_ring, axis.any_orthonormal_vector(), axis, false);
                    }
                }
                below = Some((*center, *lift));
            }
            for (center, bosses, culprit, _) in heights.into_values() {
                match bosses[..] {
                    [(radial, None)] => place(Part::Ring, center, radial, axis, culprit),
                    _ => {
                        let (x, shape) = disc_shape(&bosses, axis, connector.dimensions.pivot_radius.f32());
                        place(Part::CustomDisc(shape), center, x, axis, culprit);
                    }
                }
            }
            if occupied {
                place(Part::Cap, end_pos, axis.any_orthonormal_vector(), axis, false);
            }
        }
    }

    for (&node, &link) in &fabric.bridle_nodes {
        if let Some(clamp) = connector.bridle_clamp(node) {
            let culprit = connector.culprits.contains(&(link, node)) || connector.over_bent.contains(&node);
            place(Part::Clamp, clamp.saddle, clamp.toward_link, clamp.normal, culprit);
            continue;
        }
        let Some(plate) = connector.node_plate(node) else {
            continue;
        };
        for (cable, hole) in plate.holes {
            if is_shackle(fabric, cable) {
                continue;
            }
            let far = fabric.intervals[cable].other_joint(node);
            let aim = connector.pull_end_pivot(fabric, cable, far).unwrap_or(fabric.joints[far].location);
            let cable_dir = (aim - hole).normalize();
            let pin_axis = plate.normal.reject_from(cable_dir).normalize_or(plate.normal);
            let culprit = connector.culprits.contains(&(cable, node));
            place(Part::Clevis, hole, cable_dir, pin_axis, culprit);
        }
        let (x, y, shape) = plate_shape(&plate);
        let culprit = connector.culprits.contains(&(link, node));
        place(Part::NodePlate(shape), plate.center, x, y, culprit);
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

/// A node plate's frame and its shape: each hole's angle from `x` about
/// `y`. A plate turned over or started from another hole is the same part,
/// so of all three starting holes and both faces the frame giving the
/// smallest angles is taken: mirror-image plates then share one shape.
fn plate_shape(plate: &NodePlate) -> (Vec3, Vec3, PlateShape) {
    let toward: Vec<Vec3> = plate.holes.iter().map(|(_, hole)| *hole - plate.center).collect();
    let frames = toward
        .iter()
        .flat_map(|first| [plate.normal, -plate.normal].map(|y| (first.normalize(), y)));
    frames
        .map(|(x, y)| {
            let z = x.cross(y);
            let mut holes = [(0i16, 0u16); 3];
            for ((angle, mm), hole) in holes.iter_mut().zip(&toward) {
                *angle = (hole.dot(z).atan2(hole.dot(x)).to_degrees().round() as i16).rem_euclid(360);
                *mm = (hole.length() * 1000.0).round() as u16;
            }
            holes.sort();
            (x, y, PlateShape { holes })
        })
        .min_by_key(|(_, _, shape)| *shape)
        .expect("a plate has holes")
}

/// How many of each part the placements use.
pub fn parts_list(placements: &[Placement]) -> BTreeMap<Part, usize> {
    let mut counts = BTreeMap::new();
    for placement in placements {
        *counts.entry(placement.part).or_insert(0) += 1;
    }
    counts
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

    /// Headless Hug's parts list adds up: every boss (one per ring, several
    /// per custom disc) carries one cross-tube and a jaw terminal or a
    /// shackle, and every bridle node has a plate with a terminal for each
    /// of its cables but the one on a shackle, or a clamp.
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
        assert_eq!(bosses, count(Part::CrossTube));
        assert_eq!(plates + count(Part::Clamp), executor.fabric.bridle_nodes.len());
        assert_eq!(bosses + 2 * plates, count(Part::Clevis) + count(Part::Shackle));
        let kinds = kinds(&list);
        assert_eq!(kinds.iter().map(|kind| kind.count).sum::<usize>(), placed.len());
        assert_eq!(kinds.iter().map(|kind| kind.shapes).sum::<usize>(), list.len());
        for kind in &kinds {
            eprintln!("{:>5} × {} in {} shapes, showing {}", kind.count, kind.name, kind.shapes, kind.sample.name());
        }
    }
}
