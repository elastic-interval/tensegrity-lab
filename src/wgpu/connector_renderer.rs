/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::camera::Pick;
use crate::connector::fork::{
    JAW_CLEARANCE, JAW_NOSE_RADIUS, JAW_REACH_UNITS, JAW_THICKNESS, PIN_PROTRUSION, PIN_RADIUS,
    SHANK_LENGTH, SHANK_RADIUS, SHANK_START, TUBE_LENGTH, TUBE_RADIUS,
};
use crate::fabric::interval::Role;
use crate::fabric::{Fabric, IntervalEnd};
use crate::units::Unit;
use crate::wgpu::Wgpu;
use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use std::mem::size_of;
use wgpu::util::DeviceExt;

// Ring plate proportions, used only for rendering — see docs/connectors.md,
// "The Fabricated Part". Fork and tube dimensions come from `connector::fork`,
// the shared model that also drives culprit marking.
const BOSS_REACH: f32 = 1.2; // R_flat / (D_ring/2) = 24/20, in ring radii
const BOSS_HALF_WIDTH: f32 = 0.3; // (w_boss/2) / (D_ring/2) = 6/20, in ring radii

// Materials (docs/connectors.md, "Materials"): the connector itself — ring,
// boss, cross-tube, cap — is hot-dip galvanized steel; the fork terminal and
// clevis pin are real, bought AISI-316 stainless. Our shader is diffuse-only,
// so the stainless base colour is lifted above the table's PBR value to stand
// in for its shine — the real contrast is roughness, not brightness.
// Connector assemblies marked as collision culprits are painted red instead.
const GALVANIZED_COLOR: [f32; 4] = [0.60, 0.62, 0.64, 1.0];
const STAINLESS_COLOR: [f32; 4] = [0.72, 0.73, 0.75, 1.0];
const OVERLAP_COLOR: [f32; 4] = [0.90, 0.12, 0.12, 1.0];

/// Instance data for a cylinder (cap, cross-tube, pin, shank)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct LinkInstance {
    start: [f32; 3],
    radius: f32,
    end: [f32; 3],
    _padding: u32,
    color: [f32; 4],
}

/// Instance data for an extruded plate: full orientation, because the boss
/// (or jaw) must point toward the cable.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PlateInstance {
    center_radius: [f32; 4],  // centre + x/z scale
    axis_thickness: [f32; 4], // unit thickness axis + thickness
    boss_dir: [f32; 4],       // unit long-axis direction + unused
    color: [f32; 4],
}

/// Everything needed to draw one cable-end connector: the ring with its
/// boss, cross-tube, fork jaws, body, pin, and shank. `culprit` comes from
/// `ConnectorSystem::culprits`, marked once per slot-assignment rebuild.
struct EndConnector {
    ring_center: Vec3,
    pivot: Vec3,
    push_axis: Vec3,
    radial: Vec3,
    tangent: Vec3,
    cable_dir: Vec3,
    culprit: bool,
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

        let dims = &connector.dimensions;
        let ring_radius = fabric.dimensions.push_radius.f32(); // D_ring/2 = cap-plate radius
        let ring_thickness = dims.ring_thickness.f32();

        let cylinder = |start: Vec3, end: Vec3, radius: f32, color: [f32; 4]| LinkInstance {
            start: [start.x, start.y, start.z],
            radius,
            end: [end.x, end.y, end.z],
            _padding: 0,
            color,
        };

        // Pass 1: collect every cable-end connector, and the strut-end caps.
        let mut ends: Vec<EndConnector> = Vec::new();

        for (key, interval) in fabric.intervals.iter() {
            if !interval.has_role(Role::Pushing) {
                continue;
            }
            let alpha_pos = fabric.joints[interval.alpha_key].location;
            let omega_pos = fabric.joints[interval.omega_key].location;
            let push_dir = (omega_pos - alpha_pos).normalize();

            for (end, joint_pos, push_axis) in [
                (IntervalEnd::Alpha, alpha_pos, -push_dir),
                (IntervalEnd::Omega, omega_pos, push_dir),
            ] {
                let Some(connections) = connector.connections(key, end) else {
                    continue;
                };
                let mut occupied = false;

                for (slot_idx, conn_opt) in connections.iter().enumerate() {
                    let Some(connection) = conn_opt else { continue };
                    let Some(pull_interval) = fabric.intervals.get(connection.pull_interval_key)
                    else {
                        continue;
                    };
                    let pull_joint_key = match end {
                        IntervalEnd::Alpha => interval.alpha_key,
                        IntervalEnd::Omega => interval.omega_key,
                    };
                    let far_joint_key = if pull_interval.alpha_key == pull_joint_key {
                        pull_interval.omega_key
                    } else {
                        pull_interval.alpha_key
                    };

                    // Aim at the far end's pivot when attached (not the far
                    // joint) — on short cables the two differ by several
                    // degrees, and the fork must stay collinear with the
                    // cable it holds.
                    let aim = connector
                        .pull_end_pivot(fabric, connection.pull_interval_key, far_joint_key)
                        .unwrap_or(fabric.joints[far_joint_key].location);

                    let (pivot, _elevation) =
                        connector.pivot_geometry(joint_pos, push_axis, slot_idx, aim);
                    let ring_center = connector.ring_center(joint_pos, push_axis, slot_idx);
                    let radial = (pivot - ring_center).normalize();
                    let tangent = push_axis.cross(radial).normalize();
                    let cable_dir = (aim - pivot).normalize();

                    ends.push(EndConnector {
                        ring_center,
                        pivot,
                        push_axis,
                        radial,
                        tangent,
                        cable_dir,
                        culprit: connector
                            .culprits
                            .contains(&(connection.pull_interval_key, pull_joint_key)),
                    });
                    occupied = true;
                }

                if occupied {
                    // Cap: flush continuation of the strut tube, before the
                    // first washer gap.
                    let cap_end = joint_pos + push_axis * dims.cap_thickness.f32();
                    cylinders.push(cylinder(joint_pos, cap_end, ring_radius, GALVANIZED_COLOR));
                }
            }
        }

