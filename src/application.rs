use std::{fs, iter};
use std::time::SystemTime;

use iced_wgpu::wgpu;
use winit::{
    event::*,
};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::build::tenscript::{FabricPlan, FaceAlias, Spin};
use crate::crucible::Crucible;
use crate::graphics::GraphicsWindow;
use crate::user_interface::{Action, UserInterface};
use crate::scene::Scene;

pub struct Application {
    scene: Scene,
    user_interface: UserInterface,
    crucible: Crucible,
    graphics: GraphicsWindow,
    library_modified: SystemTime,
    fabric_plan_name: Option<String>,
}

impl Application {
    pub fn new(graphics: GraphicsWindow, window: &Window) -> Application {
        let user_interface = UserInterface::new(&graphics, window);
        let scene = Scene::new(&graphics);
        Application {
            scene,
            user_interface,
            crucible: Crucible::default(),
            graphics,
            library_modified: library_modified_timestamp(),
            fabric_plan_name: None,
        }
    }

    pub fn update(&mut self, window: &Window) {
        self.user_interface.update();
        let mut actions = self.user_interface.controls().take_actions();
        if library_modified_timestamp() > self.library_modified && let Some(ref plan_name) = self.fabric_plan_name {
            let fabric_plan = FabricPlan::load_preset(plan_name).expect("no such fabric plan");
            actions.push(Action::BuildFabric(fabric_plan));
            self.library_modified = library_modified_timestamp();
        }
        for action in actions {
            match action {
                Action::BuildFabric(fabric_plan) => {
                    self.fabric_plan_name = Some(fabric_plan.name.clone());
                    self.scene.show_surface(false);
                    self.user_interface.reset();
                    self.crucible.build_fabric(fabric_plan);
                }
                Action::GravityChanged(_gravity) => {
                    // TODO
                }
                Action::CalibrateStrain => {
                    self.user_interface.set_strain_limits(self.crucible.strain_limits());
                }
                Action::SelectFace(face_id) => {
                    self.scene.select_next_face(Some(face_id), self.crucible.fabric());
                }
                Action::AddBrick { face_alias, face_id } => {
                    self.scene.clear_face_selection();
                    self.crucible.add_brick(face_alias, face_id);
                }
                Action::ShowSurface => {
                    self.scene.show_surface(true);
                }
                Action::ToggleDebug => {
                    self.user_interface.toggle_debug_mode();
                }
                Action::SetSpeed(speed) => {
                    self.crucible.set_speed(speed);
                }
                Action::CreateBrick => {
                    self.create_brick();
                }
                Action::SelectNextFace => {
                    self.scene.select_next_face(None, self.crucible.fabric());
                }
                Action::WatchMidpoint => {
                    self.scene.watch_midpoint();
                }
                Action::WatchOrigin => {
                    self.scene.watch_origin();
                }
            }
        }
        window.request_redraw();
    }

    pub fn redraw(&mut self, window: &Window) {
        self.crucible.iterate();
        if let Some(action) = self.crucible.action() {
            self.user_interface.action(action);
        }
        self.scene.update(&self.graphics, self.user_interface.controls().variation(self.scene.target_face_id()), self.crucible.fabric());
        self.user_interface.update_viewport(window);
        match self.render() {
            Ok(_) => {}
            Err(wgpu::SurfaceError::Lost) => self.resize(self.graphics.size),
            Err(wgpu::SurfaceError::OutOfMemory) => panic!("Out of memory"),
            Err(e) => eprintln!("{e:?}"),
        }
        let cursor_icon = self.user_interface.cursor_icon();
        window.set_cursor_icon(cursor_icon);
    }

    pub fn handle_window_event(&mut self, event: &WindowEvent, window: &Window) {
        self.user_interface.window_event(event, window);
        match event {
            WindowEvent::Resized(physical_size) => self.resize(*physical_size),
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => self.resize(**new_inner_size),
            WindowEvent::KeyboardInput { .. } => self.handle_keyboard_input(event),
            WindowEvent::MouseInput { state: ElementState::Released, .. } => self.scene.window_event(event),
            WindowEvent::MouseInput { .. } | WindowEvent::CursorMoved { .. } | WindowEvent::MouseWheel { .. }
            if !self.user_interface.capturing_mouse() => self.scene.window_event(event),
            _ => {}
        }
    }

    pub fn capture_prototype(&mut self, prototype: usize) {
        self.crucible.capture_prototype(prototype);
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
        self.user_interface.render(
            &self.graphics.device,
            &mut encoder,
            &view,
        );
        self.graphics.queue.submit(iter::once(encoder.finish()));
        output.present();
        self.user_interface.post_render();
        Ok(())
    }

    fn handle_keyboard_input(&mut self, event: &WindowEvent) {
        let WindowEvent::KeyboardInput {
            input: KeyboardInput {
                virtual_keycode: Some(keycode),
                state: ElementState::Pressed, ..
            }, ..
        } = event else {
            return;
        };
        self.user_interface.key_pressed(keycode);
    }

    pub(crate) fn create_brick(&mut self) {
        let Some(face_id) = self.scene.target_face_id() else {
            return;
        };
        let face_alias = match self.crucible.fabric().face(face_id).spin.opposite() {
            Spin::Left => FaceAlias::single("Left::Bot"),
            Spin::Right => FaceAlias::single("Right::Bot"),
        };
        self.user_interface.action(Action::AddBrick { face_alias, face_id });
    }
}

fn library_modified_timestamp() -> SystemTime {
    fs::metadata("./src/build/tenscript/library.scm")
        .unwrap()
        .modified()
        .unwrap()
}
