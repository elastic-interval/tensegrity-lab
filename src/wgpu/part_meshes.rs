//! One mesh per part on the parts list (`connector::parts::Part`), in metres
//! and in the part's own frame. The connector renderer draws each mesh once,
//! instanced at every placement.

use crate::connector::fork::{
    ClevisDimensions, BOSS_SHORT_OF_PIVOT, CLEVIS, HINGE_BALL_WIDTH, HINGE_EYE_RADIUS, HINGE_EYE_WIDTH, HINGE_JAW_GAP,
    HINGE_JAW_THICKNESS, HINGE_LENGTH, HINGE_PIN_RADIUS, HINGE_SHANK_RADIUS, HINGE_THREAD_DEPTH, HINGE_THREAD_RADIUS,
    FORK_HOLE_DIAMETER, FORK_NUT_HEIGHT, FORK_NUT_RADIUS, FORK_WEB_OUTSIDE, PLATE_CORNER_RADIUS, PLATE_HOLE_DIAMETER, PLATE_THICKNESS, RING_BORE_DIAMETER,
    ROD_FORK, TUBE_BORE_RADIUS, TUBE_LENGTH,
    TUBE_RADIUS,
};
use crate::connector::dimensions::ConnectorDimensions;
use crate::connector::parts::Part;
use crate::connector::strut::{INNER_BORE_RADIUS, INNER_RADIUS, OUTER_BORE_RADIUS, OUTER_RADIUS};
use crate::fabric::FabricDimensions;
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
const BOSS_HALF_WIDTH: f32 = 0.3; // (w_boss/2) / (D_ring/2) = 6/20
const RING_ARC_SEGMENTS: usize = 48;
// The cable's hole in a terminal's barrel, 6 mm wire.
const BARREL_BORE_RADIUS: f32 = 0.003;

/// A part's mesh on the GPU, with the sphere bounding it in its own frame.
pub struct PartMesh {
    pub vertices: wgpu::Buffer,
    pub indices: wgpu::Buffer,
    pub index_count: u32,
    pub center: Vec3,
    pub radius: f32,
}

/// The sizes connector parts take from the fabric they're built for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartSizes {
    pub ring_radius: f32,
    pub pivot_radius: f32,
    pub ring_thickness: f32,
    pub washer_thickness: f32,
}

