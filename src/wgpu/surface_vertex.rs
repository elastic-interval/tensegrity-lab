use crate::fabric::quake::{
    corner_angle, drum_along, hook_foot, ring, Deck, BASE_THICKNESS, DECK_THICKNESS, DRUM_CORNERS, HANG_CORNERS, HEAD_CLEARANCE,
    HOOK_REACH, TIE_RADIUS,
};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use std::f32::consts::PI;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Default)]
pub struct SurfaceVertex {
    position: [f32; 4],
}

impl SurfaceVertex {
    pub fn for_radius(radius: f32) -> [SurfaceVertex; 18] {
        let origin = [0f32, 0.0, 0.0, 1.0];
        let point: Vec<[f32; 4]> = (0..6)
            .map(|index| index as f32 * PI / 3.0)
            .map(|angle| [radius * angle.cos(), 0.0, radius * angle.sin(), 1.0])
            .collect();
        let triangles = [
            origin, point[1], point[0], origin, point[2], point[1], origin, point[3], point[2],
            origin, point[4], point[3], origin, point[5], point[4], origin, point[0], point[5],
        ];
        triangles.map(|position| SurfaceVertex { position })
    }

    const ATTRIBUTES: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0=>Float32x4];

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<SurfaceVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

/// The wobble table, in the deck's own frame: the deck, and the hooks and
/// lugs hanging under it, which move; the base on the floor, the arms rising
/// from it and the steppers with their pulleys, which stay; the ties between
/// hooks and arms, and the cable loops from each lug round its pulleys to
/// its stepper's drum.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Default)]
pub struct DeckVertex {
    /// Its fourth says what it moves with: one the deck, zero nothing.
    position: [f32; 4],
    color: [f32; 4],
}

const STAYS: f32 = 0.0;
const ON_DECK: f32 = 1.0;
/// A lamp sunk in a deck: it goes with the deck, and shines only up its tube.
const LAMP: f32 = 2.0;

/// Half the width of a hook's or an arm's square tube, of a tie or the
/// ring, and of a motor and its drum.
const TUBE: f32 = 0.0125;
const TIE: f32 = 0.003;
const MOTOR: f32 = 0.03;

const DECK_COLOR: [f32; 3] = [0.10, 0.075, 0.05];
const BASE_COLOR: [f32; 3] = [0.12, 0.12, 0.13];
const HOOK_COLOR: [f32; 3] = [0.45, 0.46, 0.5];
const ARM_COLOR: [f32; 3] = [0.25, 0.27, 0.32];
const TIE_COLOR: [f32; 3] = [0.9, 0.85, 0.6];
const CABLE_COLOR: [f32; 3] = [0.95, 0.5, 0.2];
const MOTOR_COLOR: [f32; 3] = [0.1, 0.1, 0.1];
const WALL_COLOR: [f32; 3] = [0.8, 0.8, 0.78];
const FLOOR_COLOR: [f32; 3] = [0.3, 0.29, 0.28];
const VELVET_COLOR: [f32; 3] = [0.28, 0.03, 0.06];
/// The tube a spot's lamp is sunk in, in the middle of its deck: how wide
/// across its corners, and its black rim.
const LAMP_TUBE: f32 = 0.03;
const LAMP_RIM: f32 = 0.05;
const RIM_COLOR: [f32; 3] = [0.02, 0.02, 0.02];
/// How far out the velvet skirt hangs, as a part of the deck's radius,
/// clear of the base, and how far it clears the floor.
const SKIRT: f32 = 1.12;
const SKIRT_CLEAR: f32 = 0.01;
/// The skirt's pleats: how wide each fold, and how deep.
const PLEAT: f32 = 0.04;
const PLEAT_DEPTH: f32 = 0.012;
const CEILING_COLOR: [f32; 3] = [0.6, 0.6, 0.6];
/// The rubber cover over the power cable on the floor: flat on top, this
/// high and wide, its sides sloping out to this wide at the floor.
const COVER_COLOR: [f32; 3] = [0.06, 0.06, 0.06];
const COVER_HEIGHT: f32 = 0.025;
const COVER_TOP: f32 = 0.04;
const COVER_FOOT: f32 = 0.09;
/// How thick the gallery's walls are.
const WALL: f32 = 0.1;

