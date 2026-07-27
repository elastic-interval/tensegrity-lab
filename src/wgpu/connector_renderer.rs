/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::camera::Pick;
use crate::connector::ConnectorSystem;
use crate::fabric::interval::Role;
use crate::fabric::{Fabric, IntervalEnd, IntervalKey};
use crate::units::Unit;
use crate::wgpu::Wgpu;
use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use std::mem::size_of;
use wgpu::util::DeviceExt;

// Fabricated-part dimensions used only for rendering — see docs/connectors.md,
// "The Fabricated Part".
const BOSS_REACH: f32 = 1.2; // R_flat / (D_ring/2) = 24/20, in ring radii
const BOSS_HALF_WIDTH: f32 = 0.3; // (w_boss/2) / (D_ring/2) = 6/20, in ring radii
const TUBE_RADIUS: f32 = 0.010; // D_tube / 2, metres
const TUBE_LENGTH: f32 = 0.010; // L_tube, metres — wider than the 6 mm cables from every angle

// Fork terminal (clevis) at each cable end — estimated from the site photo
// and catalog swage forks for 6 mm wire at the 8–9 kN load class; exact
// dimensions await Peter's terminal drawings.
const JAW_THICKNESS: f32 = 0.0045;
const JAW_CLEARANCE: f32 = 0.001; // air between each jaw and the tube end — the pivot's slack
const JAW_NOSE_RADIUS: f32 = 0.0095; // jaw outline radius around the pin
const JAW_REACH_UNITS: f32 = 2.2; // jaw length behind the pin, in nose radii
const PIN_RADIUS: f32 = 0.005; // the 10 mm clevis pin
const PIN_PROTRUSION: f32 = 0.0025; // pin visible past each jaw face
const SHANK_RADIUS: f32 = 0.006; // swage shank crimped onto the 6 mm cable
const SHANK_LENGTH: f32 = 0.045;
// The shank starts embedded in the jaw plates so fork and shank read as one
// body instead of just touching.
const SHANK_START: f32 = JAW_NOSE_RADIUS * JAW_REACH_UNITS - 0.006;

/// Where the rendered cable should terminate: buried inside the swage shank,
/// so the cable visibly ends at its terminal rather than running on into the
/// connector's cross-tube. Shared with `cylinder_renderer`.
pub(crate) fn cable_termination(pivot: Vec3, pull_other_end: Vec3) -> Vec3 {
    let dir = (pull_other_end - pivot).normalize();
    pivot + dir * (SHANK_START + SHANK_LENGTH * 0.5)
}

// Steel tones: rings/cap slightly lighter than the cross-tube and pin;
// the stainless fork brighter than both.
const RING_COLOR: [f32; 4] = [0.66, 0.68, 0.72, 1.0];
const ARM_COLOR: [f32; 4] = [0.52, 0.54, 0.58, 1.0];
const FORK_COLOR: [f32; 4] = [0.78, 0.80, 0.83, 1.0];

/// Instance data for a cylinder (cap, cross-tube)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct LinkInstance {
    start: [f32; 3],
    radius: f32,
    end: [f32; 3],
    _padding: u32,
    color: [f32; 4],
}

/// Instance data for a ring+boss plate: full orientation, because the boss
/// must point toward the cable.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PlateInstance {
    center_radius: [f32; 4],  // ring centre + ring radius
    axis_thickness: [f32; 4], // unit strut axis + plate thickness
    boss_dir: [f32; 4],       // unit radial (boss) direction + unused
    color: [f32; 4],
}

pub struct ConnectorRenderer {
    cylinder_vertex_buffer: wgpu::Buffer,
    cylinder_index_buffer: wgpu::Buffer,
    cylinder_num_indices: u32,
    cylinder_pipeline: wgpu::RenderPipeline,
    cylinder_instance_buffer: Option<wgpu::Buffer>,
    num_cylinder_instances: u32,

    plate_vertex_buffer: wgpu::Buffer,
    plate_index_buffer: wgpu::Buffer,
    plate_num_indices: u32,
    plate_pipeline: wgpu::RenderPipeline,
    plate_instance_buffer: Option<wgpu::Buffer>,
    num_plate_instances: u32,

    // Fork jaws share the plate pipeline with their own stadium-profile mesh.
    jaw_vertex_buffer: wgpu::Buffer,
    jaw_index_buffer: wgpu::Buffer,
    jaw_num_indices: u32,
    jaw_instance_buffer: Option<wgpu::Buffer>,
    num_jaw_instances: u32,

