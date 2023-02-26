use std::{fs, iter};
use std::collections::HashSet;
use std::time::SystemTime;

use iced_wgpu::wgpu;
use winit::{
    event::*,
};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::build::tenscript::FabricPlan;
use crate::build::tinkerer::BrickOnFace;
use crate::crucible::{Crucible, CrucibleAction, TinkererAction};
use crate::fabric::{Fabric, UniqueId};
use crate::graphics::GraphicsWindow;
use crate::scene::{Scene, SceneAction, SceneVariant};
use crate::scene::SceneVariant::TinkeringOnFaces;
use crate::user_interface::{Action, ControlMessage, MenuEnvironment, UserInterface};

pub struct Application {
    selected_faces: HashSet<UniqueId>,
    scene: Scene,
    user_interface: UserInterface,
    crucible: Crucible,
    graphics: GraphicsWindow,
    library_modified: SystemTime,
    fabric_plan_name: Vec<String>,
}

impl Application {
    pub fn new(graphics: GraphicsWindow, window: &Window) -> Application {
        let user_interface = UserInterface::new(&graphics, window);
        let scene = Scene::new(&graphics);
        Application {
            selected_faces: HashSet::new(),
            scene,
            user_interface,
            crucible: Crucible::default(),
            graphics,
            library_modified: library_modified_timestamp(),
            fabric_plan_name: Vec::new(),
        }
    }

    pub fn update(&mut self, window: &Window) {
        self.user_interface.update();
        let mut actions = self.user_interface.controls().take_actions();
        if library_modified_timestamp() > self.library_modified {
            let fabric_plan = FabricPlan::load_preset(self.fabric_plan_name.clone())
                .expect("unable to load fabric plan");
            actions.push(Action::Crucible(CrucibleAction::BuildFabric(fabric_plan)));
            self.library_modified = library_modified_timestamp();
        }
        for action in actions {
            match action {
                Action::Crucible(crucible_action) => {
                    if let CrucibleAction::BuildFabric(fabric_plan) = &crucible_action {
                        self.fabric_plan_name = fabric_plan.name.clone();
                        self.scene.action(SceneAction::Variant(SceneVariant::Suspended));
                        self.user_interface.message(ControlMessage::Reset);
                    }
                    self.crucible.action(crucible_action);
                }
                Action::Scene(scene_action) => {
                    self.scene.action(scene_action);
                }
                Action::Keyboard(menu_choice) => {
                    self.user_interface.menu_choice(menu_choice);
                }
                Action::ShowControl(visible_control) => {
                    self.user_interface.message(ControlMessage::ShowControl(visible_control));
                }
                Action::GravityChanged(_gravity) => {
                    unimplemented!();
                }
                Action::CalibrateStrain => {
                    let strain_limits = self.crucible.fabric().strain_limits(Fabric::BOW_TIE_MATERIAL_INDEX);
                    self.user_interface.set_strain_limits(strain_limits);
                }
                Action::SelectFace(face_id) => {
                    if let Some(face_id) = face_id {
                        if self.selected_faces.contains(&face_id) {
                            self.selected_faces.clear();
                        } else {
                            self.selected_faces.insert(face_id);
                        }
                    } else {
                        self.selected_faces.clear();
                    }
                    self.scene.action(SceneAction::Variant(TinkeringOnFaces(self.selected_faces.clone())));
                    self.user_interface.set_menu_environment(MenuEnvironment{
                        face_count: self.crucible.fabric().faces.len(),
                        selection_count: self.selected_faces.len(),
                        brick_proposed: self.crucible.is_brick_proposed(),
                        pretenst_complete: self.crucible.is_pretenst_complete(),
                    })
                }
                Action::StartTinkering => {
                    unimplemented!();
                }
                Action::ToggleDebug => {
                    self.user_interface.message(ControlMessage::ToggleDebugMode);
                }
                Action::ProposeBrick { alias, face_rotation } => {
                    if let Some(face_id) = self.selected_face() {
                        let spin = self.crucible.fabric().face(face_id).spin.opposite();
                        let alias = alias + &spin.into_alias();
                        let brick_on_face = BrickOnFace { face_id, alias, face_rotation };
                        self.crucible.action(CrucibleAction::Tinkerer(TinkererAction::Propose(brick_on_face)));
                    }
                }
                Action::InitiateJoinFaces => {
                    self.crucible.action(
                        CrucibleAction::Tinkerer(
                            TinkererAction::JoinIfPair(self.selected_faces.clone())));
                }
                Action::Connect => {
                    self.crucible.action(CrucibleAction::Tinkerer(TinkererAction::Commit));
                }
                Action::Revert => {
                    self.crucible.action(CrucibleAction::Tinkerer(TinkererAction::InitiateRevert));
                }
                Action::RevertToFrozen { fabric, brick_on_face } => {
                    self.selected_faces.clear();
                    self.crucible.action(CrucibleAction::RevertTo(fabric));
                    if let Some(brick_on_face) = brick_on_face {
                        let face_id = brick_on_face.face_id;
                        self.crucible.action(
                            CrucibleAction::Tinkerer(
                                TinkererAction::Propose(brick_on_face)));
                        self.user_interface.action(Action::SelectFace(Some(face_id)));
                    }
                }
            }
        }
        window.request_redraw();
    }

    pub fn redraw(&mut self, window: &Window) {
        for action in self.crucible.iterate(!self.selected_faces.is_empty()) {
            self.user_interface.action(action);
        }
        self.scene.update(&self.graphics, self.crucible.fabric());
        if let Some(picked) = self.scene.picked() {
            self.user_interface.action(Action::SelectFace(Some(picked)))
        }
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
            WindowEvent::KeyboardInput { .. } => self.handle_keyboard_input(event),
            WindowEvent::ModifiersChanged { .. } => self.scene.window_event(event, self.crucible.fabric()),
            WindowEvent::MouseInput { state: ElementState::Released, .. } => self.scene.window_event(event, self.crucible.fabric()),
            WindowEvent::MouseInput { .. } | WindowEvent::CursorMoved { .. } | WindowEvent::MouseWheel { .. }
            if !self.user_interface.capturing_mouse() => self.scene.window_event(event, self.crucible.fabric()),
            _ => {}
        }
    }

    pub fn capture_prototype(&mut self, brick_index: usize) {
        self.crucible.action(CrucibleAction::BakeBrick(brick_index));
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
        let mut encoder = self.graphics.create_command_encoder();
        self.scene.render(&mut encoder, &view);
        self.user_interface.render(&self.graphics.device, &mut encoder, &view);
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

    fn selected_face(&self) -> Option<UniqueId> {
        let Ok([&face_id]) = self.selected_faces.iter().next_chunk() else {
            return None;
        };
        Some(face_id)
    }
}

fn library_modified_timestamp() -> SystemTime {
    fs::metadata("./src/build/tenscript/library.scm")
        .unwrap()
        .modified()
        .unwrap()
}