impl DeckVertex {
    pub fn for_radius(radius: f32, height: f32) -> Vec<DeckVertex> {
        let mut vertices = Vec::new();
        let deck_bottom = -DECK_THICKNESS;
        let base_top = -height + BASE_THICKNESS;
        let head = deck_bottom - HEAD_CLEARANCE;
        let foot = hook_foot();
        prism(&mut vertices, radius, deck_bottom, 0.0, DECK_COLOR, ON_DECK);
        prism(&mut vertices, radius * 1.08, -height, base_top, BASE_COLOR, STAYS);
        let tie_radius = TIE_RADIUS * radius;
        for corner in HANG_CORNERS {
            let angle = corner_angle(corner);
            let out = Vec3::new(angle.cos(), 0.0, angle.sin());
            let at = |along: f32, y: f32| out * along + Vec3::Y * y;
            let (inner, outer) = (tie_radius - HOOK_REACH, tie_radius + HOOK_REACH);
            let hook = [at(inner, deck_bottom), at(inner, foot), at(tie_radius + TUBE, foot)];
            bar(&mut vertices, hook[0], hook[1], TUBE, HOOK_COLOR, (ON_DECK, ON_DECK));
            bar(&mut vertices, hook[1] + out * -TUBE, hook[2], TUBE, HOOK_COLOR, (ON_DECK, ON_DECK));
            let arm = [at(outer, base_top), at(outer, head), at(tie_radius - TUBE, head)];
            bar(&mut vertices, arm[0], arm[1], TUBE, ARM_COLOR, (STAYS, STAYS));
            bar(&mut vertices, arm[1] + out * TUBE, arm[2], TUBE, ARM_COLOR, (STAYS, STAYS));
            bar(&mut vertices, at(tie_radius, foot), at(tie_radius, head), TIE, TIE_COLOR, (ON_DECK, STAYS));
        }
        let (hooks, motors) = ring(radius);
        for (drum, motor) in motors.into_iter().enumerate() {
            let below = Vec3::new(motor.x, base_top, motor.z);
            bar(&mut vertices, below, motor - Vec3::Y * MOTOR, MOTOR, MOTOR_COLOR, (STAYS, STAYS));
            let along = drum_along(drum) * MOTOR;
            bar(&mut vertices, motor - along, motor + along, MOTOR, ARM_COLOR, (STAYS, STAYS));
            let corner = DRUM_CORNERS[drum];
            for neighbour in [(corner + 5) % 6, (corner + 1) % 6] {
                let hook = hooks[HANG_CORNERS.iter().position(|&hung| hung == neighbour).unwrap()];
                bar(&mut vertices, hook, motor, TIE, CABLE_COLOR, (ON_DECK, STAYS));
            }
        }
        vertices
    }

