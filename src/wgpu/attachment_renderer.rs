/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::camera::Pick;
use crate::fabric::attachment::AttachmentPoint;
use crate::fabric::interval::Role;
use crate::fabric::Fabric;
use crate::fabric::IntervalEnd;
use crate::wgpu::{create_sphere, Wgpu};
use crate::Interval;
use crate::IntervalDetails;
use bytemuck::{Pod, Zeroable};
use std::mem::size_of;
use wgpu::util::DeviceExt;
use wgpu::PipelineCompilationOptions;

// No need for this struct as we're using AttachmentPointInstance

// Sphere to represent an attachment point
// Size is calculated dynamically based on the bar radius to make attachment points touch tangentially
const ATTACHMENT_POINT_COLOR: [f32; 4] = [1.0, 0.1, 0.0, 1.0]; // Reddish color for attachment points
const FADED_ATTACHMENT_POINT_COLOR: [f32; 4] = [0.3, 0.3, 0.3, 0.2]; // Faded attachment point color - very subtle gray

// Instance data for attachment points (similar to JointMarkerInstance)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct AttachmentPointInstance {
    position: [f32; 3], // Position of the attachment point
    scale: f32,         // Size of the marker
    color: [f32; 4],    // RGBA color
}

pub struct AttachmentRenderer {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    instance_buffer: Option<wgpu::Buffer>,
    render_pipeline: wgpu::RenderPipeline,
    num_indices: u32,
    num_instances: u32,
}

