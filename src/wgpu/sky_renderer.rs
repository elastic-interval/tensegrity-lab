use crate::wgpu::Wgpu;
use bytemuck::{cast_slice, Pod, Zeroable};
use wgpu::util::DeviceExt;
use wgpu::RenderPass;

/// Fullscreen quad vertex (clip space position + UV)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct SkyVertex {
    position: [f32; 2],
    uv: [f32; 2],
}

impl SkyVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SkyVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

/// Fullscreen quad covering the entire screen
const FULLSCREEN_QUAD: [SkyVertex; 6] = [
    // Triangle 1
    SkyVertex {
        position: [-1.0, -1.0],
        uv: [0.0, 1.0],
    },
    SkyVertex {
        position: [1.0, -1.0],
        uv: [1.0, 1.0],
    },
    SkyVertex {
        position: [1.0, 1.0],
        uv: [1.0, 0.0],
    },
    // Triangle 2
    SkyVertex {
        position: [-1.0, -1.0],
        uv: [0.0, 1.0],
    },
    SkyVertex {
        position: [1.0, 1.0],
        uv: [1.0, 0.0],
    },
    SkyVertex {
        position: [-1.0, 1.0],
        uv: [0.0, 0.0],
    },
];

pub struct SkyRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    time_buffer: wgpu::Buffer,
    time_bind_group: wgpu::BindGroup,
    time: f32,
}

impl SkyRenderer {
    pub fn new(wgpu: &Wgpu) -> Self {
        // Create time uniform buffer
        let time_buffer = wgpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Sky Time Buffer"),
                contents: cast_slice(&[0.0f32]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create bind group layout for time uniform
        let time_bind_group_layout =
            wgpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Sky Time Bind Group Layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        // Create bind group
        let time_bind_group = wgpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &time_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: time_buffer.as_entire_binding(),
            }],
            label: Some("Sky Time Bind Group"),
        });

        // Create pipeline layout
        let pipeline_layout =
            wgpu.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Sky Pipeline Layout"),
                    bind_group_layouts: &[&time_bind_group_layout],
                    immediate_size: 0,
                });

        // Create vertex buffer
        let vertex_buffer = wgpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Sky Vertex Buffer"),
                contents: cast_slice(&FULLSCREEN_QUAD),
                usage: wgpu::BufferUsages::VERTEX,
            });

        // Create render pipeline
        let pipeline = wgpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Sky Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &wgpu.shader,
                    entry_point: Some("sky_vertex"),
                    compilation_options: Default::default(),
                    buffers: &[SkyVertex::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &wgpu.shader,
                    entry_point: Some("sky_fragment"),
                    compilation_options: Default::default(),
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
                    cull_mode: None, // Don't cull for fullscreen quad
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                // No depth testing - sky is always behind everything
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });

        Self {
            pipeline,
            vertex_buffer,
            time_buffer,
            time_bind_group,
            time: 0.0,
        }
    }

    /// Update time for twinkling animation
    pub fn update_time(&mut self, queue: &wgpu::Queue, delta_seconds: f32) {
        self.time += delta_seconds;
        queue.write_buffer(&self.time_buffer, 0, cast_slice(&[self.time]));
    }

    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.time_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..6, 0..1);
    }
}
