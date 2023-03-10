use std::{fs, iter};
use std::collections::HashSet;
use std::time::SystemTime;

use iced_wgpu::wgpu;
use winit::{
    event::*,
};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::build::tenscript::{FabricPlan, FaceAlias, TenscriptError};
use crate::build::tenscript::brick::Baked;
use crate::build::tenscript::brick_library::BrickLibrary;
use crate::build::tenscript::fabric_library::FabricLibrary;
use crate::build::tinkerer::{BrickOnFace, Frozen};
use crate::camera::Pick;
use crate::crucible::{Crucible, CrucibleAction, TinkererAction};
use crate::fabric::UniqueId;
use crate::graphics::GraphicsWindow;
use crate::scene::{Scene, SceneAction, SceneVariant};
use crate::user_interface::{Action, ControlMessage, MenuAction, MenuEnvironment, UserInterface};

pub struct Application {
    selected_faces: HashSet<UniqueId>,
    scene: Scene,
    user_interface: UserInterface,
    crucible: Crucible,
    graphics: GraphicsWindow,
    fabric_plan_name: Vec<String>,
    fabric_library: FabricLibrary,
    fabric_library_modified: SystemTime,
    brick_library: BrickLibrary,
}

impl Application {
    pub fn new(graphics: GraphicsWindow, window: &Window) -> Application {
        let brick_library = BrickLibrary::from_source().unwrap();
        let fabric_library = FabricLibrary::from_source().unwrap();
        let user_interface = UserInterface::new(&graphics, window, &fabric_library.fabric_plans);
        let scene = Scene::new(&graphics);
        Application {
            selected_faces: HashSet::new(),
            scene,
            user_interface,
            crucible: Crucible::default(),
            graphics,
            fabric_plan_name: Vec::new(),
            brick_library,
            fabric_library,
            fabric_library_modified: fabric_library_modified(),
        }
    }

