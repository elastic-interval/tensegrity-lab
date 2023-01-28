use std::iter;

use iced_wgpu::wgpu;
#[allow(unused_imports)]
use log::info;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit::dpi::PhysicalSize;
use winit::window::Window;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::controls::{GUI, Message, VisibleControl};
use crate::controls::Action;
use crate::experiment::Experiment;
use crate::graphics::GraphicsWindow;
use crate::scene::Scene;

struct Application {
    graphics: GraphicsWindow,
    scene: Scene,
    gui: GUI,
}

impl Application {
    fn new(graphics: GraphicsWindow, window: &Window) -> Application {
        let gui = GUI::new(&graphics, window);
        let scene = Scene::new(&graphics);
        Application {
            graphics,
            scene,
            gui,
        }
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.graphics.size = new_size;
            self.graphics.config.width = new_size.width;
            self.graphics.config.height = new_size.height;
            self.graphics.surface.configure(&self.graphics.device, &self.graphics.config);
            self.scene.resize(&self.graphics);
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.graphics.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = self.graphics.create_depth_view();
        let mut encoder = self.graphics.create_command_encoder();
        self.scene.render(
            &mut encoder,
            &view,
            &depth_view,
        );
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

    window.set_title("Tensegrity Lab");
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
    let mut app = Application::new(graphics, &window);
    let mut experiment = Experiment::default();

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent { ref event, window_id } if window_id == window.id() => {
                app.gui.window_event(&window, event);
                match event {
                    WindowEvent::CloseRequested { .. } => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => {
                        app.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        app.resize(**new_inner_size);
                    }
                    WindowEvent::KeyboardInput {
                        input: KeyboardInput {
                            virtual_keycode: Some(keycode),
                            state: ElementState::Pressed, ..
                        }, ..
                    } => {
                        match keycode {
                            #[cfg(target_arch = "wasm32")]
                            VirtualKeyCode::F => {
                                fullscreen_web();
                            }
                            VirtualKeyCode::Escape => {
                                app.gui.change_state(Message::ShowControl(VisibleControl::ControlChoice));
                            }
                            VirtualKeyCode::Space => {
                                experiment.toggle_pause();
                            }
                            VirtualKeyCode::D => {
                                app.gui.change_state(Message::ToggleDebugMode);
                            }
                            _ => {}
                        }
                    }
                    WindowEvent::MouseInput { .. } | WindowEvent::CursorMoved { .. } | WindowEvent::MouseWheel { .. }
                    if !app.gui.capturing_mouse() => {
                        app.scene.window_event(event)
                    }
                    _ => {}
                }
            }
            Event::RedrawRequested(_) => {
                experiment.iterate();
                if let Some(jump) = experiment.camera_jump() {
                    app.scene.move_camera(jump);
                    app.scene.show_surface();
                }
                let message = app.scene.update(&app.graphics, app.gui.controls(), experiment.fabric());
                if let Some(message) = message {
                    app.gui.change_state(message)
                }
                app.gui.update_viewport(&window);
                match app.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => app.resize(app.graphics.size),
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    Err(e) => eprintln!("{e:?}"),
                }
                window.set_cursor_icon(app.gui.cursor_icon());
            }
            Event::MainEventsCleared => {
                app.gui.update();
                for action in app.gui.controls().take_actions() {
                    match action {
                        Action::BuildFabric(fabric_plan) => {
                            app.gui.change_state(Message::ShowControl(VisibleControl::ControlChoice));
                            experiment.build_fabric(fabric_plan);
                        }
                        Action::AddPulls { strain_nuance } => {
                            let maximum = experiment.fabric().max_measure_strain();
                            experiment.add_pulls(strain_nuance * maximum);
                        }
                    }
                }
                window.request_redraw();
            }
            _ => {}
        }
    });
}

fn fullscreen_web() {
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
