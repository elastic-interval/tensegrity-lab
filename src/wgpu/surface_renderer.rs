use crate::wgpu::surface_vertex::SurfaceVertex;
use crate::wgpu::Wgpu;
use bytemuck::cast_slice;
use wgpu::util::DeviceExt;
use wgpu::RenderPass;

pub struct SurfaceRenderer {
    pub vertices: Vec<SurfaceVertex>,
    pub pipeline: wgpu::RenderPipeline,
    pub buffer: wgpu::Buffer,
}

impl SurfaceRenderer {
    pub fn new(wgpu: &Wgpu) -> Self {
        let surface_vertices = SurfaceVertex::for_radius(10.0).to_vec();
        let surface_pipeline = wgpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Surface Pipeline"),
            layout: Some(&wgpu.pipeline_layout),
            vertex: wgpu::VertexState {
                module: &wgpu.shader,
                entry_point: Some("surface_vertex"),
                compilation_options: Default::default(),
                buffers: &[SurfaceVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &wgpu.shader,
                entry_point: Some("surface_fragment"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu.surface_configuration.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu.create_depth_stencil()),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let surface_buffer = wgpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Surface Buffer"),
            contents: cast_slice(&surface_vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            vertices: surface_vertices,
            pipeline: surface_pipeline,
            buffer: surface_buffer,
        }
    }

    pub fn render(&mut self, render_pass: &mut RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.buffer.slice(..));
        render_pass.draw(0..self.vertices.len() as u32, 0..1);
    }
}
