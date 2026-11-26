use crate::wgpu::cylinder_renderer::cylinder_instance_layout;
use crate::wgpu::fabric_renderer::FabricRenderer;
use crate::wgpu::{Wgpu, DEFAULT_PRIMITIVE_STATE, SPOTS, UNIFORM_FLOATS};
use bytemuck::cast_slice;
use glam::Mat4;
use wgpu::util::DeviceExt;

/// The gallery's shadow maps: the pieces drawn as each spot sees them, only
/// how far, for the room and pieces to tell where they are in shadow.
pub struct ShadowRenderer {
    pipeline: wgpu::RenderPipeline,
    /// Each spot's view, and the bind group reaching it.
    spots: Vec<(wgpu::Buffer, wgpu::BindGroup)>,
    layers: Vec<wgpu::TextureView>,
}

impl ShadowRenderer {
    pub fn new(wgpu: &Wgpu) -> Self {
        let layout = wgpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Shadow Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let pipeline_layout = wgpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shadow Pipeline Layout"),
            bind_group_layouts: &[&layout],
            immediate_size: 0,
        });
        let pipeline = wgpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Shadow Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: wgpu.shader(),
                entry_point: Some("fabric_vertex"),
                compilation_options: Default::default(),
                buffers: &[Wgpu::cylinder_vertex_layout(), cylinder_instance_layout()],
            },
            fragment: None,
            primitive: wgpu::PrimitiveState { cull_mode: None, ..DEFAULT_PRIMITIVE_STATE },
            depth_stencil: Some(wgpu::DepthStencilState {
                bias: wgpu::DepthBiasState { constant: 2, slope_scale: 2.0, clamp: 0.0 },
                ..crate::wgpu::default_depth_stencil_state()
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let spots = (0..SPOTS)
            .map(|_| {
                let buffer = wgpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Spot View"),
                    contents: cast_slice(&[0.0f32; UNIFORM_FLOATS]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
                let bind_group = wgpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Spot View"),
                    layout: &layout,
                    entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }],
                });
                (buffer, bind_group)
            })
            .collect();
        let layers = (0..SPOTS)
            .map(|layer| {
                wgpu.shadow_texture.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_array_layer: layer,
                    array_layer_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        Self { pipeline, spots, layers }
    }

    /// Each spot's shadow map: the fabric as `views` see it.
    pub fn render(&self, wgpu: &Wgpu, encoder: &mut wgpu::CommandEncoder, fabric: &FabricRenderer, views: [Mat4; SPOTS as usize]) {
        for ((buffer, bind_group), (layer, view)) in self.spots.iter().zip(self.layers.iter().zip(views)) {
            wgpu.queue.write_buffer(buffer, 0, cast_slice(&view.to_cols_array()));
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Shadow Pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: layer,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            fabric.render_shadow(&mut pass, &self.pipeline, bind_group);
        }
    }
}
