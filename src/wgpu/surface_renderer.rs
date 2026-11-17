use crate::wgpu::surface_vertex::SurfaceVertex;
use crate::wgpu::Wgpu;
use crate::wgpu::DEFAULT_PRIMITIVE_STATE;
use bytemuck::{cast_slice, Pod, Zeroable};
use glam::Vec3;
use wgpu::util::DeviceExt;
use wgpu::RenderPass;

/// In metres: the surface a fabric starts on.
const MIN_RADIUS: f32 = 0.5;

pub struct SurfaceRenderer {
    pub vertices: Vec<SurfaceVertex>,
    pub pipeline: wgpu::RenderPipeline,
    pub buffer: wgpu::Buffer,
    ground_buffer: wgpu::Buffer,
    ground_bind_group: wgpu::BindGroup,
    current_radius: f32,
}

/// The ground's hexagon lines and the fill between them, linear RGBA, as the
/// shader's `Ground` uniform reads them.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Ground {
    line: [f32; 4],
    fill: [f32; 4],
    shift: [f32; 4],
}

impl Ground {
    fn for_background(white: bool, shift: Vec3) -> [Ground; 1] {
        let shift = shift.extend(0.0).into();
        [if white {
            Ground { line: [0.35, 0.35, 0.35, 1.0], fill: [0.85, 0.85, 0.85, 0.6], shift }
        } else {
            Ground { line: [0.18, 0.19, 0.2, 1.0], fill: [0.012, 0.013, 0.016, 0.9], shift }
        }]
    }
}

impl SurfaceRenderer {
    pub fn new(wgpu: &Wgpu) -> Self {
        let ground_buffer = wgpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ground"),
            contents: cast_slice(&Ground::for_background(false, Vec3::ZERO)),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let ground_bind_group_layout =
            wgpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Ground Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let ground_bind_group = wgpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &ground_bind_group_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: ground_buffer.as_entire_binding() }],
            label: Some("Ground Bind Group"),
        });

        // The camera's uniform, then the ground's colours
        let pipeline_layout = wgpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Surface Pipeline Layout"),
                bind_group_layouts: &[&wgpu.uniform_bind_group_layout, &ground_bind_group_layout],
                immediate_size: 0,
            });

        let surface_vertices = SurfaceVertex::for_radius(MIN_RADIUS).to_vec();
        let surface_pipeline =
            wgpu.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Surface Pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &wgpu.shader,
                        entry_point: Some("surface_vertex"),
                        compilation_options: Default::default(),
                        buffers: &[SurfaceVertex::desc()],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &wgpu.shader,
                        entry_point: Some("surface_fragment"),
                        compilation_options: wgpu::PipelineCompilationOptions {
                            constants: wgpu.gamma_constants(),
                            ..Default::default()
                        },
                        targets: &[Some(wgpu::ColorTargetState {
                            format: wgpu.surface_configuration.format,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: DEFAULT_PRIMITIVE_STATE,
                    depth_stencil: Some(crate::wgpu::default_depth_stencil_state()),
                    multisample: wgpu::MultisampleState::default(),
                    multiview_mask: None,
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
            ground_buffer,
            ground_bind_group,
            current_radius: MIN_RADIUS,
        }
    }

    /// Grow the surface to about 1.5x the fabric's bounding radius when the
    /// fabric outgrows it, by more than 5%. It never shrinks while the same
    /// fabric is shown: the ground does not change size as the figure moves.
    pub fn update_radius(&mut self, queue: &wgpu::Queue, fabric_bounding_radius: f32) {
        let new_radius = (fabric_bounding_radius * 1.5).max(MIN_RADIUS);
        if new_radius > self.current_radius * 1.05 {
            self.current_radius = new_radius;
            self.vertices = SurfaceVertex::for_radius(new_radius).to_vec();
            queue.write_buffer(&self.buffer, 0, cast_slice(&self.vertices));
        }
    }

    /// Back to the smallest surface, for a fabric being built anew.
    pub fn reset_radius(&mut self, queue: &wgpu::Queue) {
        self.current_radius = MIN_RADIUS;
        self.vertices = SurfaceVertex::for_radius(MIN_RADIUS).to_vec();
        queue.write_buffer(&self.buffer, 0, cast_slice(&self.vertices));
    }

    /// The colours for the white background, or the starry sky's.
    pub fn set_white(&self, queue: &wgpu::Queue, white: bool) {
        queue.write_buffer(&self.ground_buffer, 0, cast_slice(&Ground::for_background(white, Vec3::ZERO)));
    }

    /// Move the ground as far as a quake has moved it.
    pub fn set_shift(&self, queue: &wgpu::Queue, shift: Vec3) {
        let offset = std::mem::offset_of!(Ground, shift) as u64;
        queue.write_buffer(&self.ground_buffer, offset, cast_slice(&shift.extend(0.0).to_array()));
    }

    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(1, &self.ground_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.buffer.slice(..));
        render_pass.draw(0..self.vertices.len() as u32, 0..1);
    }
}