    pub fn update(&mut self, window: &Window) {
        self.user_interface.update();
        let mut actions = self.user_interface.controls().take_actions();
        let time = fabric_library_modified();
        if time > self.fabric_library_modified {
            match self.refresh_library(time) {
                Ok(action) => {
                    actions.push(action);
                }
                Err(tenscript_error) => {
                    println!("Tenscript\n{tenscript_error}");
                    self.fabric_library_modified = time;
                }
            }
        }
        for action in actions {
            match action {
                Action::Crucible(crucible_action) => {
                    match &crucible_action {
                        CrucibleAction::BuildFabric(fabric_plan) => {
                            self.fabric_plan_name = fabric_plan.name.clone();
                            self.scene.action(SceneAction::Variant(SceneVariant::Suspended));
                            self.user_interface.message(ControlMessage::Reset);
                            self.update_menu_environment()
                        }
                        CrucibleAction::StartPretensing(_) => {
                            self.user_interface.action(Action::Keyboard(MenuAction::ReturnToRoot))
                        }
                        _ => {}
                    }
                    self.crucible.action(crucible_action);
                }
                Action::UpdateMenu => {
                    self.update_menu_environment();
                }
                Action::UpdatedLibrary(time) => {
                    let fabric_library = self.fabric_library.clone();
                    self.fabric_library_modified = time;
                    if !self.fabric_plan_name.is_empty() {
                        let fabric_plan = self.load_preset(self.fabric_plan_name.clone())
                            .expect("unable to load fabric plan");
                        self.crucible.action(CrucibleAction::BuildFabric(fabric_plan));
                    }
                    self.user_interface.message(ControlMessage::FreshLibrary(fabric_library));
                }
                Action::Scene(scene_action) => {
                    self.scene.action(scene_action);
                }
                Action::Keyboard(menu_choice) => {
                    match menu_choice {
                        MenuAction::ReturnToRoot => {
                            self.selected_faces.clear();
                            self.user_interface.action(
                                Action::Scene(SceneAction::Variant(SceneVariant::Suspended)))
                        }
                        MenuAction::TinkerMenu => {
                            self.selected_faces.clear();
                            self.user_interface.action(
                                Action::Scene(SceneAction::Variant(SceneVariant::TinkeringOnFaces(HashSet::new()))))
                        }
                        _ => {}
                    }
                    self.user_interface.menu_choice(menu_choice);
                }
                Action::ShowControl(visible_control) => {
                    self.user_interface.message(ControlMessage::ShowControl(visible_control));
                }
                Action::CalibrateStrain => {
                    let strain_limits = self.crucible.fabric().strain_limits(":bow-tie".to_string());
                    self.user_interface.set_strain_limits(strain_limits);
                }
                Action::SelectFace(face_id) => {
                    if let Some(Pick { face_id, multiple }) = face_id {
                        if !multiple {
                            self.selected_faces.clear();
                        }
                        self.selected_faces.insert(face_id);
                    } else {
                        self.selected_faces.clear();
                    }
                    self.selected_faces.retain(|id| self.crucible.fabric().faces.contains_key(id));
                    self.scene.action(SceneAction::Variant(SceneVariant::TinkeringOnFaces(self.selected_faces.clone())));
                    self.update_menu_environment();
                }
                Action::SelectAFace => {
                    if let Some(&selected) = self.selected_faces.iter().next() {
                        self.user_interface.action(Action::SelectFace(Some(Pick::just(selected))))
                    } else {
                        let pick_one = self.crucible.fabric().faces
                            .keys()
                            .next()
                            .copied()
                            .map(Pick::just);
                        self.user_interface.action(Action::SelectFace(pick_one))
                    }
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
                        self.update_menu_environment()
                    }
                }
                Action::RemoveProposedBrick => {
                    self.crucible.action(CrucibleAction::Tinkerer(TinkererAction::Clear));
                    self.update_menu_environment();
                }
                Action::InitiateJoinFaces => {
                    self.crucible.action(
                        CrucibleAction::Tinkerer(
                            TinkererAction::JoinIfPair(self.selected_faces.clone())));
                }
                Action::Connect => {
                    self.crucible.action(CrucibleAction::Tinkerer(TinkererAction::Commit));
                    self.update_menu_environment();
                }
                Action::Revert => {
                    self.crucible.action(CrucibleAction::Tinkerer(TinkererAction::InitiateRevert));
                    self.update_menu_environment();
                }
                Action::RevertToFrozen { frozen: Frozen { fabric, face_id }, brick_on_face } => {
                    self.selected_faces.clear();
                    self.crucible.action(CrucibleAction::RevertTo(fabric));
                    if let Some(brick_on_face) = brick_on_face {
                        let face_id = brick_on_face.face_id;
                        self.crucible.action(
                            CrucibleAction::Tinkerer(TinkererAction::Propose(brick_on_face)));
                        self.user_interface.action(Action::SelectFace(Some(Pick::just(face_id))));
                    } else {
                        face_id.map(|face_id| self.selected_faces.insert(face_id));
                    }
                    self.update_menu_environment();
                }
            }
        }
        window.request_redraw();
    }

    fn update_menu_environment(&mut self) {
        self.user_interface.set_menu_environment(MenuEnvironment {
            face_count: self.crucible.fabric().faces.len(),
            selection_count: self.selected_faces.len(),
            tinkering: self.crucible.is_tinkering(),
            brick_proposed: self.crucible.is_brick_proposed(),
            experimenting: self.crucible.is_experimenting(),
            history_available: self.crucible.is_history_available(),
            visible_control: self.user_interface.controls().show_controls(),
            fabric_menu: self.user_interface.create_fabric_menu(&self.fabric_library.fabric_plans),
        })
    }

    pub fn redraw(&mut self, window: &Window) {
        for action in self.crucible.iterate(!self.selected_faces.is_empty(), &self.brick_library) {
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

    pub fn run_fabric(&mut self, fabric_name: &String) {
        let fabric_plan = self.fabric_library.fabric_plans
            .iter()
            .find(|FabricPlan{name,..}| name.contains(fabric_name))
            .expect(fabric_name);
        self.user_interface.action(Action::Crucible(CrucibleAction::BuildFabric(fabric_plan.clone())))
    }

    pub fn capture_prototype(&mut self, brick_index: usize) {
        let prototype = self.brick_library.brick_definitions
            .get(brick_index).expect("no such brick")
            .proto.clone();
        self.crucible.action(CrucibleAction::BakeBrick(prototype));
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

    pub fn refresh_library(&mut self, time: SystemTime) -> Result<Action, TenscriptError> {
        self.fabric_library = FabricLibrary::from_source()?;
        Ok(Action::UpdatedLibrary(time))
    }

    pub fn load_preset(&self, plan_name: Vec<String>) -> Result<FabricPlan, TenscriptError> {
        let plan = self.fabric_library.fabric_plans
            .iter()
            .find(|plan| plan.name == plan_name);
        match plan {
            None => Err(TenscriptError::Invalid(plan_name.join(","))),
            Some(plan) => Ok(plan.clone())
        }
    }

    pub fn new_brick(&self, search_alias: &FaceAlias) -> Baked {
        self.brick_library.new_brick(search_alias)
    }
}

fn fabric_library_modified() -> SystemTime {
    fs::metadata("fabric_library.scm")
        .unwrap()
        .modified()
        .unwrap()
}
