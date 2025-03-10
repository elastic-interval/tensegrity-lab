use crate::fabric::MAX_INTERVALS;
use crate::wgpu::fabric_vertex::FabricVertex;
use crate::wgpu::Wgpu;
use bytemuck::cast_slice;
use wgpu::util::DeviceExt;
use wgpu::{Device, PipelineLayout, RenderPass, ShaderModule, SurfaceConfiguration};

pub struct FabricRenderer {
    pub vertices: Vec<FabricVertex>,
    pub pipeline: wgpu::RenderPipeline,
    pub buffer: [wgpu::Buffer; 1],
}

impl FabricRenderer {
    pub fn new(
        device: &Device,
        pipeline_layout: &PipelineLayout,
        module: &ShaderModule,
        surface_configuration: &SurfaceConfiguration,
    ) -> Self {
        let vertices = vec![FabricVertex::default(); MAX_INTERVALS * 2];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Fabric Pipeline"),
            layout: Some(pipeline_layout),
            vertex: wgpu::VertexState {
                module,
                entry_point: Some("fabric_vertex"),
                buffers: &[FabricVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module,
                entry_point: Some("fabric_fragment"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_configuration.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let buffer = [vertex_buffer];
        Self {
            vertices,
            pipeline,
            buffer,
        }
    }

    pub fn update(&mut self, wgpu: &mut Wgpu, vertexes: impl IntoIterator<Item = FabricVertex>) {
        self.vertices.clear();
        self.vertices.extend(vertexes);
        wgpu.queue
            .write_buffer(&self.buffer[0], 0, cast_slice(&self.vertices));
    }

    pub fn draw(&mut self, render_pass: &mut RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.buffer[0].slice(..));
        render_pass.draw(0..self.vertices.len() as u32, 0..1);
    }
}
