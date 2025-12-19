/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::camera::Pick;
use crate::fabric::attachment::ConnectorSpec;
use crate::fabric::interval::Role;
use crate::fabric::{Fabric, IntervalEnd};
use crate::wgpu::{default_depth_stencil_state, vertex_layout_f32x8, Wgpu, DEFAULT_PRIMITIVE_STATE};
use bytemuck::{Pod, Zeroable};
use cgmath::InnerSpace;
use std::f32::consts::PI;
use wgpu::util::DeviceExt;
use wgpu::PipelineCompilationOptions;

const KNOT_COLOR: [f32; 4] = [0.25, 0.25, 0.25, 1.0];

/// Instance data for a sphere "knot" at the end of a pull interval
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct KnotInstance {
    position: [f32; 3], // Position of the knot (at disc edge)
    scale: f32,         // Radius of the sphere
    color: [f32; 4],    // RGBA color
}

pub struct KnotRenderer {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    instance_buffer: Option<wgpu::Buffer>,
    render_pipeline: wgpu::RenderPipeline,
    num_indices: u32,
    num_instances: u32,
}

impl KnotRenderer {
    pub fn new(wgpu: &Wgpu) -> Self {
        // Use the existing sphere geometry from joint_renderer
        let (vertex_buffer, index_buffer, num_indices) = create_sphere(wgpu);

        // Use the consolidated shader module with joint_vertex/joint_fragment
        let shader = wgpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Knot Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            });

        // Create the pipeline layout
        let pipeline_layout = wgpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Knot Pipeline Layout"),
                bind_group_layouts: &[&wgpu.uniform_bind_group_layout],
                push_constant_ranges: &[],
            });

        // Define the vertex buffer layout
        let vertex_layout = vertex_layout_f32x8();

        // Define the instance buffer layout (same as JointMarkerInstance)
        let instance_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<KnotInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // scale
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32,
                },
                // color
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        };

        // Create the render pipeline
        let render_pipeline = wgpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                cache: None,
                label: Some("Knot Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    compilation_options: PipelineCompilationOptions::default(),
                    module: &shader,
                    entry_point: Some("joint_vertex"),
                    buffers: &[vertex_layout, instance_layout],
                },
                fragment: Some(wgpu::FragmentState {
                    compilation_options: PipelineCompilationOptions::default(),
                    module: &shader,
                    entry_point: Some("joint_fragment"),
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

        KnotRenderer {
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
                    label: Some("Knot Instance Buffer"),
                    contents: bytemuck::cast_slice(&instances),
                    usage: wgpu::BufferUsages::VERTEX,
                },
            ));
        }
    }

    fn create_instances(&self, fabric: &Fabric) -> Vec<KnotInstance> {
        let mut instances = Vec::new();

        // Knot radius matches pull interval radius
        let knot_radius = 0.008 * fabric.scale();
        let connector = ConnectorSpec::for_scale(fabric.scale());

        // Iterate through all push intervals to find their connections
        for interval_opt in fabric.intervals.iter() {
            if let Some(interval) = interval_opt {
                if interval.has_role(Role::Pushing) {
                    // Get joint positions
                    let alpha_pos = fabric.joints[interval.alpha_index].location;
                    let omega_pos = fabric.joints[interval.omega_index].location;
                    let push_dir = (omega_pos - alpha_pos).normalize();

                    // Process alpha end connections - knots at hinge positions
                    if let Some(connections) = interval.connections(IntervalEnd::Alpha) {
                        for (slot_idx, conn_opt) in connections.iter().enumerate() {
                            if let Some(connection) = conn_opt {
                                // Find the pull interval to get its other end position
                                if let Some(Some(pull_interval)) =
                                    fabric.intervals.get(connection.pull_interval_id.0)
                                {
                                    // Determine which end of the pull connects here
                                    let pull_other_end =
                                        if pull_interval.alpha_index == interval.alpha_index {
                                            fabric.joints[pull_interval.omega_index].location
                                        } else {
                                            fabric.joints[pull_interval.alpha_index].location
                                        };

                                    // Calculate hinge position (alpha end points outward, opposite to push_dir)
                                    let hinge_pos = connector.hinge_position(
                                        alpha_pos,
                                        -push_dir, // Outward from alpha end
                                        slot_idx,
                                        pull_other_end,
                                    );

                                    instances.push(KnotInstance {
                                        position: [hinge_pos.x, hinge_pos.y, hinge_pos.z],
                                        scale: knot_radius,
                                        color: KNOT_COLOR,
                                    });
                                }
                            }
                        }
                    }

                    // Process omega end connections - knots at hinge positions
                    if let Some(connections) = interval.connections(IntervalEnd::Omega) {
                        for (slot_idx, conn_opt) in connections.iter().enumerate() {
                            if let Some(connection) = conn_opt {
                                // Find the pull interval to get its other end position
                                if let Some(Some(pull_interval)) =
                                    fabric.intervals.get(connection.pull_interval_id.0)
                                {
                                    // Determine which end of the pull connects here
                                    let pull_other_end =
                                        if pull_interval.alpha_index == interval.omega_index {
                                            fabric.joints[pull_interval.omega_index].location
                                        } else {
                                            fabric.joints[pull_interval.alpha_index].location
                                        };

                                    // Calculate hinge position (omega end points outward, same as push_dir)
                                    let hinge_pos = connector.hinge_position(
                                        omega_pos,
                                        push_dir, // Outward from omega end
                                        slot_idx,
                                        pull_other_end,
                                    );

                                    instances.push(KnotInstance {
                                        position: [hinge_pos.x, hinge_pos.y, hinge_pos.z],
                                        scale: knot_radius,
                                        color: KNOT_COLOR,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        instances
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

/// Creates a sphere geometry for knot visualization
fn create_sphere(wgpu: &Wgpu) -> (wgpu::Buffer, wgpu::Buffer, u32) {
    let radius = 1.0;
    let sectors = 12;
    let stacks = 12;

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Vertex {
        position: [f32; 3],
        normal: [f32; 3],
        uv: [f32; 2],
    }

    let mut vertices = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Generate vertices
    for i in 0..=stacks {
        let stack_angle = PI * (i as f32) / (stacks as f32);
        let xy = radius * stack_angle.sin();
        let z = radius * stack_angle.cos();

        for j in 0..=sectors {
            let sector_angle = 2.0 * PI * (j as f32) / (sectors as f32);
            let x = xy * sector_angle.cos();
            let y = xy * sector_angle.sin();

            let position = [x, y, z];
            let length = (x * x + y * y + z * z).sqrt();
            let normal = [x / length, y / length, z / length];
            let u = j as f32 / sectors as f32;
            let v = i as f32 / stacks as f32;

            vertices.push(Vertex {
                position,
                normal,
                uv: [u, v],
            });
        }
    }

    // Generate indices
    for i in 0..stacks {
        let k1 = i * (sectors + 1);
        let k2 = k1 + sectors + 1;

        for j in 0..sectors {
            if i != 0 {
                indices.push(k1 + j);
                indices.push(k2 + j);
                indices.push(k1 + j + 1);
            }

            if i != (stacks - 1) {
                indices.push(k1 + j + 1);
                indices.push(k2 + j);
                indices.push(k2 + j + 1);
            }
        }
    }

    let vertex_buffer = wgpu
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Knot Sphere Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

    let index_buffer = wgpu
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Knot Sphere Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

    (vertex_buffer, index_buffer, indices.len() as u32)
}
