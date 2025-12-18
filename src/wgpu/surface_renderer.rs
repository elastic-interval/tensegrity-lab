use crate::wgpu::surface_vertex::SurfaceVertex;
use crate::wgpu::Wgpu;
use crate::wgpu::DEFAULT_PRIMITIVE_STATE;
use bytemuck::cast_slice;
use wgpu::util::DeviceExt;
use wgpu::RenderPass;

pub struct SurfaceRenderer {
    pub vertices: Vec<SurfaceVertex>,
    pub pipeline: wgpu::RenderPipeline,
    pub buffer: wgpu::Buffer,
    current_radius: f32,
}

impl SurfaceRenderer {
    pub fn new(wgpu: &Wgpu) -> Self {
        let surface_vertices = SurfaceVertex::for_radius(10.0).to_vec();
        let surface_pipeline =
            wgpu.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                    primitive: DEFAULT_PRIMITIVE_STATE,
                    depth_stencil: Some(crate::wgpu::default_depth_stencil_state()),
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                });
        let surface_buffer = wgpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Surface Buffer"),
                contents: cast_slice(&surface_vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        Self {
            vertices: surface_vertices,
            pipeline: surface_pipeline,
            buffer: surface_buffer,
            current_radius: 10.0,
        }
    }

    /// Update the surface radius based on fabric bounding radius.
    /// Surface should be about 2x the fabric size for good visualization.
    pub fn update_radius(&mut self, queue: &wgpu::Queue, fabric_bounding_radius: f32) {
        // Surface radius = 2x fabric bounding radius, with a minimum of 0.5m
        let new_radius = (fabric_bounding_radius * 2.0).max(0.5);

        // Only update if radius changed significantly (>5% change)
        let radius_change = (new_radius - self.current_radius).abs() / self.current_radius;
        if radius_change > 0.05 {
            self.current_radius = new_radius;
            self.vertices = SurfaceVertex::for_radius(new_radius).to_vec();
            queue.write_buffer(&self.buffer, 0, cast_slice(&self.vertices));
        }
    }

    pub fn render(&mut self, render_pass: &mut RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.buffer.slice(..));
        render_pass.draw(0..self.vertices.len() as u32, 0..1);
    }
}
