use crate::fabric::quake::{
    cable_ends, cam_axis, corner_angle, hook_foot, CRANK, BASE_THICKNESS, DECK_THICKNESS, HANG_CORNERS, HEAD_CLEARANCE,
    HOOK_REACH, TIE_RADIUS,
};
use bytemuck::{Pod, Zeroable};
use glam::Vec3;
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

/// The wobble table, in the deck's own frame: the deck and the hooks
/// hanging under it, which move; the base on the floor, the arms rising from
/// it and the motor in its middle, which stay; the ties between hooks and
/// arms; and the cam's crank and its cables to the hooks, which turn with
/// the cam.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Default)]
pub struct DeckVertex {
    /// Its fourth says what it moves with: one the deck, two the cam's
    /// pin, zero nothing.
    position: [f32; 4],
    color: [f32; 4],
}

const STAYS: f32 = 0.0;
const ON_DECK: f32 = 1.0;
const ON_PIN: f32 = 2.0;

/// Half the width of a hook's or an arm's square tube, of a tie or cable,
/// of the motor and of the crank's pin.
const TUBE: f32 = 0.0125;
const TIE: f32 = 0.003;
const MOTOR: f32 = 0.03;
const PIN: f32 = 0.004;

const DECK_COLOR: [f32; 3] = [0.30, 0.22, 0.15];
const BASE_COLOR: [f32; 3] = [0.12, 0.12, 0.13];
const HOOK_COLOR: [f32; 3] = [0.45, 0.46, 0.5];
const ARM_COLOR: [f32; 3] = [0.25, 0.27, 0.32];
const TIE_COLOR: [f32; 3] = [0.9, 0.85, 0.6];
const CABLE_COLOR: [f32; 3] = [0.95, 0.5, 0.2];
const MOTOR_COLOR: [f32; 3] = [0.1, 0.1, 0.1];

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
        let axis = cam_axis();
        bar(&mut vertices, Vec3::new(0.0, base_top, 0.0), axis, MOTOR, MOTOR_COLOR, (STAYS, STAYS));
        prism(&mut vertices, CRANK * 1.3, axis.y - PIN, axis.y, MOTOR_COLOR, STAYS);
        bar(&mut vertices, axis - Vec3::Y * PIN, axis + Vec3::Y * PIN, PIN, CABLE_COLOR, (ON_PIN, ON_PIN));
        for end in cable_ends(radius) {
            bar(&mut vertices, axis, end, TIE, CABLE_COLOR, (ON_PIN, ON_DECK));
        }
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
