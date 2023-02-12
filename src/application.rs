use std::{fs, iter};
use std::time::SystemTime;

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

use crate::build::tenscript::{FabricPlan, FaceAlias, Spin};
use crate::camera::Target::{FabricMidpoint, Hold, Origin, SelectedFace};
use crate::controls::{ControlMessage, GUI, VisibleControl};
use crate::controls::Action;
use crate::controls::strain_threshold::StrainThresholdMessage;
use crate::crucible::Crucible;
use crate::graphics::GraphicsWindow;
use crate::scene::Scene;

struct Application {
    graphics: GraphicsWindow,
    scene: Scene,
    gui: GUI,
    crucible: Crucible,
}

impl Application {
    fn new(graphics: GraphicsWindow, window: &Window) -> Application {
        let gui = GUI::new(&graphics, window);
        let scene = Scene::new(&graphics);
        let crucible = Crucible::default();
        Application {
            graphics,
            scene,
            gui,
            crucible,
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
pub fn run_with(brick_index: Option<usize>) {
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
    if let Some(brick_index) = brick_index {
        app.crucible.capture_prototype(brick_index); // TODO: should be a method on Application
    }
    // TODO: move these into Application
    let mut library_modified = library_modified_timestamp();
    let mut fabric_plan_name: Option<String> = None;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent { ref event, window_id } if window_id == window.id() => {
                app.gui.window_event(event, &window);
                match event {
                    WindowEvent::CloseRequested { .. } => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => app.resize(*physical_size),
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => app.resize(**new_inner_size),
                    WindowEvent::KeyboardInput { .. } => app.handle_keyboard_input(event),
                    WindowEvent::MouseInput { state: ElementState::Released, .. } => app.scene.window_event(event),

                    WindowEvent::MouseInput { .. } |
                    WindowEvent::CursorMoved { .. } |
                    WindowEvent::MouseWheel { .. }
                    if !app.gui.capturing_mouse() => app.scene.window_event(event),

                    _ => {}
                }
            }
            Event::RedrawRequested(_) => {
                app.crucible.iterate();
                if let Some(action) = app.crucible.action() {
                    app.gui.queue_message(ControlMessage::Action(action))
                }
                app.scene.update(&app.graphics, app.gui.controls().variation(app.scene.target_face_id()), app.crucible.fabric());
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
                let mut actions = app.gui.controls().take_actions();
                if library_modified_timestamp() > library_modified && let Some(ref plan_name) = fabric_plan_name {
                    let fabric_plan = FabricPlan::load_preset(plan_name).expect("no such fabric plan");
                    actions.push(Action::BuildFabric(fabric_plan));
                    library_modified = library_modified_timestamp();
                }
                for action in actions {
                    match action {
                        Action::BuildFabric(fabric_plan) => {
                            fabric_plan_name = Some(fabric_plan.name.clone());
                            app.scene.show_surface(false);
                            app.gui.queue_message(ControlMessage::Reset);
                            app.crucible.build_fabric(fabric_plan);
                        }
                        Action::GravityChanged(_gravity) => {
                            // TODO
                        }
                        Action::CalibrateStrain => {
                            let strain_limits = app.crucible.strain_limits();
                            app.gui.queue_message(ControlMessage::StrainThreshold(StrainThresholdMessage::SetStrainLimits(strain_limits)))
                        }
                        Action::SelectFace(face_id) => {
                            app.scene.select_face(Some(face_id));
                        }
                        Action::AddBrick { face_alias, face_id } => {
                            app.scene.select_face(None);
                            app.crucible.add_brick(face_alias, face_id)
                        }
                        Action::ShowSurface => {
                            app.scene.show_surface(true)
                        }
                    }
                }
                window.request_redraw();
            }
            _ => {}
        }
    });
}


impl Application {
    fn handle_keyboard_input(&mut self, event: &WindowEvent) {
        let WindowEvent::KeyboardInput {
            input: KeyboardInput {
                virtual_keycode: Some(keycode),
                state: ElementState::Pressed, ..
            }, ..
        } = event else {
            return;
        };
        match keycode {
            VirtualKeyCode::Escape => self.gui.queue_message(ControlMessage::ShowControl(VisibleControl::ControlChoice)),
            VirtualKeyCode::D => self.gui.queue_message(ControlMessage::ToggleDebugMode),
            VirtualKeyCode::Key0 => self.crucible.set_speed(0),
            VirtualKeyCode::Key1 => self.crucible.set_speed(1),
            VirtualKeyCode::Key2 => self.crucible.set_speed(5),
            VirtualKeyCode::Key3 => self.crucible.set_speed(25),
            VirtualKeyCode::Key4 => self.crucible.set_speed(125),
            VirtualKeyCode::Key5 => self.crucible.set_speed(625),
            VirtualKeyCode::B => self.create_brick(),
            VirtualKeyCode::F => self.select_next_face(),
            VirtualKeyCode::M => self.scene.camera.target = FabricMidpoint,
            VirtualKeyCode::O => self.scene.camera.target = Origin,
            _ => {}
        }
    }

    fn select_next_face(&mut self) {
        let fabric = self.crucible.fabric();
        self.scene.select_face(Some(match self.scene.camera.target {
            Origin | FabricMidpoint | Hold => {
                *fabric.faces.keys().next().unwrap()
            }
            SelectedFace(face_id) => {
                let face_position = fabric.faces.keys()
                    .position(|&id| face_id == id)
                    .expect("Face id not found");
                let &new_face_id = fabric.faces.keys()
                    .cycle()
                    .nth(face_position + 1)
                    .unwrap();
                new_face_id
            }
        }))
    }

    fn create_brick(&mut self) {
        let Some(face_id) = self.scene.target_face_id() else {
            return;
        };
        let face_alias = match self.crucible.fabric().face(face_id).spin.opposite() {
            Spin::Left => FaceAlias("Left::Bot".to_string()),
            Spin::Right => FaceAlias("Right::Bot".to_string()),
        };
        self.gui.queue_message(ControlMessage::Action(
            Action::AddBrick { face_alias, face_id }
        ));
    }
}

fn library_modified_timestamp() -> SystemTime {
    fs::metadata("./src/build/tenscript/library.scm")
        .unwrap()
        .modified()
        .unwrap()
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
