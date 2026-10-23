//! One mesh per part on the parts list (`connector::parts::Part`), in metres
//! and in the part's own frame. The connector renderer draws each mesh once,
//! instanced at every placement.

use crate::connector::fork::{
    ClevisDimensions, BRIDLE_LINK, CHEEK_GAP, CHEEK_RADIUS, CHEEK_THICKNESS, CLEVIS, EYE_OFFSET,
    EYE_RADIUS, SADDLE_RADIUS, TUBE_LENGTH, TUBE_RADIUS, WIRE_RADIUS,
};
use crate::connector::parts::Part;
use crate::units::Unit;
use crate::wgpu::Wgpu;
use glam::{Vec2, Vec3};
use std::f32::consts::{PI, TAU};
use wgpu::util::DeviceExt;

const ROUND_SEGMENTS: usize = 24;
const TIP_SEGMENTS: usize = 16;
const RING_WIRE_SEGMENTS: usize = 6;
// Neighbouring side walls closer than this share a smoothed normal.
const SMOOTH_ANGLE_COS: f32 = 0.85;
// Ring plate proportions, in ring radii — see docs/connectors.md, "The
// Fabricated Part".
const BOSS_REACH: f32 = 1.2; // R_flat / (D_ring/2) = 24/20
const BOSS_HALF_WIDTH: f32 = 0.3; // (w_boss/2) / (D_ring/2) = 6/20
const RING_ARC_SEGMENTS: usize = 48;
// The shackle is drawn as a rod; its collision envelope is `fork::SHACKLE_RADIUS`.
const SHACKLE_ROD_RADIUS: f32 = 0.005;

/// The sizes connector parts take from the fabric they're built for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartSizes {
    pub ring_radius: f32,
    pub pivot_radius: f32,
    pub ring_thickness: f32,
    pub washer_thickness: f32,
    pub cap_thickness: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
}

#[derive(Default)]
struct Mesh {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl Mesh {
    fn vertex(&mut self, position: Vec3, normal: Vec3) -> u32 {
        self.vertices.push(Vertex {
            position: position.into(),
            normal: normal.normalize().into(),
            uv: [0.0, 0.0],
        });
        self.vertices.len() as u32 - 1
    }

    /// Wound so its front face agrees with its vertex normals, whatever
    /// order the caller lists the corners in.
    fn triangle(&mut self, a: u32, b: u32, c: u32) {
        let [pa, pb, pc] = [a, b, c].map(|i| Vec3::from(self.vertices[i as usize].position));
        let normals: Vec3 = [a, b, c]
            .map(|i| Vec3::from(self.vertices[i as usize].normal))
            .into_iter()
            .sum();
        if (pb - pa).cross(pc - pa).dot(normals) >= 0.0 {
            self.indices.extend([a, b, c]);
        } else {
            self.indices.extend([a, c, b]);
        }
    }

    fn quad(&mut self, a: u32, b: u32, c: u32, d: u32) {
        self.triangle(a, b, c);
        self.triangle(a, c, d);
    }

    /// Prism of a convex outline in the x/z plane, from y0 to y1. Side walls
    /// meeting at a shallow angle shade smooth, sharper corners stay crisp.
    fn extrude(&mut self, outline: &[Vec2], y0: f32, y1: f32) {
        let n = outline.len();
        let centroid = outline.iter().copied().sum::<Vec2>() / n as f32;
        let at = |p: Vec2, y: f32| Vec3::new(p.x, y, p.y);
        let edge_normal = |i: usize| {
            let d = outline[(i + 1) % n] - outline[i];
            let normal = Vec2::new(d.y, -d.x).normalize();
            if normal.dot(outline[i] - centroid) < 0.0 {
                -normal
            } else {
                normal
            }
        };
        for i in 0..n {
            let j = (i + 1) % n;
            let here = edge_normal(i);
            let blend = |neighbour: Vec2| {
                if here.dot(neighbour) > SMOOTH_ANGLE_COS {
                    (here + neighbour).normalize()
                } else {
                    here
                }
            };
            let normal_i = blend(edge_normal((i + n - 1) % n));
            let normal_j = blend(edge_normal(j));
            let corners = [
                self.vertex(at(outline[i], y0), at(normal_i, 0.0)),
                self.vertex(at(outline[j], y0), at(normal_j, 0.0)),
                self.vertex(at(outline[j], y1), at(normal_j, 0.0)),
                self.vertex(at(outline[i], y1), at(normal_i, 0.0)),
            ];
            self.quad(corners[0], corners[1], corners[2], corners[3]);
        }
        for (y, normal) in [(y0, -Vec3::Y), (y1, Vec3::Y)] {
            let center = self.vertex(at(centroid, y), normal);
            let rim: Vec<u32> = outline.iter().map(|p| self.vertex(at(*p, y), normal)).collect();
            for i in 0..n {
                self.triangle(center, rim[i], rim[(i + 1) % n]);
            }
        }
    }

