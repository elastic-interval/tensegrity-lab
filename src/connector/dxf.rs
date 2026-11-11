//! DXF drawings of the laser-cut parts, for ordering: true size in
//! millimetres, as plain lines, arcs and circles on one layer, which any
//! cutter's software reads. A drawing is the flat part only; the cross-tubes
//! are welded on afterwards.

use crate::connector::dimensions::ConnectorDimensions;
use crate::connector::fork::{
    BOSS_ARM_RADIUS, BOSS_SHORT_OF_PIVOT, PLATE_CORNER_RADIUS, PLATE_HOLE_DIAMETER, PLATE_SIDE, PLATE_THICKNESS,
    RING_BORE_DIAMETER,
};
use crate::connector::parts::{parts_list, placements, Part};
use crate::fabric::Fabric;
use crate::units::{Unit, MM_PER_METER};
use std::f32::consts::TAU;
use std::fmt::Write;

const MM: f32 = MM_PER_METER;

#[derive(Clone, Copy, Debug)]
enum Entity {
    Line { from: [f32; 2], to: [f32; 2] },
    /// Counter-clockwise from `start` to `end`, in radians.
    Arc { center: [f32; 2], radius: f32, start: f32, end: f32 },
    Circle { center: [f32; 2], radius: f32 },
}

fn at(center: [f32; 2], radius: f32, angle: f32) -> [f32; 2] {
    [center[0] + radius * angle.cos(), center[1] + radius * angle.sin()]
}

/// One laser-cut part: what to call it, its plate thickness in millimetres,
/// and its outline and holes.
pub struct Drawing {
    pub name: String,
    pub thickness_mm: u16,
    entities: Vec<Entity>,
}

/// The drawing of `part`, if it is cut from plate. `index` tells apart the
/// parts of one kind. `ring_radius` is the fabric's strut radius, in metres.
pub fn drawing(part: Part, index: usize, dimensions: &ConnectorDimensions, ring_radius: f32) -> Option<Drawing> {
    let thickness_mm = (dimensions.ring_thickness.f32() * MM).round() as u16;
    let radius = ring_radius * MM;
    let short = BOSS_SHORT_OF_PIVOT.f32() * MM;
    match part {
        Part::Ring => Some(Drawing {
            name: "ring".to_string(),
            thickness_mm,
            entities: disc_entities(radius, &[(0.0, dimensions.pivot_radius.f32() * MM - short)]),
        }),
        Part::CustomDisc(shape) => {
            let bosses: Vec<(f32, f32)> = shape
                .bosses()
                .iter()
                .map(|&(degrees, reach)| ((degrees as f32).to_radians(), reach as f32 - short))
                .collect();
            Some(Drawing {
                name: format!("disc-{index}"),
                thickness_mm,
                entities: disc_entities(radius, &bosses),
            })
        }
        Part::NodePlate(_) => Some(Drawing {
            name: "node-plate".to_string(),
            thickness_mm: (PLATE_THICKNESS.f32() * MM).round() as u16,
            entities: plate_entities(),
        }),
        _ => None,
    }
}

/// A disc with its bore and a boss at each of `bosses`, (angle in radians,
/// ascending; distance from the centre to the boss's flat end).
fn disc_entities(radius: f32, bosses: &[(f32, f32)]) -> Vec<Entity> {
    let half_width = BOSS_ARM_RADIUS.f32() * MM;
    let shoulder = (half_width / radius).asin();
    let center = [0.0, 0.0];
    let mut entities = vec![Entity::Circle { center, radius: RING_BORE_DIAMETER.f32() * MM / 2.0 }];
    for (i, &(angle, end)) in bosses.iter().enumerate() {
        let (toward, side) = ([angle.cos(), angle.sin()], [-angle.sin(), angle.cos()]);
        let corner = |sign: f32| {
            [toward[0] * end + side[0] * half_width * sign, toward[1] * end + side[1] * half_width * sign]
        };
        entities.push(Entity::Line { from: at(center, radius, angle - shoulder), to: corner(-1.0) });
        entities.push(Entity::Line { from: corner(-1.0), to: corner(1.0) });
        entities.push(Entity::Line { from: corner(1.0), to: at(center, radius, angle + shoulder) });
        let next = bosses.get(i + 1).map_or(bosses[0].0 + TAU, |&(next, _)| next);
        entities.push(Entity::Arc { center, radius, start: angle + shoulder, end: next - shoulder });
    }
    entities
}

/// The equilateral node plate: round each hole a corner, with straight edges
/// between them, and the holes.
fn plate_entities() -> Vec<Entity> {
    let out = PLATE_SIDE.f32() * MM / 3f32.sqrt();
    let corner = PLATE_CORNER_RADIUS.f32() * MM;
    let hole = PLATE_HOLE_DIAMETER.f32() * MM / 2.0;
    let third = TAU / 3.0;
    let sixth = TAU / 6.0;
    let mut entities = Vec::new();
    for i in 0..3 {
        let angle = third * i as f32;
        let center = at([0.0, 0.0], out, angle);
        let next = at([0.0, 0.0], out, angle + third);
        entities.push(Entity::Circle { center, radius: hole });
        entities.push(Entity::Arc { center, radius: corner, start: angle - sixth, end: angle + sixth });
        entities.push(Entity::Line {
            from: at(center, corner, angle + sixth),
            to: at(next, corner, angle + sixth),
        });
    }
    entities
}

