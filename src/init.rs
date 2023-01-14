use std::{iter, mem};

use bytemuck::{cast_slice, Pod, Zeroable};
use iced_wgpu::wgpu;
#[allow(unused_imports)]
use log::info;
use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use gui::GUI;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::camera::Camera;
use crate::experiment::Experiment;
use crate::fabric::Fabric;
use crate::graphics::{get_depth_stencil_state, get_primitive_state, GraphicsWindow};
use crate::gui;
use crate::interval::Interval;
use crate::interval::Role::{Measure, Pull, Push};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Default)]
struct Vertex {
    position: [f32; 4],
    color: [f32; 4],
}

impl Vertex {
    pub fn for_interval(interval: &Interval, fabric: &Fabric) -> [Vertex; 2] {
        let (alpha, omega) = interval.locations(&fabric.joints);
        let color = match interval.role {
            Push => [1.0, 1.0, 1.0, 1.0],
            Pull => [0.2, 0.2, 1.0, 1.0],
            Measure => if interval.strain < 0.0 {
                [1.0, 0.8, 0.0, 1.0]
            } else {
                [0.0, 1.0, 0.0, 1.0]
            },
        };
        [
            Vertex { position: [alpha.x, alpha.y, alpha.z, 1.0], color },
            Vertex { position: [omega.x, omega.y, omega.z, 1.0], color }
        ]
    }

    const ATTRIBUTES: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0=>Float32x4, 1=>Float32x4];
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

struct State {
    vertices: Vec<Vertex>,
    graphics: GraphicsWindow,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    camera: Camera,
    gui: GUI,
}

impl State {
    fn new(graphics: GraphicsWindow, window: &Window) -> State {
        let shader = graphics.get_shader_module();
        let scale = 3.0;
        let aspect = graphics.config.width as f32 / graphics.config.height as f32;
        let camera = Camera::new((3.0 * scale, 1.5 * scale, 3.0 * scale).into(), aspect);
        let mvp_mat = camera.mvp_matrix();
        let mvp_ref: &[f32; 16] = mvp_mat.as_ref();
        let uniform_buffer = graphics.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("MVP"),
            contents: cast_slice(mvp_ref),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let uniform_bind_group_layout = graphics.create_uniform_bind_group_layout();
        let uniform_bind_group = graphics.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("Uniform Bind Group"),
        });

        let pipeline_layout = graphics.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = graphics.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: graphics.config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: get_primitive_state(),
            depth_stencil: Some(get_depth_stencil_state()),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let vertices = vec![Vertex::default(); 10000]; // TODO: why 1000?
        let vertex_buffer = graphics.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let gui = GUI::new(&graphics, window);

        State {
            vertices,
            graphics,
            pipeline,
            vertex_buffer,
            uniform_buffer,
            uniform_bind_group,
            camera,
            gui,
        }
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.graphics.size = new_size;
            self.graphics.config.width = new_size.width;
            self.graphics.config.height = new_size.height;
            self.graphics.surface.configure(&self.graphics.device, &self.graphics.config);
            let aspect = new_size.width as f32 / new_size.height as f32;
            self.camera.set_aspect(aspect);
            let mvp_mat = self.camera.mvp_matrix();
            let mvp_ref: &[f32; 16] = mvp_mat.as_ref();
            self.graphics.queue.write_buffer(&self.uniform_buffer, 0, cast_slice(mvp_ref));
        }
    }

    fn update_from_fabric(&mut self, fabric: &Fabric) {
        let num_vertices = fabric.intervals.len() * 2;
        if self.vertices.len() != num_vertices {
            self.vertices = vec![Vertex::default(); num_vertices];
        }
        let updated_vertices = fabric.interval_values()
            .flat_map(|interval| Vertex::for_interval(interval, fabric));
        for (vertex, slot) in updated_vertices.zip(self.vertices.iter_mut()) {
            *slot = vertex;
        }
        self.camera.target_approach(fabric.midpoint())
    }

    fn update(&mut self, fabric: &Fabric) {
        let mvp_mat = self.camera.mvp_matrix();
        let mvp_ref: &[f32; 16] = mvp_mat.as_ref();
        self.update_from_fabric(fabric);
        self.graphics.queue.write_buffer(&self.uniform_buffer, 0, cast_slice(mvp_ref));
        self.graphics.queue.write_buffer(&self.vertex_buffer, 0, cast_slice(&self.vertices));
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.graphics.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = self.graphics.create_depth_view();
        let mut encoder = self.graphics.create_command_encoder();
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: false,
                    }),
                    stencil_ops: None,
                }),
            });
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.draw(0..self.vertices.len() as u32, 0..1);
        }
        self.gui.render(
            &self.graphics.device,
            &mut encoder,
            &view,
        );
        self.graphics.queue.submit(iter::once(encoder.finish()));
        output.present();
        self.gui.post_render();
        Ok(())
    }
}


#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Info).expect("Couldn't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(2048, 1600))
        .build(&event_loop)
        .expect("Could not build window");

    window.set_title("Elastic Interval Geometry");
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| {
                let [width, height] = [win.inner_width(), win.inner_height()]
                    .map(|x| x.unwrap().as_f64().unwrap() * 2.0);
                window.set_inner_size(PhysicalSize::new(width, height));
                let doc = win.document()?;
                let dst = doc.get_element_by_id("body")?;
                let canvas = web_sys::Element::from(window.canvas());
                canvas.set_id("canvas");
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }
    let graphics = pollster::block_on(GraphicsWindow::new(&window));
    let mut state = State::new(graphics, &window);
    let mut experiment = Experiment::default();

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                state.gui.window_event(&window, event);
                match event {
                    WindowEvent::CloseRequested | WindowEvent::KeyboardInput {
                        input: KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }
                    WindowEvent::KeyboardInput { input, .. } =>
                        match input.virtual_keycode {
                            Some(VirtualKeyCode::F) => {
                                #[cfg(target_arch = "wasm32")]
                                web_sys::window()
                                    .and_then(|win| {
                                        let document = win.document()?;
                                        if document.fullscreen_element().is_none() {
                                            let canvas = document.get_element_by_id("canvas")?;
                                            match canvas.request_fullscreen() {
                                                Ok(_) => {}
                                                Err(e) => {
                                                    info!("Could not request fullscreen: {e:?}");
                                                }
                                            }
                                        } else {
                                            document.exit_fullscreen();
                                        }
                                        Some(())
                                    });
                            }
                            _ => {}
                        },
                    WindowEvent::MouseInput { .. } | WindowEvent::CursorMoved { .. } | WindowEvent::MouseWheel { .. }
                    if !state.gui.capturing_mouse() => {
                        state.camera.window_event(event)
                    }
                    _ => {}
                }
            }
            Event::RedrawRequested(_) => {
                let up = experiment.iterate();
                if let Some(up) = up {
                    state.camera.go_up(up);
                }
                state.update(experiment.fabric());
                state.gui.update_viewport(&window);
                match state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.graphics.size),
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    Err(e) => eprintln!("{e:?}"),
                }
            }
            Event::MainEventsCleared => {
                state.gui.update();

                window.request_redraw();
            }
            _ => {}
        }
    });
}