    /// The table `table`, as it stands where `place` puts it with its
    /// deck at `deck`: none of it left for the shader to move.
    pub fn placed(table: &[DeckVertex], deck: Deck, place: Mat4) -> impl Iterator<Item = DeckVertex> + '_ {
        table.iter().map(move |vertex| {
            let [x, y, z, moves] = vertex.position;
            let point = if moves > 0.5 { deck.carry(Vec3::new(x, y, z)) } else { Vec3::new(x, y, z) };
            let lamp = if moves == LAMP { LAMP } else { STAYS };
            DeckVertex { position: place.transform_point3(point).extend(lamp).to_array(), color: vertex.color }
        })
    }

    /// A velvet skirt hanging from just under a deck `radius` across, near
    /// to the floor `height` below, going with the deck and hiding what is
    /// under it.
    pub fn skirt(radius: f32, height: f32) -> Vec<DeckVertex> {
        let mut vertices = Vec::new();
        let (radius, top, bottom) = (radius * SKIRT, -0.005, -height + SKIRT_CLEAR);
        prism(&mut vertices, radius, -0.02, top, VELVET_COLOR, ON_DECK);
        let corner = |index: usize| {
            let angle = corner_angle(index % 6);
            Vec3::new(angle.cos(), 0.0, angle.sin()) * radius
        };
        let folds = ((corner(0).distance(corner(1)) / PLEAT).round() as usize).max(1) * 2;
        let mut around = Vec::new();
        for side in 0..6 {
            let (from, to) = (corner(side), corner(side + 1));
            let out = (from + to).normalize();
            for fold in 0..folds {
                let depth = if fold % 2 == 0 { 0.0 } else { PLEAT_DEPTH };
                around.push(from.lerp(to, fold as f32 / folds as f32) + out * depth);
            }
        }
        let lit = shade(VELVET_COLOR, 1.0);
        for (index, &here) in around.iter().enumerate() {
            let next = around[(index + 1) % around.len()];
            let at = |point: Vec3, y: f32| [point.x, y, point.z, ON_DECK];
            for position in [at(here, top), at(next, top), at(next, bottom), at(here, top), at(next, bottom), at(here, bottom)] {
                vertices.push(DeckVertex { position, color: lit });
            }
        }
        vertices
    }

    /// A spot's lamp sunk in a tube in the middle of a deck, seen only by
    /// looking down the tube, and the tube's rim around it.
    pub fn lamp() -> Vec<DeckVertex> {
        let mut vertices = Vec::new();
        prism(&mut vertices, LAMP_RIM, -0.002, 0.001, RIM_COLOR, ON_DECK);
        prism(&mut vertices, LAMP_TUBE, -0.002, 0.002, RIM_COLOR, LAMP);
        vertices
    }

    /// A closed room, `width` across and `depth` deep, its floor at `floor`
    /// and its ceiling `height` above.
    pub fn room(width: f32, depth: f32, height: f32, floor: f32) -> Vec<DeckVertex> {
        let mut vertices = Vec::new();
        let (x, z, top) = (width / 2.0, depth / 2.0, floor + height);
        for (from, to) in [
            (Vec3::new(-x - WALL, floor, -z - WALL), Vec3::new(x + WALL, top, -z)),
            (Vec3::new(-x - WALL, floor, z), Vec3::new(x + WALL, top, z + WALL)),
            (Vec3::new(-x - WALL, floor, -z), Vec3::new(-x, top, z)),
            (Vec3::new(x, floor, -z), Vec3::new(x + WALL, top, z)),
        ] {
            slab(&mut vertices, from, to, WALL_COLOR);
        }
        slab(&mut vertices, Vec3::new(-x, floor - WALL, -z), Vec3::new(x, floor, z), FLOOR_COLOR);
        slab(&mut vertices, Vec3::new(-x, top, -z), Vec3::new(x, top + WALL, z), CEILING_COLOR);
        vertices
    }

    /// The cover over a stretch of power cable on the floor at `floor`,
    /// from `from` to `to`, across the floor.
    pub fn cable_cover(from: Vec3, to: Vec3, floor: f32) -> Vec<DeckVertex> {
        let along = (to - from).normalize();
        let side = along.cross(Vec3::Y);
        let at = |end: Vec3, out: f32, up: f32| (Vec3::new(end.x, floor + up, end.z) + side * out).extend(STAYS).to_array();
        let rise = Vec3::new(0.0, COVER_FOOT - COVER_TOP, 0.0);
        let mut vertices = Vec::new();
        let mut face = |near: (f32, f32), far: (f32, f32), normal: Vec3| {
            let color = shade(COVER_COLOR, light(normal.normalize()));
            let (a, b, c, d) = (at(from, near.0, near.1), at(from, far.0, far.1), at(to, far.0, far.1), at(to, near.0, near.1));
            for position in [a, b, c, a, c, d] {
                vertices.push(DeckVertex { position, color });
            }
        };
        let (low, high) = (0.002, COVER_HEIGHT);
        face((-COVER_TOP, high), (COVER_TOP, high), Vec3::Y);
        face((COVER_TOP, high), (COVER_FOOT, low), side * COVER_HEIGHT + rise);
        face((-COVER_FOOT, low), (-COVER_TOP, high), -side * COVER_HEIGHT + rise);
        vertices
    }

    const ATTRIBUTES: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0=>Float32x4, 1=>Float32x4];

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<DeckVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

