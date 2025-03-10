use std::fmt::{Debug, Formatter};
use std::sync::Arc;

use bytemuck::cast_slice;
use cgmath::Matrix4;
use wgpu::util::DeviceExt;
use wgpu::MemoryHints::Performance;
use wgpu::{PipelineLayout, ShaderModule, TextureFormat};
use winit::event_loop::EventLoopProxy;
use winit::window::Window;

use crate::camera::Camera;
use crate::messages::LabEvent;

pub mod fabric_vertex;
pub mod surface_vertex;
pub mod surface_renderer;
pub mod fabric_renderer;

pub struct Wgpu {
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub device: wgpu::Device,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub pipeline_layout: PipelineLayout,
    pub shader: ShaderModule,
}

impl Debug for Wgpu {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "WgpuContext")
    }
}

impl Clone for Wgpu {
    fn clone(&self) -> Self {
        panic!("Clone of WgpuContext")
    }

    fn clone_from(&mut self, _source: &Self) {
        panic!("Clone of WgpuContext")
    }
}

impl Wgpu {
    pub async fn new_async(window: Arc<Window>) -> Wgpu {
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
        let shader =
            device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Shader"),
                    source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
                });
        Self {
            surface,
            surface_config,
            device,
            queue,
            uniform_buffer,
            uniform_bind_group,
            pipeline_layout,
            shader,
        }
    }

    pub fn create_and_send(window: Arc<Window>, event_loop_proxy: EventLoopProxy<LabEvent>) {
        #[cfg(target_arch = "wasm32")]
        {
            let future = Self::new_async(window);
            wasm_bindgen_futures::spawn_local(async move {
                let wgpu = future.await;
                assert!(event_loop_proxy.send_event(LabEvent::ContextCreated(wgpu)).is_ok());
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let wgpu = futures::executor::block_on(Self::new_async(window));
            assert!(event_loop_proxy.send_event(LabEvent::ContextCreated(wgpu)).is_ok());
        }
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

    pub fn create_camera(&self) -> Camera {
        let scale = 6.0;
        Camera::new(
            (2.0 * scale, 1.0 * scale, 2.0 * scale).into(),
            self.surface_config.width as f32,
            self.surface_config.height as f32,
        )
    }

    pub fn format(&self) -> TextureFormat {
        self.surface_config.format
    }
}

