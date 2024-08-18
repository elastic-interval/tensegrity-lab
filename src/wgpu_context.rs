use std::sync::Arc;

use bytemuck::cast_slice;
use cgmath::Matrix4;
use wgpu::MemoryHints::Performance;
use wgpu::PipelineLayout;
use wgpu::util::DeviceExt;
use winit::window::Window;

pub struct WgpuContext<'window> {
    pub queue: wgpu::Queue,
    surface: wgpu::Surface<'window>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub device: wgpu::Device,
    uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub pipeline_layout: PipelineLayout,
}

impl<'window> WgpuContext<'window> {
    pub async fn new_async(window: Arc<Window>) -> WgpuContext<'window> {
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(Arc::clone(&window)).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                // Request an adapter which can render to our surface
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");
        // Create the logical device and command queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swap chain.
                    required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                    memory_hints: Performance,
                },
                None,
            )
            .await
            .expect("Failed to create device");
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);
        let surface_config = surface.get_default_config(&adapter, width, height).unwrap();
        surface.configure(&device, &surface_config);
        let uniform_buffer = device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("MVP"),
                contents: cast_slice(&[0.0f32; 16]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let uniform_bind_group_layout =
            device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Uniform Bind Group Layout"),
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
        let uniform_bind_group =
            device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &uniform_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform_buffer.as_entire_binding(),
                    }],
                    label: Some("Uniform Bind Group"),
                });
        let pipeline_layout =
            device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[&uniform_bind_group_layout],
                    push_constant_ranges: &[],
                });
        Self {
            surface,
            surface_config,
            device,
            queue,
            uniform_buffer,
            uniform_bind_group,
            pipeline_layout,
        }
    }

    pub fn new(window: Arc<Window>) -> WgpuContext<'window> {
        pollster::block_on(Self::new_async(window))
    }

    pub fn resize(&mut self, new_size: (u32, u32)) {
        let (width, height) = new_size;
        self.surface_config.width = width.max(1);
        self.surface_config.height = height.max(1);
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn create_encoder(&self) -> wgpu::CommandEncoder {
        self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Encoder"),
        })
    }

    pub fn surface_texture(&self) -> Result<wgpu::SurfaceTexture, wgpu::SurfaceError> {
        self.surface.get_current_texture()
    }
    
    pub fn update_mvp_matrix(&self, matrix: Matrix4<f32>) {
        let mvp_ref: &[f32; 16] = matrix.as_ref();
        self.queue.write_buffer(&self.uniform_buffer, 0, cast_slice(mvp_ref));
    }
    
    // pub async fn new(event_loop: &ActiveEventLoop, window_attributes: &WindowAttributes) -> Self {
    //     let window = Arc::new(event_loop.create_window(window_attributes.clone()).unwrap());
    //     let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
    //         backends: wgpu::Backends::all(),
    //         ..Default::default()
    //     });
    //     let surface = instance.create_surface(window.clone()).unwrap();
    //     let adapter = instance
    //         .request_adapter(&wgpu::RequestAdapterOptions {
    //             power_preference: wgpu::PowerPreference::default(),
    //             compatible_surface: Some(&surface),
    //             force_fallback_adapter: false,
    //         })
    //         .await
    //         .expect("Could not request adapter");
    // 
    //     let (device, queue) = adapter
    //         .request_device(
    //             &wgpu::DeviceDescriptor {
    //                 label: None,
    //                 required_features: Default::default(),
    //                 required_limits: Default::default(),
    //                 memory_hints: Default::default(),
    //             },
    //             None,
    //         )
    //         .await
    //         .expect("Could not request device");
    //     let surface_caps = surface.get_capabilities(&adapter);
    //     let surface_format = surface_caps
    //         .formats
    //         .iter()
    //         .copied()
    //         .find(|f| f.is_srgb())
    //         .unwrap_or(surface_caps.formats[0]);
    //     let (width, height) = match window_attributes.inner_size {
    //         Some(Size::Physical(size)) => (size.width, size.height),
    //         _ => (100, 100),
    //     };
    //     let config = wgpu::SurfaceConfiguration {
    //         usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
    //         format: surface_format,
    //         alpha_mode: surface_caps.alpha_modes[0],
    //         width,
    //         height,
    //         present_mode: wgpu::PresentMode::Fifo,
    //         view_formats: vec![],
    //         desired_maximum_frame_latency: 0,
    //     };
    //     surface.configure(&device, &config);
    //     Self {
    //         window,
    //         config,
    //         surface,
    //         device,
    //         queue,
    //     }
    // }
}