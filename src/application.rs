use std::{fs, iter};
use std::time::SystemTime;

use iced_wgpu::wgpu;
use winit::{
    event::*,
};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::build::tenscript::{FabricPlan, FaceAlias, Spin};
use crate::camera::Target::{FabricMidpoint, Hold, Origin, SelectedFace};
use crate::gui::strain_threshold::StrainThresholdMessage;
use crate::crucible::Crucible;
use crate::graphics::GraphicsWindow;
use crate::gui::control_state::{Action, ControlMessage, VisibleControl};
use crate::gui::GUI;
use crate::scene::Scene;

pub struct Application {
    graphics: GraphicsWindow,
    scene: Scene,
    gui: GUI,
    crucible: Crucible,
    library_modified: SystemTime,
    fabric_plan_name: Option<String>,
}

impl Application {
    pub fn new(graphics: GraphicsWindow, window: &Window) -> Application {
        let gui = GUI::new(&graphics, window);
        let scene = Scene::new(&graphics);
        Application {
            graphics,
            scene,
            gui,
            crucible: Crucible::default(),
            library_modified: library_modified_timestamp(),
            fabric_plan_name: None,
        }
    }

    pub fn update(&mut self, window: &Window) {
        self.gui.update();
        let mut actions = self.gui.controls().take_actions();
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
                    self.gui.queue_message(ControlMessage::Reset);
                    self.crucible.build_fabric(fabric_plan);
                }
                Action::GravityChanged(_gravity) => {
                    // TODO
                }
                Action::CalibrateStrain => {
                    let strain_limits = self.crucible.strain_limits();
                    self.gui.queue_message(ControlMessage::StrainThreshold(StrainThresholdMessage::SetStrainLimits(strain_limits)))
                }
                Action::SelectFace(face_id) => {
                    self.scene.select_face(Some(face_id));
                }
                Action::AddBrick { face_alias, face_id } => {
                    self.scene.select_face(None);
                    self.crucible.add_brick(face_alias, face_id)
                }
                Action::ShowSurface => {
                    self.scene.show_surface(true)
                }
            }
        }
        window.request_redraw();
    }

    pub fn redraw(&mut self, window: &Window) {
        self.crucible.iterate();
        if let Some(action) = self.crucible.action() {
            self.gui.queue_message(ControlMessage::Action(action))
        }
        self.scene.update(&self.graphics, self.gui.controls().variation(self.scene.target_face_id()), self.crucible.fabric());
        self.gui.update_viewport(&window);
        match self.render() {
            Ok(_) => {}
            Err(wgpu::SurfaceError::Lost) => self.resize(self.graphics.size),
            Err(wgpu::SurfaceError::OutOfMemory) => panic!("WGPU out of memory"),
            Err(e) => eprintln!("{e:?}"),
        }
        let cursor_icon = self.gui.cursor_icon();
        window.set_cursor_icon(cursor_icon);
    }

    pub fn handle_window_event(&mut self, event: &WindowEvent, window: &Window) {
        self.gui.window_event(event, &window);
        match event {
            WindowEvent::Resized(physical_size) => self.resize(*physical_size),
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => self.resize(**new_inner_size),
            WindowEvent::KeyboardInput { .. } => self.handle_keyboard_input(event),
            WindowEvent::MouseInput { state: ElementState::Released, .. } => self.scene.window_event(event),

            WindowEvent::MouseInput { .. } |
            WindowEvent::CursorMoved { .. } |
            WindowEvent::MouseWheel { .. }
            if !self.gui.capturing_mouse() => self.scene.window_event(event),
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