    /// Surface of revolution about `axis` through the origin: `profile` is
    /// (distance along the axis, radius). Each profile segment is its own
    /// band, so steps and tapers keep crisp edges; both ends are capped.
    fn lathe(&mut self, axis: Vec3, profile: &[(f32, f32)]) {
        self.lathe_at(Vec3::ZERO, axis, profile);
    }

    /// `lathe` about an axis through `origin`.
    fn lathe_at(&mut self, origin: Vec3, axis: Vec3, profile: &[(f32, f32)]) {
        let (u, v) = axis.any_orthonormal_pair();
        let around = |k: usize| {
            let a = TAU * k as f32 / ROUND_SEGMENTS as f32;
            u * a.cos() + v * a.sin()
        };
        for pair in profile.windows(2) {
            let [(t0, r0), (t1, r1)] = [pair[0], pair[1]];
            let rings: Vec<[u32; 2]> = (0..ROUND_SEGMENTS)
                .map(|k| {
                    let radial = around(k);
                    // Outward normal of the profile line in (axial, radial).
                    let normal = (radial * (t1 - t0) - axis * (r1 - r0)).normalize();
                    [
                        self.vertex(origin + axis * t0 + radial * r0, normal),
                        self.vertex(origin + axis * t1 + radial * r1, normal),
                    ]
                })
                .collect();
            for k in 0..ROUND_SEGMENTS {
                let [a, b] = rings[k];
                let [d, c] = rings[(k + 1) % ROUND_SEGMENTS];
                self.quad(a, b, c, d);
            }
        }
        let ends = [(profile[0], -axis), (profile[profile.len() - 1], axis)];
        for ((t, r), normal) in ends {
            let center = self.vertex(origin + axis * t, normal);
            let rim: Vec<u32> = (0..ROUND_SEGMENTS)
                .map(|k| self.vertex(origin + axis * t + around(k) * r, normal))
                .collect();
            for k in 0..ROUND_SEGMENTS {
                self.triangle(center, rim[k], rim[(k + 1) % ROUND_SEGMENTS]);
            }
        }
    }

    /// Ring of wire about `center`, lying in the plane normal to `axis`.
    fn torus(&mut self, center: Vec3, axis: Vec3, radius: f32, wire: f32) {
        let (u, v) = axis.any_orthonormal_pair();
        let grid: Vec<Vec<u32>> = (0..ROUND_SEGMENTS)
            .map(|i| {
                let a = TAU * i as f32 / ROUND_SEGMENTS as f32;
                let radial = u * a.cos() + v * a.sin();
                (0..RING_WIRE_SEGMENTS)
                    .map(|j| {
                        let b = TAU * j as f32 / RING_WIRE_SEGMENTS as f32;
                        let normal = radial * b.cos() + axis * b.sin();
                        self.vertex(center + radial * radius + normal * wire, normal)
                    })
                    .collect()
            })
            .collect();
        for i in 0..ROUND_SEGMENTS {
            let next_i = (i + 1) % ROUND_SEGMENTS;
            for j in 0..RING_WIRE_SEGMENTS {
                let next_j = (j + 1) % RING_WIRE_SEGMENTS;
                self.quad(grid[i][j], grid[next_i][j], grid[next_i][next_j], grid[i][next_j]);
            }
        }
    }
}

impl Wgpu {
    /// A part's mesh, in metres in its own frame (see `Part`), drawn by
    /// `plate_vertex` at unit scale.
    pub fn create_part(&self, part: Part, sizes: PartSizes) -> (wgpu::Buffer, wgpu::Buffer, u32) {
        let mut mesh = Mesh::default();
        match part {
            Part::Cap => mesh.lathe(Vec3::Y, &[(0.0, sizes.ring_radius), (sizes.cap_thickness, sizes.ring_radius)]),
            Part::Ring => {
                let boss = BOSS_REACH * sizes.ring_radius;
                ring(&mut mesh, sizes.ring_radius, sizes.ring_thickness, &[(0.0, boss)]);
            }
            Part::CustomDisc(shape) => {
                // A boss ends as far short of its pivot as a standard ring's.
                let short_of_pivot = sizes.pivot_radius - BOSS_REACH * sizes.ring_radius;
                let bosses: Vec<(f32, f32)> = shape
                    .bosses()
                    .iter()
                    .map(|&(degrees, reach)| ((degrees as f32).to_radians(), reach as f32 / 1000.0 - short_of_pivot))
                    .collect();
                ring(&mut mesh, sizes.ring_radius, sizes.ring_thickness, &bosses);
            }
            Part::Spacer { washers } => {
                let height = washers as f32 * sizes.washer_thickness;
                mesh.lathe(Vec3::Y, &[(0.0, sizes.ring_radius), (height, sizes.ring_radius)]);
            }
            Part::CrossTube => {
                let half = TUBE_LENGTH / 2.0;
                mesh.lathe(Vec3::Y, &[(-half, TUBE_RADIUS), (half, TUBE_RADIUS)]);
            }
            Part::Clevis => clevis(&mut mesh, &CLEVIS),
            Part::Shackle => mesh.lathe(Vec3::X, &[(0.0, SHACKLE_ROD_RADIUS), (BRIDLE_LINK.f32(), SHACKLE_ROD_RADIUS)]),
            Part::Clamp => clamp(&mut mesh),
        }
        self.mesh_buffers(&part.name(), &mesh)
    }

