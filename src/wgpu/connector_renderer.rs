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

// Pastel colors for the two link types
const AXIAL_COLOR: [f32; 4] = [1.0, 1.0, 0.6, 1.0]; // Pastel yellow
const RADIAL_COLOR: [f32; 4] = [1.0, 0.8, 0.5, 1.0]; // Pastel orange

/// Instance data for a cylinder link
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct LinkInstance {
    start: [f32; 3],
    radius: f32,
    end: [f32; 3],
    _padding: u32,
    color: [f32; 4],
}

pub struct ConnectorRenderer {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    instance_buffer: Option<wgpu::Buffer>,
    render_pipeline: wgpu::RenderPipeline,
    num_indices: u32,
    num_instances: u32,
}

impl ConnectorRenderer {
    pub fn new(wgpu: &Wgpu) -> Self {
        let (vertex_buffer, index_buffer, num_indices) = wgpu.create_cylinder();

        let instance_layout = wgpu::VertexBufferLayout {
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

        let render_pipeline = wgpu.create_fabric_pipeline("Connector Pipeline", instance_layout);

        ConnectorRenderer {
            vertex_buffer,
            index_buffer,
            instance_buffer: None,
            render_pipeline,
            num_indices,
            num_instances: 0,
        }
    }

    pub fn update(&mut self, wgpu: &Wgpu, fabric: &Fabric, _pick: &Pick) {
        let instances = self.create_instances(fabric);
        self.num_instances = instances.len() as u32;

        if self.num_instances > 0 {
            self.instance_buffer = Some(wgpu.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Link Instance Buffer"),
                    contents: bytemuck::cast_slice(&instances),
                    usage: wgpu::BufferUsages::VERTEX,
                },
            ));
        }
    }

    fn create_instances(&self, fabric: &Fabric) -> Vec<LinkInstance> {
        let mut instances = Vec::new();

        let Some(connector) = fabric.connector.as_ref() else {
            return instances;
        };
        // Render the connector links at the cable thickness, matching the
        // cylinder_renderer's physical_radius() for pulls.
        let link_radius = fabric.dimensions.pull_radius.f32();

        // Iterate through all push intervals to find their connections
        for (key, interval) in fabric.intervals.iter() {
            if !interval.has_role(Role::Pushing) {
                continue;
            }

            // Get joint positions
            let alpha_pos = fabric.joints[interval.alpha_key].location;
            let omega_pos = fabric.joints[interval.omega_key].location;
            let push_dir = (omega_pos - alpha_pos).normalize();

            // Process alpha end connections
            self.add_links_for_end(
                &mut instances,
                fabric,
                key,
                interval,
                IntervalEnd::Alpha,
                alpha_pos,
                -push_dir, // outward axis
                connector,
                link_radius,
            );

            // Process omega end connections
            self.add_links_for_end(
                &mut instances,
                fabric,
                key,
                interval,
                IntervalEnd::Omega,
                omega_pos,
                push_dir, // outward axis
                connector,
                link_radius,
            );
        }

        instances
    }

    fn add_links_for_end(
        &self,
        instances: &mut Vec<LinkInstance>,
        fabric: &Fabric,
        push_key: IntervalKey,
        push_interval: &crate::fabric::interval::Interval,
        end: IntervalEnd,
        joint_pos: Vec3,
        push_axis: Vec3,
        connector: &ConnectorSystem,
        link_radius: f32,
    ) {
        let connections = match connector.connections(push_key, end) {
            Some(c) => c,
            None => return,
        };

        // Collect connections with their slot indices
        let mut slot_connections: Vec<(usize, Vec3)> = Vec::new();

        for (slot_idx, conn_opt) in connections.iter().enumerate() {
            if let Some(connection) = conn_opt {
                if let Some(pull_interval) = fabric.intervals.get(connection.pull_interval_key) {
                    let pull_joint_key = match end {
                        IntervalEnd::Alpha => push_interval.alpha_key,
                        IntervalEnd::Omega => push_interval.omega_key,
                    };

                    let pull_other_end = if pull_interval.alpha_key == pull_joint_key {
                        fabric.joints[pull_interval.omega_key].location
                    } else {
                        fabric.joints[pull_interval.alpha_key].location
                    };

                    let (pivot_pos, _elevation) = connector.pivot_geometry(
                        joint_pos,
                        push_axis,
                        slot_idx,
                        pull_other_end,
                    );

                    slot_connections.push((slot_idx, pivot_pos));
                }
            }
        }

        if slot_connections.is_empty() {
            return;
        }

        // Sort by slot
        slot_connections.sort_by_key(|(slot, _)| *slot);

        // Generate axial chain and radial arm links; the cable itself
        // continues from the pivot pin (see cylinder_renderer).
        let mut prev_pos = joint_pos;

        for (slot, pivot_pos) in &slot_connections {
            let ring_center = connector.ring_center(joint_pos, push_axis, *slot);

            // Axial link: previous position → ring center
            instances.push(LinkInstance {
                start: [prev_pos.x, prev_pos.y, prev_pos.z],
                radius: link_radius,
                end: [ring_center.x, ring_center.y, ring_center.z],
                _padding: 0,
                color: AXIAL_COLOR,
            });

            // Radial arm: ring center → pivot pin
            instances.push(LinkInstance {
                start: [ring_center.x, ring_center.y, ring_center.z],
                radius: link_radius,
                end: [pivot_pos.x, pivot_pos.y, pivot_pos.z],
                _padding: 0,
                color: RADIAL_COLOR,
            });

            prev_pos = ring_center;
        }
    }

    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        bind_group: &'a wgpu::BindGroup,
    ) {
        if self.num_instances > 0 && self.instance_buffer.is_some() {
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.as_ref().unwrap().slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..self.num_instances);
        }
    }
}