        // Pass 2: emit every part of every connector, red for the culprits
        // that `ConnectorSystem::mark_culprits` flagged at rebuild time.
        // The connector (ring, boss, tube) is galvanized; the fork terminal
        // and its pin are stainless.
        for e in &ends {
            let (connector_color, fork_color) = if e.culprit {
                (OVERLAP_COLOR, OVERLAP_COLOR)
            } else {
                (GALVANIZED_COLOR, STAINLESS_COLOR)
            };

            // The flat ring with its boss, aimed at the cable
            plates.push(PlateInstance {
                center_radius: [e.ring_center.x, e.ring_center.y, e.ring_center.z, ring_radius],
                axis_thickness: [e.push_axis.x, e.push_axis.y, e.push_axis.z, ring_thickness],
                boss_dir: [e.radial.x, e.radial.y, e.radial.z, 0.0],
                color: connector_color,
            });

            // Cross-tube along the tangent, filling the fork's jaw gap
            let tube_start = e.pivot - e.tangent * (TUBE_LENGTH / 2.0);
            let tube_end = e.pivot + e.tangent * (TUBE_LENGTH / 2.0);
            cylinders.push(cylinder(tube_start, tube_end, TUBE_RADIUS, connector_color));

            // Fork jaws: rounded-nose plates astride the tube with a little
            // air between jaw and tube end (the pivot's slack — no washer),
            // thickness along the tangent, noses wrapping the pin, reaching
            // along the cable toward the shank. Their orientation expresses
            // both the ring's azimuth and the fork's free pivot elevation.
            let jaw_offset = TUBE_LENGTH / 2.0 + JAW_CLEARANCE + JAW_THICKNESS / 2.0;
            for side in [-1.0f32, 1.0] {
                let jaw_center = e.pivot + e.tangent * (side * jaw_offset);
                jaws.push(PlateInstance {
                    center_radius: [jaw_center.x, jaw_center.y, jaw_center.z, JAW_NOSE_RADIUS],
                    axis_thickness: [e.tangent.x, e.tangent.y, e.tangent.z, JAW_THICKNESS],
                    boss_dir: [e.cable_dir.x, e.cable_dir.y, e.cable_dir.z, 0.0],
                    color: fork_color,
                });
            }

            // Fork body: the box joining the jaw tails to the shank, flush
            // with the jaws' outer faces and edges, so the clevis reads as
            // one forged object. Ends flush with the jaw tails; the shank
            // emerges from it.
            let body_center = e.pivot + e.cable_dir * (JAW_NOSE_RADIUS * (JAW_REACH_UNITS - 0.5));
            bodies.push(PlateInstance {
                center_radius: [body_center.x, body_center.y, body_center.z, JAW_NOSE_RADIUS],
                axis_thickness: [
                    e.tangent.x,
                    e.tangent.y,
                    e.tangent.z,
                    2.0 * jaw_offset + JAW_THICKNESS,
                ],
                boss_dir: [e.cable_dir.x, e.cable_dir.y, e.cable_dir.z, 0.0],
                color: fork_color,
            });

            // Clevis pin through jaws and tube, protruding past each jaw
            let half_pin = jaw_offset + JAW_THICKNESS / 2.0 + PIN_PROTRUSION;
            cylinders.push(cylinder(
                e.pivot - e.tangent * half_pin,
                e.pivot + e.tangent * half_pin,
                PIN_RADIUS,
                fork_color,
            ));

            // Swage shank: the cable disappears into it beyond the jaws
            let shank_start = e.pivot + e.cable_dir * SHANK_START;
            let shank_end = shank_start + e.cable_dir * SHANK_LENGTH;
            cylinders.push(cylinder(shank_start, shank_end, SHANK_RADIUS, fork_color));
        }

        (cylinders, plates, jaws, bodies)
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
                render_pass.set_index_buffer(
                    self.cylinder_index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                render_pass.draw_indexed(
                    0..self.cylinder_num_indices,
                    0,
                    0..self.num_cylinder_instances,
                );
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