impl PartSizes {
    pub fn of(fabric: &FabricDimensions, connector: &ConnectorDimensions) -> Self {
        PartSizes {
            ring_radius: fabric.push_radius.f32(),
            pivot_radius: connector.pivot_radius.f32(),
            ring_thickness: connector.ring_thickness.f32(),
            washer_thickness: connector.washer_thickness.f32(),
        }
    }
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
    /// The centre of the mesh's bounding box and the distance from it to the
    /// farthest vertex.
    fn bounds(&self) -> (Vec3, f32) {
        let points = self.vertices.iter().map(|vertex| Vec3::from(vertex.position));
        let (min, max) = points
            .clone()
            .fold((Vec3::MAX, Vec3::MIN), |(min, max), p| (min.min(p), max.max(p)));
        let center = (min + max) / 2.0;
        let radius = points.map(|p| p.distance(center)).fold(0.0, f32::max);
        (center, radius)
    }

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
        self.extrude_sides(outline, y0, y1);
        let n = outline.len();
        let centroid = outline.iter().copied().sum::<Vec2>() / n as f32;
        let at = |p: Vec2, y: f32| Vec3::new(p.x, y, p.y);
        for (y, normal) in [(y0, -Vec3::Y), (y1, Vec3::Y)] {
            let center = self.vertex(at(centroid, y), normal);
            let rim: Vec<u32> = outline.iter().map(|p| self.vertex(at(*p, y), normal)).collect();
            for i in 0..n {
                self.triangle(center, rim[i], rim[(i + 1) % n]);
            }
        }
    }

    /// `extrude` with round holes through it, each (centre, radius).
    fn extrude_with_holes(&mut self, outline: &[Vec2], holes: &[(Vec2, f32)], y0: f32, y1: f32) {
        self.extrude_sides(outline, y0, y1);
        let at = |p: Vec2, y: f32| Vec3::new(p.x, y, p.y);
        let circles: Vec<Vec<Vec2>> = holes.iter().map(|&(center, radius)| circle(center, radius)).collect();
        for (circle, &(center, _)) in circles.iter().zip(holes) {
            for i in 0..circle.len() {
                let (p, q) = (circle[i], circle[(i + 1) % circle.len()]);
                let corners = [
                    self.vertex(at(p, y0), at(center - p, 0.0)),
                    self.vertex(at(q, y0), at(center - q, 0.0)),
                    self.vertex(at(q, y1), at(center - q, 0.0)),
                    self.vertex(at(p, y1), at(center - p, 0.0)),
                ];
                self.quad(corners[0], corners[1], corners[2], corners[3]);
            }
        }
        let triangles = triangulate(outline, &circles);
        for (y, normal) in [(y0, -Vec3::Y), (y1, Vec3::Y)] {
            for corners in &triangles {
                let [a, b, c] = corners.map(|p| self.vertex(at(p, y), normal));
                self.triangle(a, b, c);
            }
        }
    }

    /// The side walls of `extrude`.
    fn extrude_sides(&mut self, outline: &[Vec2], y0: f32, y1: f32) {
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
    }

    /// Surface of revolution about `axis` through the origin: `profile` is
    /// (distance along the axis, radius). Each profile segment is its own
    /// band, so steps and tapers keep crisp edges; both ends are capped.
    fn lathe(&mut self, axis: Vec3, profile: &[(f32, f32)]) {
        self.lathe_at(Vec3::ZERO, axis, profile);
    }

    /// `lathe` about an axis through `origin`.
    fn lathe_at(&mut self, origin: Vec3, axis: Vec3, profile: &[(f32, f32)]) {
        self.lathe_bands(origin, axis, profile);
        let (u, v) = axis.any_orthonormal_pair();
        let around = |k: usize| {
            let a = TAU * k as f32 / ROUND_SEGMENTS as f32;
            u * a.cos() + v * a.sin()
        };
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

    /// A tube about `axis` through `origin`, from `t0` to `t1` along it.
    fn hollow(&mut self, origin: Vec3, axis: Vec3, t0: f32, t1: f32, outer: f32, inner: f32) {
        self.lathe_bands(origin, axis, &[(t0, inner), (t0, outer), (t1, outer), (t1, inner), (t0, inner)]);
    }

    /// The bands of `lathe_at`, without its end caps.
    fn lathe_bands(&mut self, origin: Vec3, axis: Vec3, profile: &[(f32, f32)]) {
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
    }

    /// A hex nut about the x axis from `t0` to `t1`, its flats square to y so
    /// that it sits between a fork's jaws.
    fn nut(&mut self, t0: f32, t1: f32) {
        let corner = |k: usize| Vec2::from_angle(TAU * (k as f32 + 0.5) / 6.0) * FORK_NUT_RADIUS.f32();
        let at = |t: f32, p: Vec2| Vec3::new(t, p.x, p.y);
        for k in 0..6 {
            let (p, q) = (corner(k), corner(k + 1));
            let normal = at(0.0, p + q);
            let corners = [
                self.vertex(at(t0, p), normal),
                self.vertex(at(t0, q), normal),
                self.vertex(at(t1, q), normal),
                self.vertex(at(t1, p), normal),
            ];
            self.quad(corners[0], corners[1], corners[2], corners[3]);
        }
        for (t, normal) in [(t0, -Vec3::X), (t1, Vec3::X)] {
            let center = self.vertex(at(t, Vec2::ZERO), normal);
            for k in 0..6 {
                let (p, q) = (self.vertex(at(t, corner(k)), normal), self.vertex(at(t, corner(k + 1)), normal));
                self.triangle(center, p, q);
            }
        }
    }

    /// `other` with its x and y swapped, so an extrusion along y runs along x.
    fn append_along_x(&mut self, other: &Mesh) {
        let swapped = |[x, y, z]: [f32; 3]| [y, x, z];
        let offset = self.vertices.len() as u32;
        self.vertices.extend(other.vertices.iter().map(|vertex| Vertex {
            position: swapped(vertex.position),
            normal: swapped(vertex.normal),
            uv: vertex.uv,
        }));
        // Swapping two axes mirrors the mesh, so each triangle turns over.
        for triangle in other.indices.chunks(3) {
            self.indices.extend([triangle[0] + offset, triangle[2] + offset, triangle[1] + offset]);
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
    pub fn create_part(&self, part: Part, sizes: PartSizes) -> PartMesh {
        let mut mesh = Mesh::default();
        match part {
            Part::Ring => {
                let boss = sizes.pivot_radius - BOSS_SHORT_OF_PIVOT.f32();
                ring(&mut mesh, sizes.ring_radius, sizes.ring_thickness, &[(0.0, boss)]);
                cross_tube(&mut mesh, 0.0, sizes.pivot_radius);
            }
            Part::CustomDisc(shape) => {
                let short_of_pivot = BOSS_SHORT_OF_PIVOT.f32();
                let bosses: Vec<(f32, f32)> = shape
                    .bosses()
                    .iter()
                    .map(|&(degrees, reach)| ((degrees as f32).to_radians(), reach as f32 / 1000.0 - short_of_pivot))
                    .collect();
                ring(&mut mesh, sizes.ring_radius, sizes.ring_thickness, &bosses);
                for &(degrees, reach) in shape.bosses() {
                    cross_tube(&mut mesh, (degrees as f32).to_radians(), reach as f32 / 1000.0);
                }
            }
            Part::Washer => {
                let height = sizes.washer_thickness;
                mesh.hollow(Vec3::ZERO, Vec3::Y, 0.0, height, sizes.ring_radius, RING_BORE_DIAMETER.f32() / 2.0);
            }
            Part::Clevis => clevis(&mut mesh, &CLEVIS),
            Part::Hinge => hinge_clevis(&mut mesh),
            Part::HingeEye => hinge_rod_end(&mut mesh),
            Part::RodFork => rod_fork(&mut mesh, &ROD_FORK),
            Part::ForkNuts => {
                let (web, height) = (ROD_FORK.pin_to_jaw_base.f32(), FORK_NUT_HEIGHT.f32());
                mesh.nut(web - height, web);
                mesh.nut(FORK_WEB_OUTSIDE.f32(), FORK_WEB_OUTSIDE.f32() + height);
            }
            Part::NodePlate(shape) => {
                let centers: Vec<Vec2> = shape
                    .holes
                    .iter()
                    .map(|&(degrees, mm)| Vec2::from_angle((degrees as f32).to_radians()) * (mm as f32 / 1000.0))
                    .collect();
                let corners: Vec<(Vec2, f32)> = centers.iter().map(|&c| (c, PLATE_CORNER_RADIUS.f32())).collect();
                let holes: Vec<(Vec2, f32)> = centers.iter().map(|&c| (c, PLATE_HOLE_DIAMETER.f32() / 2.0)).collect();
                let half = PLATE_THICKNESS.f32() / 2.0;
                mesh.extrude_with_holes(&hull_of_circles(&corners), &holes, -half, half);
            }
            Part::OuterTube(mm) => {
                mesh.hollow(Vec3::ZERO, Vec3::X, 0.0, mm as f32 / 1000.0, OUTER_RADIUS.f32(), OUTER_BORE_RADIUS.f32())
            }
            Part::InnerTube(mm) => {
                mesh.hollow(Vec3::ZERO, Vec3::X, 0.0, mm as f32 / 1000.0, INNER_RADIUS.f32(), INNER_BORE_RADIUS.f32())
            }
        }
        let (vertices, indices, index_count) = self.mesh_buffers(&part.name(), &mesh);
        let (center, radius) = mesh.bounds();
        PartMesh { vertices, indices, index_count, center, radius }
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

/// The cross-tube welded across the end of a boss at angle `at` from +x in
/// the x/z plane, centred on the pivot `reach` out, square to the boss.
fn cross_tube(mesh: &mut Mesh, at: f32, reach: f32) {
    let toward = Vec3::new(at.cos(), 0.0, at.sin());
    let along = Vec3::new(-at.sin(), 0.0, at.cos());
    let half = TUBE_LENGTH / 2.0;
    mesh.hollow(toward * reach, along, -half, half, TUBE_RADIUS, TUBE_BORE_RADIUS);
}

/// A flat disc with a boss at each of `bosses`: (angle from +x in the x/z
/// plane, ascending; distance from the centre to the boss's end), with
/// `thickness` through y.
fn ring(mesh: &mut Mesh, radius: f32, thickness: f32, bosses: &[(f32, f32)]) {
    let bore = [(Vec2::ZERO, RING_BORE_DIAMETER.f32() / 2.0)];
    mesh.extrude_with_holes(&ring_outline(radius, bosses), &bore, -thickness / 2.0, thickness / 2.0);
}

/// The outline of `ring`.
fn ring_outline(radius: f32, bosses: &[(f32, f32)]) -> Vec<Vec2> {
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
    outline
}

/// A round hole's edge, as a polygon.
fn circle(center: Vec2, radius: f32) -> Vec<Vec2> {
    (0..ROUND_SEGMENTS).map(|k| center + Vec2::from_angle(TAU * k as f32 / ROUND_SEGMENTS as f32) * radius).collect()
}

/// Twice a polygon's area, positive when counter-clockwise.
fn doubled_area(polygon: &[Vec2]) -> f32 {
    (0..polygon.len()).map(|i| polygon[i].perp_dot(polygon[(i + 1) % polygon.len()])).sum()
}

/// Triangles covering `outline` but not `holes`: each hole is joined to the
/// outline by a bridge between their closest corners, making one polygon, whose
/// ears are then clipped.
fn triangulate(outline: &[Vec2], holes: &[Vec<Vec2>]) -> Vec<[Vec2; 3]> {
    let mut polygon = outline.to_vec();
    if doubled_area(&polygon) < 0.0 {
        polygon.reverse();
    }
    for hole in holes {
        let mut hole = hole.clone();
        if doubled_area(&hole) > 0.0 {
            hole.reverse();
        }
        let distance = |&(i, j): &(usize, usize)| polygon[i].distance_squared(hole[j]);
        let pairs = (0..polygon.len()).flat_map(|i| (0..hole.len()).map(move |j| (i, j)));
        let (i, j) = pairs.min_by(|a, b| distance(a).total_cmp(&distance(b))).expect("polygons have corners");
        let bridged: Vec<Vec2> = polygon[..=i]
            .iter()
            .chain(&hole[j..])
            .chain(&hole[..=j])
            .chain(&polygon[i..])
            .copied()
            .collect();
        polygon = bridged;
    }
    let mut triangles = Vec::new();
    while polygon.len() >= 3 {
        let n = polygon.len();
        let corners = |i: usize| [polygon[(i + n - 1) % n], polygon[i], polygon[(i + 1) % n]];
        let inside = |[a, b, c]: [Vec2; 3], p: Vec2| {
            (b - a).perp_dot(p - a) >= 0.0 && (c - b).perp_dot(p - b) >= 0.0 && (a - c).perp_dot(p - c) >= 0.0
        };
        let is_ear = |i: usize| {
            let [a, b, c] = corners(i);
            (b - a).perp_dot(c - b) > 0.0
                && polygon.iter().all(|&p| p == a || p == b || p == c || !inside([a, b, c], p))
        };
        let Some(ear) = (0..n).find(|&i| is_ear(i)) else {
            break;
        };
        triangles.push(corners(ear));
        polygon.remove(ear);
    }
    triangles
}

/// A hinge's clevis and pin: the pin at the origin along y, the jaws round
/// it either side of the rod end's eye, closing into a block and the
/// threaded body down the strut along +x to its base.
fn hinge_clevis(mesh: &mut Mesh) {
    let (gap, plate) = (HINGE_JAW_GAP.f32() / 2.0, HINGE_JAW_THICKNESS.f32());
    let (eye, length) = (HINGE_EYE_RADIUS.f32(), HINGE_LENGTH.f32());
    let block = eye + 0.004;
    let jaw = hull_of_circles(&[(Vec2::ZERO, eye), (Vec2::new(block, 0.0), eye)]);
    mesh.extrude(&jaw, gap, gap + plate);
    mesh.extrude(&jaw, -gap - plate, -gap);
    let bridge = [
        Vec2::new(block, -eye),
        Vec2::new(block + plate, -eye),
        Vec2::new(block + plate, eye),
        Vec2::new(block, eye),
    ];
    mesh.extrude(&bridge, -gap, gap);
    let shank = HINGE_SHANK_RADIUS.f32();
    threaded_body(mesh, block + plate, shank, length);
    let through = gap + plate + 0.003;
    mesh.lathe(Vec3::Y, &[(-through, HINGE_PIN_RADIUS.f32()), (through, HINGE_PIN_RADIUS.f32())]);
}

/// A hinge's rod end: the eye round the pin at the origin along y, its
/// ball standing proud either side, and the threaded body down the strut
/// along +x to its base.
fn hinge_rod_end(mesh: &mut Mesh) {
    let (eye, half) = (HINGE_EYE_RADIUS.f32(), HINGE_EYE_WIDTH.f32() / 2.0);
    mesh.lathe(Vec3::Y, &[(-half, eye), (half, eye)]);
    let ball = HINGE_BALL_WIDTH.f32() / 2.0;
    mesh.lathe(Vec3::Y, &[(-ball, eye * 0.6), (ball, eye * 0.6)]);
    threaded_body(mesh, eye * 0.8, HINGE_SHANK_RADIUS.f32(), HINGE_LENGTH.f32());
}

/// A hinge fitting's body along +x, its base tapped for the strut's bolt.
fn threaded_body(mesh: &mut Mesh, start: f32, radius: f32, length: f32) {
    let (thread, depth) = (HINGE_THREAD_RADIUS.f32(), HINGE_THREAD_DEPTH.f32());
    mesh.lathe(Vec3::X, &[(start, radius), (length, radius), (length, thread), (length - depth, thread)]);
}

/// The convex outline round discs given as (centre, radius), in the x/z
/// plane.
fn hull_of_circles(circles: &[(Vec2, f32)]) -> Vec<Vec2> {
    let mut points: Vec<Vec2> = circles
        .iter()
        .flat_map(|&(center, radius)| {
            (0..ROUND_SEGMENTS).map(move |k| center + Vec2::from_angle(TAU * k as f32 / ROUND_SEGMENTS as f32) * radius)
        })
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
    let block_end = c.jaw_block_end();
    jaw(mesh, c, block_end);
    let jaw_half = c.jaw_half_thickness();
    let width_half = c.plate_width.f32() / 2.0;
    let length = c.overall_length.f32();
    // Neck and swaged barrel, tapering where the wire enters.
    mesh.lathe(
        Vec3::X,
        &[
            (block_end, jaw_half.min(width_half)),
            (block_end + c.neck_length.f32(), c.barrel_radius.f32()),
            (length - c.barrel_taper.f32(), c.barrel_radius.f32()),
            (length, c.barrel_tip_radius.f32()),
            (length, BARREL_BORE_RADIUS),
            (length - c.barrel_length.f32(), BARREL_BORE_RADIUS),
        ],
    );
}

/// A rod fork: a jaw with its pin and split ring, pin centre at the origin along
/// +y toward its head, closed along +x by a web with a hole for the threaded rod.
fn rod_fork(mesh: &mut Mesh, c: &ClevisDimensions) {
    let web_outside = FORK_WEB_OUTSIDE.f32();
    jaw_plates_and_pin(mesh, c, web_outside);
    let (jaw_half, width_half) = (c.jaw_half_thickness(), c.plate_width.f32() / 2.0);
    let outline = [
        Vec2::new(-jaw_half, -width_half),
        Vec2::new(jaw_half, -width_half),
        Vec2::new(jaw_half, width_half),
        Vec2::new(-jaw_half, width_half),
    ];
    let hole = [(Vec2::ZERO, FORK_HOLE_DIAMETER.f32() / 2.0)];
    let mut web = Mesh::default();
    web.extrude_with_holes(&outline, &hole, c.pin_to_jaw_base.f32(), web_outside);
    mesh.append_along_x(&web);
}

/// A jaw with its pin and split ring, as on the jaw terminal: pin centre at
/// the origin along +y toward its head, the solid block out along +x to
/// `block_end`.
fn jaw(mesh: &mut Mesh, c: &ClevisDimensions, block_end: f32) {
    jaw_plates_and_pin(mesh, c, block_end);
    let gap_half = c.jaw_gap.f32() / 2.0;
    let width_half = c.plate_width.f32() / 2.0;
    let jaw_base = c.pin_to_jaw_base.f32();
    let block = [
        Vec2::new(jaw_base, -width_half),
        Vec2::new(block_end, -width_half),
        Vec2::new(block_end, width_half),
        Vec2::new(jaw_base, width_half),
    ];
    mesh.extrude(&block, -gap_half, gap_half);
}

/// A jaw's two plates, running out along +x to `plate_end`, with its pin and
/// split ring.
fn jaw_plates_and_pin(mesh: &mut Mesh, c: &ClevisDimensions, plate_end: f32) {
    let gap_half = c.jaw_gap.f32() / 2.0;
    let jaw_half = c.jaw_half_thickness();
    let width_half = c.plate_width.f32() / 2.0;
    let tip = c.pin_to_tip.f32();

    // Jaw plates: rounded tip around the pin, running back to whatever closes
    // the slot at the jaw base.
    let flank = (width_half / tip).min(1.0).asin();
    let mut plate: Vec<Vec2> = (0..=TIP_SEGMENTS)
        .map(|i| {
            let a = PI - flank + 2.0 * flank * i as f32 / TIP_SEGMENTS as f32;
            Vec2::new(a.cos(), a.sin()) * tip
        })
        .collect();
    plate.push(Vec2::new(plate_end, plate[TIP_SEGMENTS].y));
    plate.push(Vec2::new(plate_end, plate[0].y));
    mesh.extrude(&plate, gap_half, jaw_half);
    mesh.extrude(&plate, -jaw_half, -gap_half);

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

#[cfg(test)]
mod tests {
    use super::*;

    fn covered(triangles: &[[Vec2; 3]]) -> f32 {
        triangles.iter().map(|corners| doubled_area(corners) / 2.0).sum()
    }

    fn assert_covers(outline: &[Vec2], holes: &[Vec<Vec2>]) {
        let holes_area: f32 = holes.iter().map(|hole| doubled_area(hole).abs() / 2.0).sum();
        let expected = doubled_area(outline).abs() / 2.0 - holes_area;
        let area = covered(&triangulate(outline, holes));
        assert!((area - expected).abs() < expected * 1e-4, "covered {area}, expected {expected}");
    }

    #[test]
    fn ring_cap_leaves_the_bore_open() {
        let bore = [circle(Vec2::ZERO, RING_BORE_DIAMETER.f32() / 2.0)];
        assert_covers(&ring_outline(0.02, &[(0.0, 0.022)]), &bore);
        let bosses = [(-2.6, 0.042), (-0.42, 0.032), (0.42, 0.032), (2.6, 0.042)];
        assert_covers(&ring_outline(0.02, &bosses), &bore);
    }

    #[test]
    fn plate_cap_leaves_its_three_holes_open() {
        let centers = [0.0_f32, 120.0, 240.0].map(|degrees| Vec2::from_angle(degrees.to_radians()) * 0.035);
        let corners = centers.map(|c| (c, PLATE_CORNER_RADIUS.f32()));
        let holes = centers.map(|c| circle(c, PLATE_HOLE_DIAMETER.f32() / 2.0));
        assert_covers(&hull_of_circles(&corners), &holes);
    }
}
