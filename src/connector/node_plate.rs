//! A node plate: the triangular steel plate at a bridle node, with a hole
//! for each of the three cables meeting there. Each cable is a whole cable
//! with a jaw terminal at both ends, and the node end's terminal pins
//! through its hole; or the plate hangs on a shackle from the strut end it
//! bridles. Three forces holding a point are coplanar, so the plate lies
//! square to its cables, each hole on its cable's line.
//!
//! Each hole sits as close to the centre as it can, so plates are small and
//! irregular: a terminal's hole moves out only until its terminal clears the
//! plate's others, and a shackle's hole is wherever the shackle reaches.
//!
//! The simulation knows the plate only as the joint where the three cables'
//! lines meet: the plate's centre.

use crate::connector::fork::{
    self, Assembly, PLATE_HOLE_MAX_RADIUS, PLATE_HOLE_MIN_RADIUS, SHACKLE_LENGTH,
};
use crate::fabric::IntervalKey;
use crate::units::Unit;
use glam::Vec3;

/// How far a hole steps out at a time while clearing its terminal.
const HOLE_STEP: f32 = 0.002;

#[derive(Clone, Debug)]
pub struct NodePlate {
    /// Where the cables' lines meet: the simulated joint.
    pub center: Vec3,
    /// Square to the plate, along its pins.
    pub normal: Vec3,
    /// Each cable with its hole, where its terminal's or shackle's pin passes.
    pub holes: [(IntervalKey, Vec3); 3],
    /// The shackle can't reach: its hole would fall inside the plate's
    /// minimum, so the plate would jam against the strut end.
    pub jammed: bool,
}

impl NodePlate {
    /// The plate at `center` for three cables, each aimed at its far end;
    /// one may hang the plate on a shackle from the strut end's pivot it
    /// aims at.
    pub fn new(center: Vec3, cables: [(IntervalKey, Vec3, bool); 3]) -> NodePlate {
        let [a, b, c] = cables.map(|(_, aim, _)| (aim - center).normalize());
        let normal = (a.cross(b) + b.cross(c) + c.cross(a)).normalize_or(a.any_orthonormal_vector());
        let toward = cables.map(|(_, aim, _)| (aim - center).reject_from(normal).normalize_or(a));
        let (min, max) = (PLATE_HOLE_MIN_RADIUS.f32(), PLATE_HOLE_MAX_RADIUS.f32());
        let mut jammed = false;
        let mut radii = [min; 3];
        for (i, &(_, pivot, shackle)) in cables.iter().enumerate() {
            if shackle {
                let reach = (pivot - center).dot(toward[i]) - SHACKLE_LENGTH.f32();
                jammed = reach < min;
                radii[i] = reach.max(min);
            }
        }
        let assembly = |i: usize, radii: &[f32; 3]| -> Assembly {
            let (_, aim, shackle) = cables[i];
            let hole = center + toward[i] * radii[i];
            if shackle {
                fork::shackle_capsules(hole, aim)
            } else {
                let cable_dir = (aim - hole).normalize();
                fork::capsules(hole, normal.reject_from(cable_dir).normalize_or(normal), cable_dir)
            }
        };
        // Step out whichever terminals still touch another until all clear.
        loop {
            let mut moved = false;
            for i in 0..3 {
                let (_, _, shackle) = cables[i];
                if shackle || radii[i] >= max {
                    continue;
                }
                let touches = (0..3)
                    .filter(|&j| j != i)
                    .any(|j| fork::assemblies_collide(&assembly(i, &radii), &assembly(j, &radii)));
                if touches {
                    radii[i] = (radii[i] + HOLE_STEP).min(max);
                    moved = true;
                }
            }
            if !moved {
                break;
            }
        }
        let holes = [0, 1, 2].map(|i| (cables[i].0, center + toward[i] * radii[i]));
        NodePlate { center, normal, holes, jammed }
    }

    /// The hole `cable`'s terminal or shackle pins through, if it is one of
    /// this plate's.
    pub fn hole(&self, cable: IntervalKey) -> Option<Vec3> {
        self.holes.iter().find(|(key, _)| *key == cable).map(|(_, hole)| *hole)
    }
}
