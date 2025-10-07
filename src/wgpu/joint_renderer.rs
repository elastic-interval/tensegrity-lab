use crate::camera::Pick;
use crate::fabric::Fabric;
use crate::wgpu::{Wgpu, DEFAULT_PRIMITIVE_STATE, default_depth_stencil_state, vertex_layout_f32x8};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use wgpu::PipelineCompilationOptions;

// Instance data for joint markers (spheres)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct JointMarkerInstance {
    position: [f32; 3], // Position of the joint
    scale: f32,         // Size of the marker
    color: [f32; 4],    // RGBA color
}

pub struct JointRenderer {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    instance_buffer: Option<wgpu::Buffer>,
    render_pipeline: wgpu::RenderPipeline,
    num_indices: u32,
    num_instances: u32,
}

impl JointRenderer {
    pub fn new(wgpu: &Wgpu) -> Self {
        // Create a simple sphere for joint visualization
        let (vertex_buffer, index_buffer, num_indices) = create_sphere(wgpu);

        // Use the consolidated shader module
        let shader = wgpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Joint Marker Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            });

        // Create the pipeline layout
        let pipeline_layout = wgpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Joint Marker Pipeline Layout"),
                bind_group_layouts: &[&wgpu.uniform_bind_group_layout],
                push_constant_ranges: &[],
            });

        // Define the vertex buffer layout
        let vertex_layout = vertex_layout_f32x8();

        // Define the instance buffer layout
        let instance_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<JointMarkerInstance>() as wgpu::BufferAddress,
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
                label: Some("Joint Marker Pipeline"),
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

        JointRenderer {
            vertex_buffer,
            index_buffer,
            instance_buffer: None,
            render_pipeline,
            num_indices,
            num_instances: 0,
        }
    }

    pub fn update(&mut self, wgpu: &Wgpu, fabric: &Fabric, pick: &Pick) {
        // Create instances for selected joints
        let instances = self.create_instances(fabric, pick);
        self.num_instances = instances.len() as u32;

        // Update instance buffer if there are instances to render
        if self.num_instances > 0 {
            self.instance_buffer = Some(wgpu.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Joint Marker Instance Buffer"),
                    contents: bytemuck::cast_slice(&instances),
                    usage: wgpu::BufferUsages::VERTEX,
                },
            ));
        }
    }

    fn create_instances(&self, _fabric: &Fabric, _pick: &Pick) -> Vec<JointMarkerInstance> {
        // Return an empty vector - no joint spheres will be displayed
        Vec::new()
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

// Helper function to create a simple sphere for visualization
pub fn create_sphere(wgpu: &Wgpu) -> (wgpu::Buffer, wgpu::Buffer, u32) {
    // For simplicity, we'll create a low-poly sphere
    let radius = 1.0;
    let sectors = 12;
    let stacks = 12;

    // Vertex format: (position[3], normal[3], uv[2])
    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Vertex {
        position: [f32; 3],
        normal: [f32; 3],
        uv: [f32; 2],
    }

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Generate vertices
    for i in 0..=stacks {
        let stack_angle = std::f32::consts::PI * (i as f32) / (stacks as f32);
        let xy = radius * stack_angle.sin();
        let z = radius * stack_angle.cos();

        for j in 0..=sectors {
            let sector_angle = 2.0 * std::f32::consts::PI * (j as f32) / (sectors as f32);
            let x = xy * sector_angle.cos();
            let y = xy * sector_angle.sin();

            // Position
            let position = [x, y, z];

            // Normal (normalized position for a sphere)
            let length = (x * x + y * y + z * z).sqrt();
            let normal = [x / length, y / length, z / length];

            // UV coordinates
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
            // 2 triangles per sector
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

    // Create vertex and index buffers
    let vertex_buffer = wgpu
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sphere Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

    let index_buffer = wgpu
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sphere Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

    (vertex_buffer, index_buffer, indices.len() as u32)
}