/// How lit a face is that faces `normal`.
fn light(normal: Vec3) -> f32 {
    0.55 + 0.45 * normal.dot(Vec3::new(0.3, 0.8, 0.5).normalize()).max(0.0)
}

fn shade(color: [f32; 3], light: f32) -> [f32; 4] {
    [color[0] * light, color[1] * light, color[2] * light, 1.0]
}

/// A hexagonal prism from `bottom` to `top`: its top and its six sides.
fn prism(vertices: &mut Vec<DeckVertex>, radius: f32, bottom: f32, top: f32, color: [f32; 3], moves: f32) {
    let corner = |index: usize, y: f32| {
        let angle = corner_angle(index);
        [radius * angle.cos(), y, radius * angle.sin(), moves]
    };
    let mut push = |position: [f32; 4], color: [f32; 4]| vertices.push(DeckVertex { position, color });
    for index in 0..6 {
        let next = (index + 1) % 6;
        let lit = shade(color, light(Vec3::Y));
        push([0.0, top, 0.0, moves], lit);
        push(corner(next, top), lit);
        push(corner(index, top), lit);
        let middle = corner_angle(index) + PI / 6.0;
        let side = shade(color, light(Vec3::new(middle.cos(), 0.0, middle.sin())));
        for position in [corner(index, top), corner(next, top), corner(next, bottom)] {
            push(position, side);
        }
        for position in [corner(index, top), corner(next, bottom), corner(index, bottom)] {
            push(position, side);
        }
    }
}

/// A square tube from `from` to `to`, `half` its half-width, each end
/// moving with the deck or not.
fn bar(vertices: &mut Vec<DeckVertex>, from: Vec3, to: Vec3, half: f32, color: [f32; 3], moves: (f32, f32)) {
    let along = (to - from).normalize();
    let across = along.cross(if along.y.abs() > 0.9 { Vec3::X } else { Vec3::Y }).normalize() * half;
    let other = along.cross(across).normalize() * half;
    let ring = [across + other, other - across, -across - other, across - other];
    let end = |point: Vec3, moves: f32| ring.map(|offset| (point + offset).extend(moves).to_array());
    let (a, b) = (end(from, moves.0), end(to, moves.1));
    let mut push = |position: [f32; 4], color: [f32; 4]| vertices.push(DeckVertex { position, color });
    for side in 0..4 {
        let next = (side + 1) % 4;
        let lit = shade(color, light((ring[side] + ring[next]).normalize()));
        for position in [a[side], a[next], b[next], a[side], b[next], b[side]] {
            push(position, lit);
        }
    }
    for (cap, normal) in [(a, -along), (b, along)] {
        let lit = shade(color, light(normal));
        for position in [cap[0], cap[1], cap[2], cap[0], cap[2], cap[3]] {
            push(position, lit);
        }
    }
}

/// A box from corner `low` to corner `high`, still, its six faces.
fn slab(vertices: &mut Vec<DeckVertex>, low: Vec3, high: Vec3, color: [f32; 3]) {
    let corner = |x: bool, y: bool, z: bool| {
        let pick = |high_side: bool, low: f32, high: f32| if high_side { high } else { low };
        [pick(x, low.x, high.x), pick(y, low.y, high.y), pick(z, low.z, high.z), STAYS]
    };
    for axis in 0..3 {
        for side in [false, true] {
            let mut normal = Vec3::ZERO;
            normal[axis] = if side { 1.0 } else { -1.0 };
            let face = |a: bool, b: bool| match axis {
                0 => corner(side, a, b),
                1 => corner(a, side, b),
                _ => corner(a, b, side),
            };
            let lit = shade(color, light(normal));
            for (a, b) in [(false, false), (true, false), (true, true), (false, false), (true, true), (false, true)] {
                vertices.push(DeckVertex { position: face(a, b), color: lit });
            }
        }
    }
}
