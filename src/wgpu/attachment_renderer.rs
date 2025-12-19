/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::camera::Pick;
use crate::fabric::attachment::{AttachmentPoint, ConnectorSpec};
use crate::fabric::interval::{Interval, Role};
use crate::fabric::{Fabric, IntervalEnd};
use crate::wgpu::{Wgpu, DEFAULT_PRIMITIVE_STATE};
use crate::IntervalDetails;
use cgmath::{InnerSpace, Vector3};
use std::f32::consts::PI;
use std::mem::size_of;
use wgpu::util::DeviceExt;
use wgpu::PipelineCompilationOptions;

const ORANGE: [f32; 4] = [1.0, 0.1, 0.0, 1.0];
const GRAY: [f32; 4] = [0.3, 0.3, 0.3, 0.5];

/// Instance data for a ring/disc - oriented perpendicular to the push axis
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RingInstance {
    position: [f32; 3], // Center position of the ring
    radius: f32,        // Radius of the ring (matches push interval)
    normal: [f32; 3],   // Normal direction (push axis, for orientation)
    thickness: f32,     // Thickness of the ring (axial extent)
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
        // Create a unit disc geometry (flat cylinder)
        let (vertex_buffer, index_buffer, num_indices) = create_disc(wgpu);

        // Create shader module (uses ring_vertex and ring_fragment from shared shader file)
        let shader = wgpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Ring Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            });

        // Create the pipeline layout
        let pipeline_layout = wgpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Ring Pipeline Layout"),
                bind_group_layouts: &[&wgpu.uniform_bind_group_layout],
                push_constant_ranges: &[],
            });

        // Define the vertex buffer layout for disc geometry
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: size_of::<[f32; 8]>() as wgpu::BufferAddress, // position[3] + normal[3] + uv[2]
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

        // Define the instance buffer layout for rings
        let instance_layout = wgpu::VertexBufferLayout {
            array_stride: size_of::<RingInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // position
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
                // normal (push axis direction)
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // thickness
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 7]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32,
                },
                // color
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        };

        // Create the render pipeline
        let render_pipeline = wgpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                cache: None,
                label: Some("Ring Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    compilation_options: PipelineCompilationOptions::default(),
                    module: &shader,
                    entry_point: Some("ring_vertex"),
                    buffers: &[vertex_layout, instance_layout],
                },
                fragment: Some(wgpu::FragmentState {
                    compilation_options: PipelineCompilationOptions::default(),
                    module: &shader,
                    entry_point: Some("ring_fragment"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu.surface_configuration.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: DEFAULT_PRIMITIVE_STATE,
                depth_stencil: Some(crate::wgpu::default_depth_stencil_state()),
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
        // Create instances for attachment points (now as rings)
        let instances = self.create_instances(fabric, pick);
        self.num_instances = instances.len() as u32;

        // Update instance buffer if there are instances to render
        if self.num_instances > 0 {
            self.instance_buffer = Some(wgpu.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Ring Instance Buffer"),
                    contents: bytemuck::cast_slice(&instances),
                    usage: wgpu::BufferUsages::VERTEX,
                },
            ));
        }
    }

    fn create_instances(&self, fabric: &Fabric, pick: &Pick) -> Vec<RingInstance> {
        let mut instances = Vec::new();

        // Track which push intervals are selected/highlighted to avoid duplicate points
        let (selected_push_interval, selected_joint, original_push_interval) = match pick {
            Pick::Interval(IntervalDetails {
                id,
                role,
                selected_push: original_interval_id,
                ..
            }) => {
                if role.is(Role::Pushing) {
                    (Some(id.0), None, None)
                } else if role.is(Role::Pulling) {
                    // For pull intervals, check if there's an original push interval to highlight
                    (None, None, original_interval_id.map(|id| id.0))
                } else {
                    (None, None, None)
                }
            }
            Pick::Joint(joint_details) => (None, Some(joint_details.index), None),
            _ => (None, None, None),
        };

        let connector = ConnectorSpec::for_scale(fabric.scale());
        // Ring radius should match push interval radius: base_radius * radius_factor * scale
        // base_radius = 0.04, push radius_factor = 1.2
        let ring_radius = 0.04 * 1.2 * fabric.scale();
        // Ring thickness with small gap between rings
        let ring_thickness = connector.ring_thickness * 0.8; // 80% of slot width leaves 20% gap

        // Add all push interval attachment points with appropriate coloring
        self.add_push_interval_rings(
            &mut instances,
            fabric,
            selected_push_interval,
            selected_joint,
            original_push_interval,
            ring_radius,
            ring_thickness,
            &connector,
            pick,
        );

        // Handle the case of a selected pull interval
        self.add_pull_interval_rings(
            &mut instances,
            fabric,
            pick,
            ring_radius,
            ring_thickness,
            &connector,
        );

        instances
    }

    /// Adds ring instances for push intervals
    fn add_push_interval_rings(
        &self,
        instances: &mut Vec<RingInstance>,
        fabric: &Fabric,
        selected_push_interval: Option<usize>,
        selected_joint: Option<usize>,
        original_push_interval: Option<usize>,
        ring_radius: f32,
        ring_thickness: f32,
        connector: &ConnectorSpec,
        pick: &Pick,
    ) {
        for (idx, interval_opt) in fabric.intervals.iter().enumerate() {
            if let Some(interval) = interval_opt {
                if interval.has_role(Role::Pushing) {
                    // Determine if this push interval is selected, connected to a selected joint,
                    // or is the original interval of a selected pull interval
                    let is_selected = selected_push_interval
                        .map_or(false, |selected_idx| selected_idx == idx)
                        || selected_joint.map_or(false, |joint_idx| {
                            interval.alpha_index == joint_idx || interval.omega_index == joint_idx
                        })
                        || original_push_interval.map_or(false, |orig_idx| orig_idx == idx);

                    // Get attachment points for this interval
                    if let Ok((alpha_points, omega_points)) =
                        interval.attachment_points(&fabric.joints, connector)
                    {
                        // Calculate push axis direction
                        let alpha_pos = fabric.joints[interval.alpha_index].location;
                        let omega_pos = fabric.joints[interval.omega_index].location;
                        let push_dir = (omega_pos - alpha_pos).normalize();

                        // Alpha end normal points outward (opposite to push direction)
                        let alpha_normal = -push_dir;
                        // Omega end normal points outward (same as push direction)
                        let omega_normal = push_dir;

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

                            // Add all ring instances with appropriate color
                            self.add_ring_instances(
                                instances,
                                &alpha_points,
                                &alpha_occupied,
                                ring_radius,
                                ring_thickness,
                                alpha_normal,
                                interval,
                                IntervalEnd::Alpha,
                                pick,
                            );

                            self.add_ring_instances(
                                instances,
                                &omega_points,
                                &omega_occupied,
                                ring_radius,
                                ring_thickness,
                                omega_normal,
                                interval,
                                IntervalEnd::Omega,
                                pick,
                            );
                        } else {
                            // For non-selected intervals, only show occupied attachment points
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

                            // Add only occupied ring instances
                            self.add_ring_instances(
                                instances,
                                &alpha_points,
                                &alpha_occupied,
                                ring_radius,
                                ring_thickness,
                                alpha_normal,
                                interval,
                                IntervalEnd::Alpha,
                                pick,
                            );

                            self.add_ring_instances(
                                instances,
                                &omega_points,
                                &omega_occupied,
                                ring_radius,
                                ring_thickness,
                                omega_normal,
                                interval,
                                IntervalEnd::Omega,
                                pick,
                            );
                        }
                    }
                }
            }
        }
    }

    /// Adds ring instances for a selected pull interval
    fn add_pull_interval_rings(
        &self,
        instances: &mut Vec<RingInstance>,
        fabric: &Fabric,
        pick: &Pick,
        ring_radius: f32,
        ring_thickness: f32,
        connector: &ConnectorSpec,
    ) {
        if let Pick::Interval(IntervalDetails {
            id: _,
            role,
            near_joint,
            selected_push: original_interval_id,
            ..
        }) = pick
        {
            if role.is(Role::Pulling) {
                // First, handle the original push interval if present
                if let Some(orig_id) = original_interval_id {
                    if let Some(orig_interval) = fabric.intervals[orig_id.0].as_ref() {
                        if orig_interval.has_role(Role::Pushing) {
                            // Get attachment points for the original push interval
                            if let Ok((alpha_points, omega_points)) =
                                orig_interval.attachment_points(&fabric.joints, connector)
                            {
                                // Calculate push axis direction
                                let alpha_pos = fabric.joints[orig_interval.alpha_index].location;
                                let omega_pos = fabric.joints[orig_interval.omega_index].location;
                                let push_dir = (omega_pos - alpha_pos).normalize();
                                let alpha_normal = -push_dir;
                                let omega_normal = push_dir;

                                // Create a set of occupied attachment indices for alpha and omega ends
                                let alpha_occupied = self.get_occupied_indices(
                                    orig_interval,
                                    IntervalEnd::Alpha,
                                    alpha_points.len(),
                                );
                                let omega_occupied = self.get_occupied_indices(
                                    orig_interval,
                                    IntervalEnd::Omega,
                                    omega_points.len(),
                                );

                                // Add all ring instances with appropriate color
                                self.add_ring_instances(
                                    instances,
                                    &alpha_points,
                                    &alpha_occupied,
                                    ring_radius,
                                    ring_thickness,
                                    alpha_normal,
                                    orig_interval,
                                    IntervalEnd::Alpha,
                                    pick,
                                );

                                self.add_ring_instances(
                                    instances,
                                    &omega_points,
                                    &omega_occupied,
                                    ring_radius,
                                    ring_thickness,
                                    omega_normal,
                                    orig_interval,
                                    IntervalEnd::Omega,
                                    pick,
                                );
                            }
                        }
                    }
                }

                // Then handle any push intervals connected to the near joint
                let joint_index = *near_joint;

                // Find all push intervals connected to this joint
                for (_idx, interval_opt) in fabric.intervals.iter().enumerate() {
                    if let Some(interval) = interval_opt {
                        if interval.has_role(Role::Pushing) {
                            // Skip the original interval if it's one of the connected push intervals
                            // to avoid duplicate attachment points
                            if let Some(orig_id) = original_interval_id {
                                if _idx == orig_id.0 {
                                    continue;
                                }
                            }

                            if interval.alpha_index == joint_index
                                || interval.omega_index == joint_index
                            {
                                // Get attachment points for this push interval
                                if let Ok((alpha_points, omega_points)) =
                                    interval.attachment_points(&fabric.joints, connector)
                                {
                                    // Calculate push axis direction
                                    let alpha_pos = fabric.joints[interval.alpha_index].location;
                                    let omega_pos = fabric.joints[interval.omega_index].location;
                                    let push_dir = (omega_pos - alpha_pos).normalize();

                                    // Only show attachment points for the end connected to the joint
                                    let (points_to_show, normal) =
                                        if interval.alpha_index == joint_index {
                                            (&alpha_points, -push_dir)
                                        } else {
                                            (&omega_points, push_dir)
                                        };

                                    // Add the ring instances
                                    for point in points_to_show.iter() {
                                        instances.push(RingInstance {
                                            position: [
                                                point.position.x,
                                                point.position.y,
                                                point.position.z,
                                            ],
                                            radius: ring_radius,
                                            normal: [normal.x, normal.y, normal.z],
                                            thickness: ring_thickness,
                                            color: ORANGE,
                                        });
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

    /// Adds ring instances for a set of attachment points
    fn add_ring_instances(
        &self,
        instances: &mut Vec<RingInstance>,
        points: &[AttachmentPoint],
        occupied: &[bool],
        ring_radius: f32,
        ring_thickness: f32,
        normal: Vector3<f32>,
        interval: &Interval,
        end: IntervalEnd,
        pick: &Pick,
    ) {
        for (i, point) in points.iter().enumerate() {
            let is_occupied = i < occupied.len() && occupied[i];

            // Only render occupied attachment points
            if !is_occupied {
                continue;
            }

            // Determine color: ORANGE if connected to selected pull interval, otherwise GRAY
            let color = if let Pick::Interval(IntervalDetails {
                id: selected_id,
                role,
                ..
            }) = pick
            {
                if role.is(Role::Pulling) {
                    // Check if this specific attachment point is connected to the selected pull interval
                    if let Some(connections) = interval.connections(end) {
                        if let Some(Some(connection)) = connections.get(i) {
                            if connection.pull_interval_id == *selected_id {
                                ORANGE
                            } else {
                                GRAY
                            }
                        } else {
                            GRAY
                        }
                    } else {
                        GRAY
                    }
                } else {
                    GRAY
                }
            } else {
                GRAY
            };

            instances.push(RingInstance {
                position: [point.position.x, point.position.y, point.position.z],
                radius: ring_radius,
                normal: [normal.x, normal.y, normal.z],
                thickness: ring_thickness,
                color,
            });
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

/// Creates a unit disc (flat cylinder) geometry centered at origin
/// The disc lies in the XZ plane with Y as the normal (up)
/// Radius is 1.0, thickness (height) is 1.0 (from -0.5 to 0.5 in Y)
fn create_disc(wgpu: &Wgpu) -> (wgpu::Buffer, wgpu::Buffer, u32) {
    use bytemuck::cast_slice;

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct DiscVertex {
        position: [f32; 3],
        normal: [f32; 3],
        uv: [f32; 2],
    }

    const SEGMENTS: u32 = 16;
    const HALF_HEIGHT: f32 = 0.5;

    let mut vertices = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Pre-calculate ring positions
    let mut ring_positions = Vec::with_capacity(SEGMENTS as usize);
    for i in 0..SEGMENTS {
        let angle = (i as f32) / (SEGMENTS as f32) * 2.0 * PI;
        let x = angle.cos();
        let z = angle.sin();
        ring_positions.push((x, z));
    }

    // Top face (Y = +HALF_HEIGHT, normal pointing up)
    let top_center_idx = vertices.len() as u32;
    vertices.push(DiscVertex {
        position: [0.0, HALF_HEIGHT, 0.0],
        normal: [0.0, 1.0, 0.0],
        uv: [0.5, 0.5],
    });

    let top_ring_start = vertices.len() as u32;
    for (x, z) in &ring_positions {
        vertices.push(DiscVertex {
            position: [*x, HALF_HEIGHT, *z],
            normal: [0.0, 1.0, 0.0],
            uv: [0.5 + 0.5 * x, 0.5 + 0.5 * z],
        });
    }

    // Top face triangles (counter-clockwise when viewed from above)
    for i in 0..SEGMENTS {
        let current = top_ring_start + i;
        let next = top_ring_start + ((i + 1) % SEGMENTS);
        indices.push(top_center_idx);
        indices.push(next);
        indices.push(current);
    }

    // Bottom face (Y = -HALF_HEIGHT, normal pointing down)
    let bottom_center_idx = vertices.len() as u32;
    vertices.push(DiscVertex {
        position: [0.0, -HALF_HEIGHT, 0.0],
        normal: [0.0, -1.0, 0.0],
        uv: [0.5, 0.5],
    });

    let bottom_ring_start = vertices.len() as u32;
    for (x, z) in &ring_positions {
        vertices.push(DiscVertex {
            position: [*x, -HALF_HEIGHT, *z],
            normal: [0.0, -1.0, 0.0],
            uv: [0.5 + 0.5 * x, 0.5 + 0.5 * z],
        });
    }

    // Bottom face triangles (counter-clockwise when viewed from below)
    for i in 0..SEGMENTS {
        let current = bottom_ring_start + i;
        let next = bottom_ring_start + ((i + 1) % SEGMENTS);
        indices.push(bottom_center_idx);
        indices.push(current);
        indices.push(next);
    }

    // Side faces (cylinder wall)
    let side_top_start = vertices.len() as u32;
    for (x, z) in &ring_positions {
        vertices.push(DiscVertex {
            position: [*x, HALF_HEIGHT, *z],
            normal: [*x, 0.0, *z], // Normal points outward radially
            uv: [0.0, 0.0],
        });
    }

    let side_bottom_start = vertices.len() as u32;
    for (x, z) in &ring_positions {
        vertices.push(DiscVertex {
            position: [*x, -HALF_HEIGHT, *z],
            normal: [*x, 0.0, *z], // Normal points outward radially
            uv: [0.0, 1.0],
        });
    }

    // Side triangles
    for i in 0..SEGMENTS {
        let top_current = side_top_start + i;
        let top_next = side_top_start + ((i + 1) % SEGMENTS);
        let bottom_current = side_bottom_start + i;
        let bottom_next = side_bottom_start + ((i + 1) % SEGMENTS);

        // First triangle
        indices.push(top_current);
        indices.push(top_next);
        indices.push(bottom_current);

        // Second triangle
        indices.push(bottom_current);
        indices.push(top_next);
        indices.push(bottom_next);
    }

    let vertex_buffer = wgpu
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Disc Vertex Buffer"),
            contents: cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

    let index_buffer = wgpu
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Disc Index Buffer"),
            contents: cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

    (vertex_buffer, index_buffer, indices.len() as u32)
}
