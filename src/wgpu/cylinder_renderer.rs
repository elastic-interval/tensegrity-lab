use crate::camera::Pick;
use crate::fabric::interval::Role;
use crate::fabric::material::Material;
use crate::fabric::{Fabric, IntervalEnd, UniqueId};
use crate::wgpu::{Wgpu, DEFAULT_PRIMITIVE_STATE};
use crate::{Appearance, AppearanceMode, IntervalDetails, JointDetails, RenderStyle};
use bytemuck::{Pod, Zeroable};
use std::mem::size_of;
use wgpu::util::DeviceExt;
use wgpu::PipelineCompilationOptions;

// Instance data for cylinders - to be transformed by the GPU
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CylinderInstance {
    pub start: [f32; 3],    // Start position
    pub radius_factor: f32, // Relative radius (1.0 = standard)
    pub end: [f32; 3],      // End position
    pub material_type: u32, // 0=Push, 1=Pull, 2=Spring
    pub color: [f32; 4],    // RGBA color
}

pub struct CylinderRenderer {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    instance_buffer: Option<wgpu::Buffer>,
    render_pipeline: wgpu::RenderPipeline,
    num_indices: u32,
    num_instances: u32,
}

impl CylinderRenderer {
    pub fn new(wgpu: &Wgpu) -> Self {
        // Create a unit cylinder
        let (vertex_buffer, index_buffer, num_indices) = wgpu.create_cylinder();

        // Create the shader module
        let shader = wgpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Cylinder Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            });

        // Create the pipeline layout
        let pipeline_layout = wgpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Cylinder Pipeline Layout"),
                bind_group_layouts: &[&wgpu.uniform_bind_group_layout],
                push_constant_ranges: &[],
            });

        // Create the render pipeline
        let render_pipeline = wgpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                cache: None,
                label: Some("Cylinder Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    compilation_options: PipelineCompilationOptions::default(),
                    module: &shader,
                    entry_point: Option::from("fabric_vertex"),
                    buffers: &[
                        // Vertex buffer layout
                        Wgpu::cylinder_vertex_layout(),
                        // Instance buffer layout
                        wgpu::VertexBufferLayout {
                            array_stride: size_of::<CylinderInstance>() as wgpu::BufferAddress,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &[
                                // start position
                                wgpu::VertexAttribute {
                                    offset: 0,
                                    shader_location: 3,
                                    format: wgpu::VertexFormat::Float32x3,
                                },
                                // radius factor
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
                                // material type
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
                        },
                    ],
                },
                fragment: Some(wgpu::FragmentState {
                    compilation_options: PipelineCompilationOptions::default(),
                    module: &shader,
                    entry_point: Option::from("fabric_fragment"),
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

        Self {
            vertex_buffer,
            index_buffer,
            instance_buffer: None,
            render_pipeline,
            num_indices,
            num_instances: 0,
        }
    }

    pub fn update(
        &mut self,
        wgpu: &Wgpu,
        fabric: &Fabric,
        pick: &Pick,
        render_style: &mut RenderStyle,
    ) {
        let instances = self.create_instances_from_fabric(fabric, pick, render_style);
        self.num_instances = instances.len() as u32;
        // Update instance buffer if there are instances to render
        if self.num_instances > 0 {
            self.instance_buffer = Some(wgpu.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Cylinder Instance Buffer"),
                    contents: bytemuck::cast_slice(&instances),
                    usage: wgpu::BufferUsages::VERTEX,
                },
            ));
        }
    }

    // Create instances from fabric intervals - minimal CPU processing
    fn create_instances_from_fabric(
        &self,
        fabric: &Fabric,
        pick: &Pick,
        render_style: &RenderStyle,
    ) -> Vec<CylinderInstance> {
        use RenderStyle::*;
        let mut instances = Vec::with_capacity(fabric.intervals.len());

        // Scale interval thickness based on fabric size (bounding radius)
        // This ensures intervals look proportional regardless of fabric scale
        let radius_scale = fabric.bounding_radius().max(0.1);
        for (index, interval_opt) in fabric.intervals.iter().enumerate() {
            if let Some(interval) = interval_opt {
                let interval_id = UniqueId(index);
                let push = interval.material == Material::Push;
                match render_style {
                    WithPullMap { .. } if push => continue,
                    WithPushMap { .. } if !push => continue,
                    _ => {}
                }
                let (alpha, omega) = (interval.alpha_index, interval.omega_index);
                let start = fabric.joints[alpha].location;
                let end = fabric.joints[omega].location;
                let appearance = interval.role.appearance();
                let appearance = match pick {
                    Pick::Nothing => match render_style {
                        Normal { .. } => appearance,
                        ColorByRole { .. } => Appearance {
                            color: interval.role.color(),
                            radius: appearance.radius,
                        },
                        WithAppearanceFunction { function, .. } => {
                            function(interval).unwrap_or(appearance)
                        }
                        WithPullMap { map, .. } => {
                            let key = interval.key();
                            match map.get(&key) {
                                None => appearance.apply_mode(AppearanceMode::Faded),
                                Some(color) => Appearance {
                                    color: *color,
                                    radius: appearance.radius * 2.0, // Double thickness for colored intervals
                                },
                            }
                        }
                        WithPushMap { map, .. } => {
                            let key = interval.key();
                            match map.get(&key) {
                                None => appearance.apply_mode(AppearanceMode::Faded),
                                Some(color) => Appearance {
                                    color: *color,
                                    radius: appearance.radius * 2.0, // Double thickness for colored intervals
                                },
                            }
                        }
                    },
                    Pick::Joint(JointDetails { index, .. }) => {
                        if interval.touches(*index) {
                            appearance.highlighted_for_role(interval.role)
                        } else {
                            appearance.apply_mode(AppearanceMode::Faded)
                        }
                    }
                    Pick::Interval(IntervalDetails {
                        near_joint,
                        far_joint,
                        id,
                        role,
                        selected_push: original_interval_id,
                        ..
                    }) => {
                        // If this is the currently selected interval, highlight it based on its type
                        if *id == interval_id {
                            // Use the appropriate selected mode based on interval role
                            appearance.selected_for_role(interval.role)
                        } else if let Some(orig_id) = original_interval_id {
                            // Only highlight the interval if it's currently selected
                            if *orig_id == interval_id
                                && *id == interval_id
                                && interval.has_role(Role::Pushing)
                            {
                                appearance.apply_mode(AppearanceMode::SelectedPush)
                            } else {
                                if interval.touches(*near_joint) {
                                    // Use the appropriate highlighted mode based on interval role
                                    appearance
                                        .highlighted_for_role(interval.role)
                                } else {
                                    // Use the Faded mode for non-adjacent intervals
                                    appearance.apply_mode(AppearanceMode::Faded)
                                }
                            }
                        } else {
                            // For intervals without an original_interval_id
                            // This case is less common but we should maintain consistent behavior
                            let active = match role {
                                Role::Pushing => {
                                    // For push intervals, consider intervals adjacent to both near and far joints
                                    interval.touches(*near_joint) || interval.touches(*far_joint)
                                }
                                Role::Springy => false,
                                _ => interval.touches(*near_joint),
                            };
                            if active {
                                // Use the appropriate highlighted mode based on interval role
                                appearance.highlighted_for_role(interval.role)
                            } else {
                                // Use the Faded mode for non-adjacent intervals
                                appearance.apply_mode(AppearanceMode::Faded)
                            }
                        }
                    }
                };
                // Check if this is a pull interval connected to a selected push interval or joint
                let mut modified_start = start;
                let mut modified_end = end;

                // For pull-like intervals, connect them to attachment points on push intervals
                // only when attachment points are visible (knots mode)
                if interval.role.is_pull_like() && render_style.show_attachment_points() {
                    // Use the current index as the pull interval ID
                    let pull_id = interval_id;

                    // Process both ends of the pull interval
                    let joint_indices = [interval.alpha_index, interval.omega_index];
                    let modified_points = [&mut modified_start, &mut modified_end];

                    // For each end of the pull interval
                    for (i, joint_index) in joint_indices.iter().enumerate() {
                        // Find all push intervals connected to this joint
                        for push_opt in fabric.intervals.iter() {
                            if let Some(push_interval) = push_opt {
                                // Only consider push intervals
                                if push_interval.has_role(Role::Pushing) {
                                    // Check if this push interval is connected to the current joint
                                    if push_interval.touches(*joint_index) {
                                        // Get attachment points for this push interval
                                        if let Ok((alpha_points, omega_points)) =
                                            push_interval.attachment_points(&fabric.joints)
                                        {
                                            // Determine which end of the push interval is connected to the joint
                                            let end = if push_interval.alpha_index == *joint_index {
                                                IntervalEnd::Alpha
                                            } else {
                                                IntervalEnd::Omega
                                            };

                                            // Get the connection data for this end
                                            if let Some(connections) =
                                                push_interval.connections(end)
                                            {
                                                // Look for a connection to this pull interval
                                                for conn in connections.iter() {
                                                    if let Some(pull_conn) = conn {
                                                        if pull_conn.pull_interval_id == pull_id {
                                                            // Found the connection - use the actual attachment point
                                                            let points =
                                                                if end == IntervalEnd::Alpha {
                                                                    &alpha_points
                                                                } else {
                                                                    &omega_points
                                                                };

                                                            // Use the attachment index from the connection data
                                                            let attachment_idx =
                                                                pull_conn.attachment_index;
                                                            if attachment_idx < points.len() {
                                                                *modified_points[i] =
                                                                    points[attachment_idx].position;
                                                            }

                                                            // We found the connection, no need to check others
                                                            break;
                                                        }
                                                    }
                                                }
                                            }

                                            // We found a push interval for this joint, no need to check others
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Additional processing for selected elements
                    match pick {
                        // If a push interval is selected, we want to ensure that pull intervals
                        // connected to it are properly visualized
                        Pick::Interval(IntervalDetails { id, .. }) => {
                            if let Some(push_interval) = fabric.intervals[id.0].as_ref() {
                                if push_interval.has_role(Role::Pushing) {
                                    // We've already handled the basic case above, but we might need
                                    // additional logic for selected push intervals if needed
                                }
                            }
                        }
                        // If a joint is selected, we've already handled it in the general case above
                        Pick::Joint(_) => {}
                        _ => {}
                    }
                }

                instances.push(CylinderInstance {
                    start: [modified_start.x, modified_start.y, modified_start.z],
                    radius_factor: appearance.radius * radius_scale,
                    end: [modified_end.x, modified_end.y, modified_end.z],
                    material_type: interval.role as u32,
                    color: appearance.color,
                });
            }
        }

        instances
    }

    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        bind_group: &'a wgpu::BindGroup,
    ) {
        // Skip if no instances to render
        if self.num_instances == 0 || self.instance_buffer.is_none() {
            return;
        }

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.as_ref().unwrap().slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        // Draw all instances at once
        render_pass.draw_indexed(0..self.num_indices, 0, 0..self.num_instances);
    }
}