    fn mesh_buffers(&self, name: &str, mesh: &Mesh) -> (wgpu::Buffer, wgpu::Buffer, u32) {
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{name} Vertex Buffer")),
                contents: bytemuck::cast_slice(&mesh.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{name} Index Buffer")),
                contents: bytemuck::cast_slice(&mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        (vertex_buffer, index_buffer, mesh.indices.len() as u32)
    }
}

/// A flat disc with a boss at each of `bosses`: (angle from +x in the x/z
/// plane, ascending; distance from the centre to the boss's end), with
/// `thickness` through y.
fn ring(mesh: &mut Mesh, radius: f32, thickness: f32, bosses: &[(f32, f32)]) {
    let theta = BOSS_HALF_WIDTH.asin();
    let mut outline: Vec<Vec2> = Vec::new();
    for (i, &(at, end)) in bosses.iter().enumerate() {
        let toward = Vec2::new(at.cos(), at.sin());
        let side = Vec2::new(-at.sin(), at.cos()) * BOSS_HALF_WIDTH * radius;
        outline.push(toward * end - side);
        outline.push(toward * end + side);
        // Rim from this boss round to the next.
        let next = bosses.get(i + 1).map_or(bosses[0].0 + TAU, |&(angle, _)| angle);
        let (from, to) = (at + theta, next - theta);
        let segments = ((to - from) / TAU * RING_ARC_SEGMENTS as f32).ceil().max(1.0) as usize;
        outline.extend((0..=segments).map(|k| {
            let a = from + (to - from) * k as f32 / segments as f32;
            Vec2::new(a.cos(), a.sin()) * radius
        }));
    }
    mesh.extrude(&outline, -thickness / 2.0, thickness / 2.0);
}

/// The bridle clamp, saddle centre at the origin, eye along +x, cheeks
/// square to y: two cheek plates round the saddle and the eye, the grooved
/// saddle spool between them (drawn at the groove's floor), the keeper
/// pressing the wire into the groove on the eye side with two bolts, and
/// the shackle pin through the eye.
fn clamp(mesh: &mut Mesh) {
    let gap = CHEEK_GAP.f32() / 2.0;
    let outer = gap + CHEEK_THICKNESS.f32();
    let eye = EYE_OFFSET.f32();
    let cheek = cheek_outline(CHEEK_RADIUS.f32(), eye, EYE_RADIUS.f32());
    mesh.extrude(&cheek, gap, outer);
    mesh.extrude(&cheek, -outer, -gap);

    let groove_floor = SADDLE_RADIUS.f32() - WIRE_RADIUS.f32();
    mesh.lathe(Vec3::Y, &[(-gap, groove_floor), (gap, groove_floor)]);

    // Keeper block over the wire, between the cheeks.
    let wire_top = SADDLE_RADIUS.f32() + WIRE_RADIUS.f32();
    let keeper_top = (eye - EYE_RADIUS.f32() / 2.0).max(wire_top + 0.004);
    let half_width = 0.010;
    let keeper = [
        Vec2::new(wire_top, -half_width),
        Vec2::new(keeper_top, -half_width),
        Vec2::new(keeper_top, half_width),
        Vec2::new(wire_top, half_width),
    ];
    mesh.extrude(&keeper, -gap, gap);

    // Keeper bolts and the shackle pin, through both cheeks.
    let through = outer + 0.003;
    let bolt_x = (wire_top + keeper_top) / 2.0;
    for z in [-half_width * 0.6, half_width * 0.6] {
        mesh.lathe_at(Vec3::new(bolt_x, 0.0, z), Vec3::Y, &[(-through, 0.003), (through, 0.003)]);
    }
    mesh.lathe_at(Vec3::new(eye, 0.0, 0.0), Vec3::Y, &[(-through, 0.005), (through, 0.005)]);
}

/// The convex outline round a disc of `radius` at the origin and a lobe
/// of `lobe` round a point `reach` along +x, in the x/z plane.
fn cheek_outline(radius: f32, reach: f32, lobe: f32) -> Vec<Vec2> {
    let circle = |center: Vec2, r: f32, n: usize| {
        (0..n).map(move |k| center + Vec2::from_angle(TAU * k as f32 / n as f32) * r)
    };
    let mut points: Vec<Vec2> = circle(Vec2::ZERO, radius, RING_ARC_SEGMENTS)
        .chain(circle(Vec2::new(reach, 0.0), lobe, ROUND_SEGMENTS))
        .collect();
    // Convex hull by monotone chain, counter-clockwise.
    points.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    let cross = |o: Vec2, a: Vec2, b: Vec2| (a - o).perp_dot(b - o);
    let mut hull: Vec<Vec2> = Vec::new();
    for pass in [points.clone(), points.iter().rev().copied().collect()] {
        let start = hull.len();
        for p in pass {
            while hull.len() >= start + 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0 {
                hull.pop();
            }
            hull.push(p);
        }
        hull.pop();
    }
    hull
}

/// The whole jaw terminal with its pin and split ring: pin centre at the
/// origin, +x along the cable toward the barrel, +y along the pin toward
/// its head.
fn clevis(mesh: &mut Mesh, c: &ClevisDimensions) {
    let gap_half = c.jaw_gap.f32() / 2.0;
    let jaw_half = c.jaw_half_thickness();
    let width_half = c.plate_width.f32() / 2.0;
    let tip = c.pin_to_tip.f32();
    let jaw_base = c.pin_to_jaw_base.f32();
    let block_end = c.jaw_block_end();
    let length = c.overall_length.f32();

    // Jaw plates: rounded tip around the pin, running back into the
    // solid block. The block between them closes the slot at the jaw base.
    let flank = (width_half / tip).min(1.0).asin();
    let mut plate: Vec<Vec2> = (0..=TIP_SEGMENTS)
        .map(|i| {
            let a = PI - flank + 2.0 * flank * i as f32 / TIP_SEGMENTS as f32;
            Vec2::new(a.cos(), a.sin()) * tip
        })
        .collect();
    plate.push(Vec2::new(block_end, plate[TIP_SEGMENTS].y));
    plate.push(Vec2::new(block_end, plate[0].y));
    mesh.extrude(&plate, gap_half, jaw_half);
    mesh.extrude(&plate, -jaw_half, -gap_half);
    let block = [
        Vec2::new(jaw_base, -width_half),
        Vec2::new(block_end, -width_half),
        Vec2::new(block_end, width_half),
        Vec2::new(jaw_base, width_half),
    ];
    mesh.extrude(&block, -gap_half, gap_half);

    // Neck and swaged barrel, tapering where the wire enters.
    mesh.lathe(
        Vec3::X,
        &[
            (block_end, jaw_half.min(width_half)),
            (block_end + c.neck_length.f32(), c.barrel_radius.f32()),
            (length - c.barrel_taper.f32(), c.barrel_radius.f32()),
            (length, c.barrel_tip_radius.f32()),
        ],
    );

    // Headed pin, overhanging the far jaw for the split ring's cross-hole.
    let far_end = -jaw_half - c.pin_overhang.f32();
    mesh.lathe(
        Vec3::Y,
        &[
            (far_end, c.pin_radius.f32()),
            (jaw_half, c.pin_radius.f32()),
            (jaw_half, c.pin_head_radius.f32()),
            (jaw_half + c.pin_head_height.f32(), c.pin_head_radius.f32()),
        ],
    );

    // Split ring through the cross-hole, lying flat against the jaw.
    let ring_radius = c.split_ring_radius.f32();
    mesh.torus(
        Vec3::new(ring_radius, -jaw_half - c.pin_overhang.f32() / 2.0, 0.0),
        Vec3::Y,
        ring_radius,
        c.split_ring_wire.f32(),
    );
}
