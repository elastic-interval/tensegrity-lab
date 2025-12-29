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
    pub texture_bind_group: wgpu::BindGroup,
    current_radius: f32,
}

impl SurfaceRenderer {
    pub fn new(wgpu: &Wgpu) -> Self {
        // Load the surface texture
        let texture_bytes = include_bytes!("../../assets/surface_tile.jpg");
        let img = image::load_from_memory(texture_bytes)
            .expect("Failed to load surface texture")
            .to_rgba8();
        let dimensions = img.dimensions();

        // Create the texture
        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = wgpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Surface Texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Upload the texture data
        wgpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &img,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            texture_size,
        );

        // Create texture view and sampler
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = wgpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Surface Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        // Create bind group layout for texture
        let texture_bind_group_layout =
            wgpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Surface Texture Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        // Create bind group
        let texture_bind_group = wgpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("Surface Texture Bind Group"),
        });

        // Create pipeline layout with both uniform and texture bind groups
        let pipeline_layout =
            wgpu.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Surface Pipeline Layout"),
                    bind_group_layouts: &[&wgpu.uniform_bind_group_layout, &texture_bind_group_layout],
                    immediate_size: 0,
                });

        let surface_vertices = SurfaceVertex::for_radius(10.0).to_vec();
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
            texture_bind_group,
            current_radius: 10.0,
        }
    }

    /// Update the surface radius based on fabric bounding radius.
    /// Surface should be about 1.5x the fabric size for good visualization.
    pub fn update_radius(&mut self, queue: &wgpu::Queue, fabric_bounding_radius: f32) {
        // Surface radius = 1.5x fabric bounding radius, with a minimum of 0.5m
        let new_radius = (fabric_bounding_radius * 1.5).max(0.5);

        // Only update if radius changed significantly (>5% change)
        let radius_change = (new_radius - self.current_radius).abs() / self.current_radius;
        if radius_change > 0.05 {
            self.current_radius = new_radius;
            self.vertices = SurfaceVertex::for_radius(new_radius).to_vec();
            queue.write_buffer(&self.buffer, 0, cast_slice(&self.vertices));
        }
    }

    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(1, &self.texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.buffer.slice(..));
        render_pass.draw(0..self.vertices.len() as u32, 0..1);
    }
}
