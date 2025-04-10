use crate::camera::Pick;
use crate::fabric::material::{interval_material, Material};
use crate::fabric::Fabric;
use crate::messages::RenderStyle;
use crate::wgpu::Wgpu;
use bytemuck::{Pod, Zeroable};
use std::mem::size_of;
use wgpu::util::DeviceExt;
use wgpu::PipelineCompilationOptions;

// Instance data for cylinders - to be transformed by the GPU
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CylinderInstance {
    start: [f32; 3],    // Start position
    radius_factor: f32, // Relative radius (1.0 = standard)
    end: [f32; 3],      // End position
    material_type: u32, // 0=Push, 1=Pull, 2=Spring
    color: [f32; 4],    // RGBA color
}

pub struct FabricRenderer {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    instance_buffer: Option<wgpu::Buffer>,
    render_pipeline: wgpu::RenderPipeline,
    num_indices: u32,
    num_instances: u32,
}

impl FabricRenderer {
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
                                    offset: size_of::<[f32; 7]>() as wgpu::BufferAddress + 4,
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
                    entry_point: Some("fabric_fragment"),
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
                depth_stencil: Some(wgpu.create_depth_stencil()),
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

    pub fn update_from_fabric(
        &mut self,
        wgpu: &Wgpu,
        fabric: &Fabric,
        pick: &Pick,
        render_style: &mut RenderStyle,
    ) {
        // Create instances from fabric data
        let instances = self.create_instances_from_fabric(fabric, pick, render_style);
        self.num_instances = instances.len() as u32;

        // Update instance buffer
        self.instance_buffer = Some(wgpu.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Cylinder Instance Buffer"),
                contents: bytemuck::cast_slice(&instances),
                usage: wgpu::BufferUsages::VERTEX,
            },
        ));
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

    // Create instances from fabric intervals - minimal CPU processing
    fn create_instances_from_fabric(
        &self,
        fabric: &Fabric,
        pick: &Pick,
        render_style: &RenderStyle,
    ) -> Vec<CylinderInstance> {
        use RenderStyle::*;
        let mut instances = Vec::with_capacity(fabric.intervals.len());
        for (interval_id, interval) in &fabric.intervals {
            let push = interval.material == Material::PushMaterial;
            match render_style {
                WithPullMap(_) if push => continue,
                WithPushMap(_) if !push => continue,
                _ => {}
            }
            let (alpha, omega) = (interval.alpha_index, interval.omega_index);
            let start = fabric.joints[alpha].location;
            let end = fabric.joints[omega].location;
            let material = interval_material(interval.material);
            let role_appearance = material.role.appearance();
            let appearance = match pick {
                Pick::Nothing => match render_style {
                    Normal => role_appearance,
                    WithAppearanceFunction(appearance) => {
                        appearance(interval).unwrap_or(role_appearance)
                    }
                    WithPullMap(color_map) | WithPushMap(color_map) => {
                        let key = interval.key();
                        match color_map.get(&key) {
                            None => role_appearance.faded(),
                            Some(color) => role_appearance.with_color(*color),
                        }
                    }
                },
                Pick::Joint { index, .. } => {
                    if !interval.touches(*index) {
                        role_appearance.faded()
                    } else {
                        role_appearance
                    }
                }
                Pick::Interval { joint, id, .. } => {
                    if *id == *interval_id {
                        role_appearance.highlighted()
                    } else if !interval.touches(*joint) {
                        role_appearance.faded()
                    } else {
                        role_appearance
                    }
                }
            };
            instances.push(CylinderInstance {
                start: [start.x, start.y, start.z],
                radius_factor: appearance.radius,
                end: [end.x, end.y, end.z],
                material_type: material.role as u32,
                color: appearance.color,
            });
        }

        instances
    }
}
