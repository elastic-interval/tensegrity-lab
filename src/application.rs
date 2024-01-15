use std::{fs, iter};
use std::collections::HashSet;
use std::time::SystemTime;

use winit_input_helper::WinitInputHelper;

use crate::build::tenscript::{FabricPlan, FaceAlias, TenscriptError};
use crate::build::tenscript::brick::Baked;
use crate::build::tenscript::brick_library::BrickLibrary;
use crate::build::tenscript::fabric_library::FabricLibrary;
use crate::build::tinkerer::{BrickOnFace, Frozen};
use crate::camera::Pick;
use crate::crucible::{Crucible, CrucibleAction, TinkererAction};
use crate::fabric::UniqueId;
use crate::graphics::Graphics;
use crate::scene::{Scene, SceneAction, SceneVariant};
use crate::user_interface::{Action, ControlMessage, MenuAction, MenuContext, UserInterface};

pub struct Application {
    selected_faces: HashSet<UniqueId>,
    scene: Scene,
    user_interface: UserInterface,
    crucible: Crucible,
    graphics: Graphics,
    fabric_plan_name: Vec<String>,
    fabric_library: FabricLibrary,
    fabric_library_modified: SystemTime,
    brick_library: BrickLibrary,
}

impl Application {
    pub fn new(graphics: Graphics) -> Application {
        let brick_library = BrickLibrary::from_source().unwrap();
        let fabric_library = FabricLibrary::from_source().unwrap();
        let user_interface = UserInterface::new();
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

    pub fn update(&mut self) {
        let mut actions = self.user_interface.take_actions();
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
                            self.update_menu_context()
                        }
                        CrucibleAction::StartPretensing(_) => {
                            self.user_interface.action(Action::Keyboard(MenuAction::ReturnToRoot))
                        }
                        _ => {}
                    }
                    self.crucible.action(crucible_action);
                }
                Action::UpdateMenu => {
                    self.update_menu_context();
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
                    self.update_menu_context();
                    println!("Select face {:?}", face_id);
                }
                Action::ProposeBrick { alias, face_rotation } => {
                    if let Some(face_id) = self.selected_face() {
                        let spin = self.crucible.fabric().face(face_id).spin.opposite();
                        let alias = alias + &spin.into_alias();
                        let brick_on_face = BrickOnFace { face_id, alias, face_rotation };
                        self.crucible.action(CrucibleAction::Tinkerer(TinkererAction::Propose(brick_on_face)));
                        self.update_menu_context()
                    }
                }
                Action::RemoveProposedBrick => {
                    self.crucible.action(CrucibleAction::Tinkerer(TinkererAction::Clear));
                    self.update_menu_context();
                }
                Action::InitiateJoinFaces => {
                    self.crucible.action(
                        CrucibleAction::Tinkerer(
                            TinkererAction::JoinIfPair(self.selected_faces.clone())));
                }
                Action::Connect => {
                    self.crucible.action(CrucibleAction::Tinkerer(TinkererAction::Commit));
                    self.update_menu_context();
                }
                Action::Revert => {
                    self.crucible.action(CrucibleAction::Tinkerer(TinkererAction::InitiateRevert));
                    self.update_menu_context();
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
                    self.update_menu_context();
                }
            }
        }
    }

    fn update_menu_context(&mut self) {
        self.user_interface.set_menu_context(MenuContext {
            selection_count: self.selected_faces.len(),
            crucible_state: self.crucible.state(),
            fabric_menu: self.user_interface.create_fabric_menu(&self.fabric_library.fabric_plans),
        })
    }

    pub fn redraw(&mut self) {
        for action in self.crucible.iterate(!self.selected_faces.is_empty(), &self.brick_library) {
            self.user_interface.action(action);
        }
        self.scene.update(&self.graphics, self.crucible.fabric());
        if let Some(picked) = self.scene.picked() {
            self.user_interface.action(Action::SelectFace(Some(picked)))
        }
        match self.render() {
            Ok(_) => {}
            Err(wgpu::SurfaceError::Lost) => self.resize(self.graphics.config.width, self.graphics.config.height),
            Err(wgpu::SurfaceError::OutOfMemory) => panic!("Out of memory"),
            Err(e) => eprintln!("{e:?}"),
        }
        // let cursor_icon = self.user_interface.cursor_icon();
        // window.set_cursor_icon(cursor_icon);
    }

    pub fn handle_input(&mut self, input: &WinitInputHelper) {
        if let Some(size) = input.window_resized() {
            self.resize(size.width, size.height);
        }
        self.scene.handle_input(input, self.crucible.fabric());
        self.user_interface.handle_input(input);
    }

    pub fn run_fabric(&mut self, fabric_name: &String) {
        let fabric_plan = self.fabric_library.fabric_plans
            .iter()
            .find(|FabricPlan { name, .. }| name.contains(fabric_name))
            .expect(fabric_name);
        self.user_interface.queue_action(Action::Crucible(CrucibleAction::BuildFabric(fabric_plan.clone())))
    }

    pub fn capture_prototype(&mut self, brick_index: usize) {
        let prototype = self.brick_library.brick_definitions
            .get(brick_index).expect("no such brick")
            .proto.clone();
        self.crucible.action(CrucibleAction::BakeBrick(prototype));
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.graphics.config.width = width;
            self.graphics.config.height = height;
            self.graphics.surface.configure(&self.graphics.device, &self.graphics.config);
            self.scene.resize(&self.graphics);
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.graphics.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.graphics.create_command_encoder();
        self.scene.render(&mut encoder, &view);
        self.graphics.queue.submit(iter::once(encoder.finish()));
        output.present();
        Ok(())
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

// /// We derive Deserialize/Serialize so we can persist app state on shutdown.
// #[derive(serde::Deserialize, serde::Serialize)]
// #[serde(default)] // if we add new fields, give them default values when deserializing old state
// pub struct TemplateApp {
//     // Example stuff:
//     label: String,
//
//     #[serde(skip)] // This how you opt-out of serialization of a field
//     value: f32,
// }
//
// impl Default for TemplateApp {
//     fn default() -> Self {
//         Self {
//             // Example stuff:
//             label: "Hello World!".to_owned(),
//             value: 2.7,
//         }
//     }
// }
//
// impl TemplateApp {
//     /// Called once before the first frame.
//     pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
//         // This is also where you can customize the look and feel of egui using
//         // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.
//
//         // Load previous app state (if any).
//         // Note that you must enable the `persistence` feature for this to work.
//         if let Some(storage) = cc.storage {
//             return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
//         }
//
//         Default::default()
//     }
// }
//
// impl eframe::App for TemplateApp {
//     /// Called each time the UI needs repainting, which may be many times per second.
//     fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
//         // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
//         // For inspiration and more examples, go to https://emilk.github.io/egui
//
//         egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
//             // The top panel is often a good place for a menu bar:
//
//             egui::menu::bar(ui, |ui| {
//                 // NOTE: no File->Quit on web pages!
//                 let is_web = cfg!(target_arch = "wasm32");
//                 if !is_web {
//                     ui.menu_button("File", |ui| {
//                         if ui.button("Quit").clicked() {
//                             ctx.send_viewport_cmd(egui::ViewportCommand::Close);
//                         }
//                     });
//                     ui.add_space(16.0);
//                 }
//
//                 egui::widgets::global_dark_light_mode_buttons(ui);
//             });
//         });
//
//         egui::CentralPanel::default().show(ctx, |ui| {
//             // The central panel the region left after adding TopPanel's and SidePanel's
//             ui.heading("eframe template");
//
//             ui.horizontal(|ui| {
//                 ui.label("Write something: ");
//                 ui.text_edit_singleline(&mut self.label);
//             });
//
//             ui.add(egui::Slider::new(&mut self.value, 0.0..=10.0).text("value"));
//             if ui.button("Increment").clicked() {
//                 self.value += 1.0;
//             }
//
//             ui.separator();
//
//             ui.add(egui::github_link_file!(
//                 "https://github.com/emilk/eframe_template/blob/master/",
//                 "Source code."
//             ));
//
//             ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
//                 powered_by_egui_and_eframe(ui);
//                 egui::warn_if_debug_build(ui);
//             });
//         });
//     }
//
//     /// Called by the frame work to save state before shutdown.
//     fn save(&mut self, storage: &mut dyn eframe::Storage) {
//         eframe::set_value(storage, eframe::APP_KEY, self);
//     }
// }
//
// fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
//     ui.horizontal(|ui| {
//         ui.spacing_mut().item_spacing.x = 0.0;
//         ui.label("Powered by ");
//         ui.hyperlink_to("egui", "https://github.com/emilk/egui");
//         ui.label(" and ");
//         ui.hyperlink_to(
//             "eframe",
//             "https://github.com/emilk/egui/tree/master/crates/eframe",
//         );
//         ui.label(".");
//     });
// }