impl AttachmentRenderer {
    pub fn new(wgpu: &Wgpu) -> Self {
        // Create a simple sphere for attachment point visualization
        // Reusing the same sphere creation function as JointRenderer
        let (vertex_buffer, index_buffer, num_indices) = create_sphere(wgpu);

        // Use the same shader as the joint renderer
        let shader = wgpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Attachment Point Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            });

        // Create the pipeline layout
        let pipeline_layout = wgpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Attachment Point Pipeline Layout"),
                bind_group_layouts: &[&wgpu.uniform_bind_group_layout],
                push_constant_ranges: &[],
            });

        // Define the vertex buffer layout
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: size_of::<[f32; 8]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // normal
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // uv
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        };

        // Define the instance buffer layout
        let instance_layout = wgpu::VertexBufferLayout {
            array_stride: size_of::<AttachmentPointInstance>() as wgpu::BufferAddress,
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
                    offset: size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32,
                },
                // color
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
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
                label: Some("Attachment Point Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    compilation_options: PipelineCompilationOptions::default(),
                    module: &shader,
                    entry_point: Some("joint_vertex"), // Reuse the joint vertex shader
                    buffers: &[vertex_layout, instance_layout],
                },
                fragment: Some(wgpu::FragmentState {
                    compilation_options: PipelineCompilationOptions::default(),
                    module: &shader,
                    entry_point: Some("joint_fragment"), // Reuse the joint fragment shader
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu.surface_configuration.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            });

        AttachmentRenderer {
            vertex_buffer,
            index_buffer,
            instance_buffer: None,
            render_pipeline,
            num_indices,
            num_instances: 0,
        }
    }

    pub fn update(&mut self, wgpu: &Wgpu, fabric: &Fabric, pick: &Pick) {
        // Create instances for attachment points
        let instances = self.create_instances(fabric, pick);
        self.num_instances = instances.len() as u32;

        // Update instance buffer if there are instances to render
        if self.num_instances > 0 {
            self.instance_buffer = Some(wgpu.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Attachment Point Instance Buffer"),
                    contents: bytemuck::cast_slice(&instances),
                    usage: wgpu::BufferUsages::VERTEX,
                },
            ));
        }
    }

    fn create_instances(&self, fabric: &Fabric, pick: &Pick) -> Vec<AttachmentPointInstance> {
        let mut instances = Vec::new();

        // Track which push intervals are selected/highlighted to avoid duplicate points
        let (selected_push_interval, selected_joint) = match pick {
            Pick::Interval(IntervalDetails { id, role, .. }) if *role == Role::Pushing => {
                (Some(id.0), None)
            }
            Pick::Joint(joint_details) => (None, Some(joint_details.index)),
            _ => (None, None),
        };

        // Calculate point radius once - use a consistent size for all attachment points
        let point_radius = Role::Pulling.appearance().radius * 0.12;

        // Add all push interval attachment points with appropriate coloring
        self.add_push_interval_attachment_points(
            &mut instances,
            fabric,
            selected_push_interval,
            selected_joint,
            point_radius,
        );

        // Handle the case of a selected pull interval
        self.add_pull_interval_attachment_points(&mut instances, fabric, pick, point_radius);

        instances
    }

    /// Adds attachment points for push intervals
    fn add_push_interval_attachment_points(
        &self,
        instances: &mut Vec<AttachmentPointInstance>,
        fabric: &Fabric,
        selected_push_interval: Option<usize>,
        selected_joint: Option<usize>,
        point_radius: f32,
    ) {
        for (idx, interval_opt) in fabric.intervals.iter().enumerate() {
            if let Some(interval) = interval_opt {
                if interval.material.properties().role == Role::Pushing {
                    // Determine if this push interval is selected or connected to a selected joint
                    let is_selected = selected_push_interval
                        .map_or(false, |selected_idx| selected_idx == idx)
                        || selected_joint.map_or(false, |joint_idx| {
                            interval.alpha_index == joint_idx || interval.omega_index == joint_idx
                        });

                    // Get attachment points for this interval
                    if let Ok((alpha_points, omega_points)) =
                        interval.attachment_points(&fabric.joints)
                    {
                        if is_selected {
                            // Create a set of occupied attachment indices for alpha and omega ends
                            let alpha_occupied = self.get_occupied_indices(
                                interval,
                                IntervalEnd::Alpha,
                                alpha_points.len(),
                            );
                            let omega_occupied = self.get_occupied_indices(
                                interval,
                                IntervalEnd::Omega,
                                omega_points.len(),
                            );

                            // Add all attachment points with appropriate color
                            self.add_attachment_point_instances(
                                instances,
                                &alpha_points,
                                &alpha_occupied,
                                point_radius,
                                true,
                            );

                            self.add_attachment_point_instances(
                                instances,
                                &omega_points,
                                &omega_occupied,
                                point_radius,
                                true,
                            );
                        } else {
                            // For non-selected intervals, just show faded attachment points
                            for point in alpha_points.iter().chain(omega_points.iter()) {
                                instances.push(self.create_attachment_point_instance(
                                    point,
                                    point_radius,
                                    FADED_ATTACHMENT_POINT_COLOR,
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    /// Adds attachment points for a selected pull interval
    fn add_pull_interval_attachment_points(
        &self,
        instances: &mut Vec<AttachmentPointInstance>,
        fabric: &Fabric,
        pick: &Pick,
        point_radius: f32,
    ) {
        if let Pick::Interval(IntervalDetails {
            id,
            role,
            near_joint,
            ..
        }) = pick
        {
            if *role == Role::Pulling {
                if let Some(_interval) = fabric.intervals[id.0].as_ref() {
                    // For pull intervals, show attachment points at the near joint
                    let joint_index = *near_joint;

                    // Find all push intervals connected to this joint
                    for (_idx, interval_opt) in fabric.intervals.iter().enumerate() {
                        if let Some(interval) = interval_opt {
                            if interval.material.properties().role == Role::Pushing {
                                if interval.alpha_index == joint_index
                                    || interval.omega_index == joint_index
                                {
                                    // Get attachment points for this push interval
                                    if let Ok((alpha_points, omega_points)) =
                                        interval.attachment_points(&fabric.joints)
                                    {
                                        // Only show attachment points for the end connected to the joint
                                        let points_to_show = if interval.alpha_index == joint_index
                                        {
                                            &alpha_points
                                        } else {
                                            &omega_points
                                        };

                                        // Add the attachment points to the instances
                                        for point in points_to_show.iter() {
                                            instances.push(self.create_attachment_point_instance(
                                                point,
                                                point_radius,
                                                ATTACHMENT_POINT_COLOR,
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Gets a vector of booleans indicating which attachment points are occupied
    fn get_occupied_indices(
        &self,
        interval: &Interval,
        end: IntervalEnd,
        points_len: usize,
    ) -> Vec<bool> {
        let mut occupied = vec![false; points_len];

        if let Some(connections) = interval.connections(end) {
            for (idx, conn) in connections.iter().enumerate() {
                if conn.is_some() && idx < occupied.len() {
                    occupied[idx] = true;
                }
            }
        }

        occupied
    }

    /// Adds attachment point instances for a set of points
    fn add_attachment_point_instances(
        &self,
        instances: &mut Vec<AttachmentPointInstance>,
        points: &[AttachmentPoint],
        occupied: &[bool],
        point_radius: f32,
        is_selected: bool,
    ) {
        for (i, point) in points.iter().enumerate() {
            let is_occupied = i < occupied.len() && occupied[i];
            let color = if is_occupied {
                // Use a bluish color for occupied points to match your preference for highlighted elements
                [0.4, 0.4, 0.9, 1.0]
            } else if is_selected {
                ATTACHMENT_POINT_COLOR // Reddish for available
            } else {
                FADED_ATTACHMENT_POINT_COLOR // Faded for non-selected
            };

            instances.push(self.create_attachment_point_instance(point, point_radius, color));
        }
    }

    /// Creates an attachment point instance with the given parameters
    fn create_attachment_point_instance(
        &self,
        point: &AttachmentPoint,
        scale: f32,
        color: [f32; 4],
    ) -> AttachmentPointInstance {
        AttachmentPointInstance {
            position: [point.position.x, point.position.y, point.position.z],
            scale,
            color,
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
