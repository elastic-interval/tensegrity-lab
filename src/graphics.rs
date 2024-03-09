use winit::window::Window;

pub struct Graphics<'a> {
    pub config: wgpu::SurfaceConfiguration,
    pub surface: wgpu::Surface<'a>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl<'a> Graphics<'a> {
    pub async fn new(window: &'a Window) -> Self {
        let mut size = window.inner_size();
        size.width = size.width.max(1);
        size.height = size.height.max(1);
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance.create_surface(window).unwrap();
        let adapter_fut = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
        });
        let adapter = adapter_fut.await.expect("Could not request adapter");
        log::info!("YOLOSWAG");

        #[cfg(target_arch = "wasm32")]
        let required_limits =
            wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits());

        #[cfg(not(target_arch = "wasm32"))]
        let required_limits = wgpu::Limits::default();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: Default::default(),
                    required_limits,
                },
                None,
            )
            .await
            .expect("Could not request device");
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            alpha_mode: surface_caps.alpha_modes[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            view_formats: vec![],
            desired_maximum_frame_latency: 2u32,
        };
        surface.configure(&device, &config);
        Self {
            surface,
            device,
            queue,
            config,
        }
    }
}
