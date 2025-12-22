/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::camera::Pick;
use crate::fabric::attachment::ConnectorSpec;
use crate::fabric::interval::Role;
use crate::fabric::{Fabric, IntervalEnd};
use crate::wgpu::{default_depth_stencil_state, Wgpu, DEFAULT_PRIMITIVE_STATE};
use bytemuck::{Pod, Zeroable};
use cgmath::{InnerSpace, Point3};
use std::mem::size_of;
use wgpu::util::DeviceExt;
use wgpu::PipelineCompilationOptions;

// Pastel colors for the three link types
const AXIAL_COLOR: [f32; 4] = [1.0, 1.0, 0.6, 1.0];   // Pastel yellow
const RADIAL_COLOR: [f32; 4] = [1.0, 0.8, 0.5, 1.0];  // Pastel orange
const HINGE_COLOR: [f32; 4] = [1.0, 0.6, 0.6, 1.0];   // Pastel red

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

pub struct HingeRenderer {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    instance_buffer: Option<wgpu::Buffer>,
    render_pipeline: wgpu::RenderPipeline,
    num_indices: u32,
    num_instances: u32,
}

impl HingeRenderer {
    pub fn new(wgpu: &Wgpu) -> Self {
        let (vertex_buffer, index_buffer, num_indices) = wgpu.create_cylinder();

        let shader = wgpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Link Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            });

        let pipeline_layout = wgpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Link Pipeline Layout"),
                bind_group_layouts: &[&wgpu.uniform_bind_group_layout],
                push_constant_ranges: &[],
            });

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

        let render_pipeline = wgpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                cache: None,
                label: Some("Link Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    compilation_options: PipelineCompilationOptions::default(),
                    module: &shader,
                    entry_point: Some("fabric_vertex"),
                    buffers: &[Wgpu::cylinder_vertex_layout(), instance_layout],
                },
                fragment: Some(wgpu::FragmentState {
                    compilation_options: PipelineCompilationOptions::default(),
                    module: &shader,
                    entry_point: Some("fabric_fragment"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu.surface_configuration.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: DEFAULT_PRIMITIVE_STATE,
                depth_stencil: Some(default_depth_stencil_state()),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            });

        HingeRenderer {
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

        let connector = ConnectorSpec::for_scale(fabric.scale());
        // Use same radius as pull intervals in rendering (Role::Pulling.radius() * scale)
        let link_radius = 0.14 * fabric.scale();

        // Iterate through all push intervals to find their connections
        for (_key, interval) in fabric.intervals.iter() {
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
                interval,
                IntervalEnd::Alpha,
                alpha_pos,
                -push_dir, // outward axis
                &connector,
                link_radius,
            );

            // Process omega end connections
            self.add_links_for_end(
                &mut instances,
                fabric,
                interval,
                IntervalEnd::Omega,
                omega_pos,
                push_dir, // outward axis
                &connector,
                link_radius,
            );
        }

        instances
    }

    fn add_links_for_end(
        &self,
        instances: &mut Vec<LinkInstance>,
        fabric: &Fabric,
        push_interval: &crate::fabric::interval::Interval,
        end: IntervalEnd,
        joint_pos: Point3<f32>,
        push_axis: cgmath::Vector3<f32>,
        connector: &ConnectorSpec,
        link_radius: f32,
    ) {
        let connections = match push_interval.connections(end) {
            Some(c) => c,
            None => return,
        };

        // Collect connections with their slot indices
        let mut slot_connections: Vec<(usize, Point3<f32>, Point3<f32>)> = Vec::new();

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

                    let hinge_pos = connector.hinge_position(
                        joint_pos,
                        push_axis,
                        slot_idx,
                        pull_other_end,
                    );

                    // Calculate pull_end position (hinge + hinge_length along pull direction)
                    let pull_direction = (pull_other_end - hinge_pos).normalize();
                    let pull_end_pos = hinge_pos + pull_direction * *connector.hinge_length;

                    slot_connections.push((slot_idx + 1, hinge_pos, pull_end_pos));
                }
            }
        }

        if slot_connections.is_empty() {
            return;
        }

        // Sort by slot
        slot_connections.sort_by_key(|(slot, _, _)| *slot);

        // Generate axial chain and radial/hinge links
        let mut prev_pos = joint_pos;

        for (slot, hinge_pos, pull_end_pos) in &slot_connections {
            // Ring center at this slot (1x, 2x, 3x ring_thickness)
            let ring_center = joint_pos + push_axis * *connector.ring_thickness * *slot as f32;

            // Axial link: previous position → ring center
            instances.push(LinkInstance {
                start: [prev_pos.x, prev_pos.y, prev_pos.z],
                radius: link_radius,
                end: [ring_center.x, ring_center.y, ring_center.z],
                _padding: 0,
                color: AXIAL_COLOR,
            });

            // Radial link: ring center → hinge
            instances.push(LinkInstance {
                start: [ring_center.x, ring_center.y, ring_center.z],
                radius: link_radius,
                end: [hinge_pos.x, hinge_pos.y, hinge_pos.z],
                _padding: 0,
                color: RADIAL_COLOR,
            });

            // Hinge link: hinge → pull_end
            instances.push(LinkInstance {
                start: [hinge_pos.x, hinge_pos.y, hinge_pos.z],
                radius: link_radius,
                end: [pull_end_pos.x, pull_end_pos.y, pull_end_pos.z],
                _padding: 0,
                color: HINGE_COLOR,
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