impl Drawing {
    /// The file's name, saying what to order: part, thickness and quantity.
    pub fn file_name(&self, quantity: usize) -> String {
        format!("{}-{}mm-qty{quantity}.dxf", self.name, self.thickness_mm)
    }

    /// The drawing as a DXF file, in millimetres.
    pub fn dxf(&self) -> String {
        let mut dxf = String::new();
        let _ = write!(
            dxf,
            "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1009\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n"
        );
        for entity in &self.entities {
            let _ = match *entity {
                Entity::Line { from, to } => write!(
                    dxf,
                    "0\nLINE\n8\n0\n10\n{:.4}\n20\n{:.4}\n30\n0.0\n11\n{:.4}\n21\n{:.4}\n31\n0.0\n",
                    from[0], from[1], to[0], to[1]
                ),
                Entity::Arc { center, radius, start, end } => write!(
                    dxf,
                    "0\nARC\n8\n0\n10\n{:.4}\n20\n{:.4}\n30\n0.0\n40\n{:.4}\n50\n{:.4}\n51\n{:.4}\n",
                    center[0],
                    center[1],
                    radius,
                    start.to_degrees().rem_euclid(360.0),
                    end.to_degrees().rem_euclid(360.0)
                ),
                Entity::Circle { center, radius } => {
                    write!(dxf, "0\nCIRCLE\n8\n0\n10\n{:.4}\n20\n{:.4}\n30\n0.0\n40\n{:.4}\n", center[0], center[1], radius)
                }
            };
        }
        dxf.push_str("0\nENDSEC\n0\nEOF\n");
        dxf
    }
}

/// Every drawing with its quantity, one per distinct laser-cut part.
pub fn drawings(fabric: &Fabric) -> Vec<(Drawing, usize)> {
    let connector = fabric.connector.as_ref().expect("fabric has connectors");
    let radius = fabric.dimensions.push_radius.f32();
    parts_list(&placements(fabric))
        .into_iter()
        .enumerate()
        .filter_map(|(index, (part, quantity))| Some((drawing(part, index, &connector.dimensions, radius)?, quantity)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::dsl::fabric_library::{self, FabricName};
    use crate::build::dsl::fabric_plan_executor::FabricPlanExecutor;
    use crate::fabric::Fabric;

    fn built_headless_hug() -> Fabric {
        let mut executor = FabricPlanExecutor::new(fabric_library::get_fabric_plan(FabricName::HeadlessHug));
        while !executor.is_complete() {
            let _ = executor.iterate();
        }
        let mut fabric = executor.fabric;
        fabric.update_all_attachment_connections();
        fabric
    }

    /// A cutter needs one closed outline: every end of every line and arc
    /// meets exactly one other, and every arc turns the short way it should.
    #[test]
    fn outlines_are_closed() {
        let fabric = built_headless_hug();
        let drawings = drawings(&fabric);
        assert_eq!(drawings.len(), 6, "ring, 4 discs, node plate");
        for (drawing, _) in &drawings {
            let mut ends: Vec<[f32; 2]> = Vec::new();
            for entity in &drawing.entities {
                match *entity {
                    Entity::Line { from, to } => ends.extend([from, to]),
                    Entity::Arc { center, radius, start, end } => {
                        assert!(end > start, "{}: a boss runs into the next", drawing.name);
                        ends.extend([at(center, radius, start), at(center, radius, end)]);
                    }
                    Entity::Circle { .. } => {}
                }
            }
            for end in &ends {
                let meeting = ends.iter().filter(|other| (other[0] - end[0]).hypot(other[1] - end[1]) < 0.001).count();
                assert_eq!(meeting, 2, "{}: open outline at {end:?}", drawing.name);
            }
        }
    }

    /// Writes `HeadlessHug-parts-<date>/`, one DXF per laser-cut part.
    /// Marked `#[ignore]` so it runs only on demand:
    ///
    /// ```
    /// cargo test --release --lib export_headless_hug_dxf -- --ignored
    /// ```
    #[test]
    #[ignore]
    fn export_headless_hug_dxf() {
        let fabric = built_headless_hug();
        let directory = format!("HeadlessHug-parts-{}", chrono::Local::now().format("%Y-%m-%d"));
        std::fs::create_dir_all(&directory).expect("create directory");
        for (drawing, quantity) in drawings(&fabric) {
            let path = format!("{directory}/{}", drawing.file_name(quantity));
            std::fs::write(&path, drawing.dxf()).expect("write DXF");
            println!("{path}");
        }
    }
}