    // Fork body: the box joining jaws to shank, also on the plate pipeline.
    body_vertex_buffer: wgpu::Buffer,
    body_index_buffer: wgpu::Buffer,
    body_num_indices: u32,
    body_instance_buffer: Option<wgpu::Buffer>,
    num_body_instances: u32,
}

impl ConnectorRenderer {
    pub fn new(wgpu: &Wgpu) -> Self {
        // Connector parts are seen up close; give them a round silhouette.
        let (cylinder_vertex_buffer, cylinder_index_buffer, cylinder_num_indices) =
            wgpu.create_cylinder(48);

        let cylinder_instance_layout = wgpu::VertexBufferLayout {
            array_stride: size_of::<LinkInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // start position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // radius
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32,
                },
                // end position
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // padding (material_type placeholder)
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 7]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Uint32,
                },
                // color
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 7]>() as wgpu::BufferAddress
                        + size_of::<u32>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        };
        let cylinder_pipeline =
            wgpu.create_fabric_pipeline("Connector Cylinder Pipeline", cylinder_instance_layout);

        let (plate_vertex_buffer, plate_index_buffer, plate_num_indices) =
            wgpu.create_connector_plate(BOSS_REACH, BOSS_HALF_WIDTH);

        let plate_instance_layout = wgpu::VertexBufferLayout {
            array_stride: size_of::<PlateInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        };
        let plate_pipeline =
            wgpu.create_plate_pipeline("Connector Plate Pipeline", plate_instance_layout);

        // A jaw is the same outline family as the ring+boss plate: with a
        // "boss" of half-width 1.0 the arc is exactly a semicircular nose,
        // giving a rounded-nose stadium profile.
        let (jaw_vertex_buffer, jaw_index_buffer, jaw_num_indices) =
            wgpu.create_connector_plate(JAW_REACH_UNITS, 1.0);

        let (body_vertex_buffer, body_index_buffer, body_num_indices) = wgpu.create_box_plate();

        ConnectorRenderer {
            cylinder_vertex_buffer,
            cylinder_index_buffer,
            cylinder_num_indices,
            cylinder_pipeline,
            cylinder_instance_buffer: None,
            num_cylinder_instances: 0,
            plate_vertex_buffer,
            plate_index_buffer,
            plate_num_indices,
            plate_pipeline,
            plate_instance_buffer: None,
            num_plate_instances: 0,
            jaw_vertex_buffer,
            jaw_index_buffer,
            jaw_num_indices,
            jaw_instance_buffer: None,
            num_jaw_instances: 0,
            body_vertex_buffer,
            body_index_buffer,
            body_num_indices,
            body_instance_buffer: None,
            num_body_instances: 0,
        }
    }

    pub fn update(&mut self, wgpu: &Wgpu, fabric: &Fabric, _pick: &Pick) {
        let (cylinders, plates, jaws, bodies) = self.create_instances(fabric);

        self.num_cylinder_instances = cylinders.len() as u32;
        self.cylinder_instance_buffer = (!cylinders.is_empty()).then(|| {
            wgpu.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Connector Cylinder Instance Buffer"),
                    contents: bytemuck::cast_slice(&cylinders),
                    usage: wgpu::BufferUsages::VERTEX,
                })
        });

        self.num_plate_instances = plates.len() as u32;
        self.plate_instance_buffer = (!plates.is_empty()).then(|| {
            wgpu.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Connector Plate Instance Buffer"),
                    contents: bytemuck::cast_slice(&plates),
                    usage: wgpu::BufferUsages::VERTEX,
                })
        });

        self.num_jaw_instances = jaws.len() as u32;
        self.jaw_instance_buffer = (!jaws.is_empty()).then(|| {
            wgpu.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Connector Jaw Instance Buffer"),
                    contents: bytemuck::cast_slice(&jaws),
                    usage: wgpu::BufferUsages::VERTEX,
                })
        });

        self.num_body_instances = bodies.len() as u32;
        self.body_instance_buffer = (!bodies.is_empty()).then(|| {
            wgpu.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Connector Body Instance Buffer"),
                    contents: bytemuck::cast_slice(&bodies),
                    usage: wgpu::BufferUsages::VERTEX,
                })
        });
    }

    fn create_instances(
        &self,
        fabric: &Fabric,
    ) -> (
        Vec<LinkInstance>,
        Vec<PlateInstance>,
        Vec<PlateInstance>,
        Vec<PlateInstance>,
    ) {
        let mut cylinders = Vec::new();
        let mut plates = Vec::new();
        let mut jaws = Vec::new();
        let mut bodies = Vec::new();

        let Some(connector) = fabric.connector.as_ref() else {
            return (cylinders, plates, jaws, bodies);
        };

        // Iterate through all push intervals to find their connections
        for (key, interval) in fabric.intervals.iter() {
            if !interval.has_role(Role::Pushing) {
                continue;
            }

            // Get joint positions
            let alpha_pos = fabric.joints[interval.alpha_key].location;
            let omega_pos = fabric.joints[interval.omega_key].location;
            let push_dir = (omega_pos - alpha_pos).normalize();

            for (end, joint_pos, push_axis) in [
                (IntervalEnd::Alpha, alpha_pos, -push_dir),
                (IntervalEnd::Omega, omega_pos, push_dir),
            ] {
                self.add_connectors_for_end(
                    &mut cylinders,
                    &mut plates,
                    &mut jaws,
                    &mut bodies,
                    fabric,
                    key,
                    interval,
                    end,
                    joint_pos,
                    push_axis,
                    connector,
                );
            }
        }

        (cylinders, plates, jaws, bodies)
    }

    /// One symbolic connector per occupied slot: the flat ring and its boss
    /// as a single plate, the cross-tube as a stubby cylinder along the
    /// tangent, and the cable's fork terminal — two jaws astride the tube,
    /// the clevis pin through them, and the swage shank the cable disappears
    /// into. Washers are left as empty space, so the stack reads as separate
    /// rings; the cap extends the strut tube flush before the first gap.
    fn add_connectors_for_end(
        &self,
        cylinders: &mut Vec<LinkInstance>,
        plates: &mut Vec<PlateInstance>,
        jaws: &mut Vec<PlateInstance>,
        bodies: &mut Vec<PlateInstance>,
        fabric: &Fabric,
        push_key: IntervalKey,
        push_interval: &crate::fabric::interval::Interval,
        end: IntervalEnd,
        joint_pos: Vec3,
        push_axis: Vec3,
        connector: &ConnectorSystem,
    ) {
        let connections = match connector.connections(push_key, end) {
            Some(c) => c,
            None => return,
        };

        // Collect connections: (slot, pivot position, cable aim point). The
        // aim is the far end's pivot when attached (not the far joint) — on
        // short cables the two differ by several degrees, and the fork must
        // stay collinear with the cable it holds.
        let mut slot_connections: Vec<(usize, Vec3, Vec3)> = Vec::new();

        for (slot_idx, conn_opt) in connections.iter().enumerate() {
            if let Some(connection) = conn_opt {
                if let Some(pull_interval) = fabric.intervals.get(connection.pull_interval_key) {
                    let pull_joint_key = match end {
                        IntervalEnd::Alpha => push_interval.alpha_key,
                        IntervalEnd::Omega => push_interval.omega_key,
                    };

                    let far_joint_key = if pull_interval.alpha_key == pull_joint_key {
                        pull_interval.omega_key
                    } else {
                        pull_interval.alpha_key
                    };

                    let aim = connector
                        .pull_end_pivot(fabric, connection.pull_interval_key, far_joint_key)
                        .unwrap_or(fabric.joints[far_joint_key].location);

                    let (pivot_pos, _elevation) =
                        connector.pivot_geometry(joint_pos, push_axis, slot_idx, aim);

                    slot_connections.push((slot_idx, pivot_pos, aim));
                }
            }
        }

        if slot_connections.is_empty() {
            return;
        }

        let dims = &connector.dimensions;
        let ring_radius = fabric.dimensions.push_radius.f32(); // D_ring/2 = cap-plate radius

        let cylinder = |start: Vec3, end: Vec3, radius: f32, color: [f32; 4]| LinkInstance {
            start: [start.x, start.y, start.z],
            radius,
            end: [end.x, end.y, end.z],
            _padding: 0,
            color,
        };

        // Cap: flush continuation of the strut tube, before the first washer gap.
        let cap_end = joint_pos + push_axis * dims.cap_thickness.f32();
        cylinders.push(cylinder(joint_pos, cap_end, ring_radius, RING_COLOR));

        for (slot, pivot_pos, aim) in &slot_connections {
            let ring_center = connector.ring_center(joint_pos, push_axis, *slot);
            let radial = (*pivot_pos - ring_center).normalize();
            let tangent = push_axis.cross(radial).normalize();
            // Direction the cable leaves the pin — always perpendicular to
            // the tangent, so it serves as the fork's long axis.
            let cable_dir = (*aim - *pivot_pos).normalize();

            // The flat ring with its boss, aimed at the cable
            plates.push(PlateInstance {
                center_radius: [ring_center.x, ring_center.y, ring_center.z, ring_radius],
                axis_thickness: [
                    push_axis.x,
                    push_axis.y,
                    push_axis.z,
                    dims.ring_thickness.f32(),
                ],
                boss_dir: [radial.x, radial.y, radial.z, 0.0],
                color: RING_COLOR,
            });

            // Cross-tube along the tangent, filling the fork's jaw gap
            let tube_start = *pivot_pos - tangent * (TUBE_LENGTH / 2.0);
            let tube_end = *pivot_pos + tangent * (TUBE_LENGTH / 2.0);
            cylinders.push(cylinder(tube_start, tube_end, TUBE_RADIUS, ARM_COLOR));

            // Fork jaws: rounded-nose plates astride the tube with a little
            // air between jaw and tube end (the pivot's slack — no washer),
            // thickness along the tangent, noses wrapping the pin, reaching
            // along the cable toward the shank. Their orientation expresses
            // both the ring's azimuth and the fork's free pivot elevation.
            let jaw_offset = TUBE_LENGTH / 2.0 + JAW_CLEARANCE + JAW_THICKNESS / 2.0;
            for side in [-1.0f32, 1.0] {
                let jaw_center = *pivot_pos + tangent * (side * jaw_offset);
                jaws.push(PlateInstance {
                    center_radius: [
                        jaw_center.x,
                        jaw_center.y,
                        jaw_center.z,
                        JAW_NOSE_RADIUS,
                    ],
                    axis_thickness: [tangent.x, tangent.y, tangent.z, JAW_THICKNESS],
                    boss_dir: [cable_dir.x, cable_dir.y, cable_dir.z, 0.0],
                    color: FORK_COLOR,
                });
            }

            // Fork body: the box joining the jaw tails to the shank, flush
            // with the jaws' outer faces and edges, so the clevis reads as
            // one forged object. Ends flush with the jaw tails; the shank
            // emerges from it.
            let body_center = *pivot_pos
                + cable_dir * (JAW_NOSE_RADIUS * (JAW_REACH_UNITS - 0.5));
            bodies.push(PlateInstance {
                center_radius: [body_center.x, body_center.y, body_center.z, JAW_NOSE_RADIUS],
                axis_thickness: [
                    tangent.x,
                    tangent.y,
                    tangent.z,
                    2.0 * jaw_offset + JAW_THICKNESS,
                ],
                boss_dir: [cable_dir.x, cable_dir.y, cable_dir.z, 0.0],
                color: FORK_COLOR,
            });

            // Clevis pin through jaws and tube, protruding past each jaw
            let half_pin = jaw_offset + JAW_THICKNESS / 2.0 + PIN_PROTRUSION;
            cylinders.push(cylinder(
                *pivot_pos - tangent * half_pin,
                *pivot_pos + tangent * half_pin,
                PIN_RADIUS,
                ARM_COLOR,
            ));

            // Swage shank: the cable disappears into it beyond the jaws
            let shank_start = *pivot_pos + cable_dir * SHANK_START;
            let shank_end = shank_start + cable_dir * SHANK_LENGTH;
            cylinders.push(cylinder(shank_start, shank_end, SHANK_RADIUS, FORK_COLOR));
        }
    }

    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        bind_group: &'a wgpu::BindGroup,
    ) {
        if let Some(instance_buffer) = &self.cylinder_instance_buffer {
            if self.num_cylinder_instances > 0 {
                render_pass.set_pipeline(&self.cylinder_pipeline);
                render_pass.set_bind_group(0, bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.cylinder_vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
                render_pass
                    .set_index_buffer(self.cylinder_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.cylinder_num_indices, 0, 0..self.num_cylinder_instances);
            }
        }
        if let Some(instance_buffer) = &self.plate_instance_buffer {
            if self.num_plate_instances > 0 {
                render_pass.set_pipeline(&self.plate_pipeline);
                render_pass.set_bind_group(0, bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.plate_vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
                render_pass
                    .set_index_buffer(self.plate_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.plate_num_indices, 0, 0..self.num_plate_instances);
            }
        }
        if let Some(instance_buffer) = &self.jaw_instance_buffer {
            if self.num_jaw_instances > 0 {
                render_pass.set_pipeline(&self.plate_pipeline);
                render_pass.set_bind_group(0, bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.jaw_vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
                render_pass
                    .set_index_buffer(self.jaw_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.jaw_num_indices, 0, 0..self.num_jaw_instances);
            }
        }
        if let Some(instance_buffer) = &self.body_instance_buffer {
            if self.num_body_instances > 0 {
                render_pass.set_pipeline(&self.plate_pipeline);
                render_pass.set_bind_group(0, bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.body_vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
                render_pass
                    .set_index_buffer(self.body_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.body_num_indices, 0, 0..self.num_body_instances);
            }
        }
    }
}
