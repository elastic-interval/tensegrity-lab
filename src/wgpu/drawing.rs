use bytemuck::cast_slice;
use wgpu::util::DeviceExt;
use crate::fabric::MAX_INTERVALS;
use crate::wgpu::fabric_vertex::FabricVertex;
use crate::wgpu::surface_vertex::SurfaceVertex;
use crate::wgpu::Wgpu;

pub struct Drawing<V> {
    pub(crate) vertices: Vec<V>,
    pub(crate) pipeline: wgpu::RenderPipeline,
    pub(crate) buffer: wgpu::Buffer,
}

impl Wgpu {
    pub fn create_fabric_drawing(&self) -> Drawing<FabricVertex> {
        let vertices = vec![FabricVertex::default(); MAX_INTERVALS * 2];
        let pipeline = self.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Fabric Pipeline"),
                layout: Some(&self.pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &self.shader,
                    entry_point: "fabric_vertex",
                    buffers: &[FabricVertex::desc()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &self.shader,
                    entry_point: "fabric_fragment",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: self.surface_config.format,
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
        let buffer = self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        Drawing {
            vertices,
            pipeline,
            buffer,
        }
    }

    pub fn create_surface_drawing(&self) -> Drawing<SurfaceVertex> {
        let surface_vertices = SurfaceVertex::for_radius(10.0).to_vec();
        let surface_pipeline = self.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Surface Pipeline"),
                layout: Some(&self.pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &self.shader,
                    entry_point: "surface_vertex",
                    compilation_options: Default::default(),
                    buffers: &[SurfaceVertex::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &self.shader,
                    entry_point: "surface_fragment",
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: self.surface_config.format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });
        let surface_buffer = self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Surface Buffer"),
                contents: cast_slice(&surface_vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        Drawing {
            vertices: surface_vertices,
            pipeline: surface_pipeline,
            buffer: surface_buffer,
        }
    }
}
