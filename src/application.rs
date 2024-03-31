use std::iter;
use std::sync::mpsc::{Receiver, Sender};
use std::time::SystemTime;

use leptos::{ReadSignal, WriteSignal};
use winit_input_helper::WinitInputHelper;

use control_overlay::ControlState;

use crate::build::tenscript::brick::Baked;
use crate::build::tenscript::brick_library::BrickLibrary;
use crate::build::tenscript::fabric_library::FabricLibrary;
use crate::build::tenscript::{FabricPlan, FaceAlias, TenscriptError};
use crate::camera::Pick;
use crate::control_overlay;
use crate::control_overlay::action::Action;
use crate::crucible::{Crucible, CrucibleAction};
use crate::graphics::Graphics;
use crate::scene::{Scene, SceneAction};

pub struct Application {
    scene: Scene,
    crucible: Crucible,
    fabric_plan_name: String,
    fabric_library: FabricLibrary,
    #[cfg(not(target_arch = "wasm32"))]
    fabric_library_modified: SystemTime,
    pub brick_library: BrickLibrary,
    pub control_state: ReadSignal<ControlState>,
    pub set_control_state: WriteSignal<ControlState>,
    pub actions_rx: Receiver<Action>,
    pub actions_tx: Sender<Action>,
}

impl Application {
    pub fn new(
        graphics: Graphics,
        (control_state, set_control_state): (ReadSignal<ControlState>, WriteSignal<ControlState>),
        (actions_tx, actions_rx): (Sender<Action>, Receiver<Action>),
    ) -> Application {
        let brick_library = BrickLibrary::from_source().unwrap();
        let fabric_library = FabricLibrary::from_source().unwrap();
        let scene = Scene::new(graphics, (control_state, set_control_state));
        Application {
            scene,
            crucible: Crucible::default(),
            fabric_plan_name: "Halo by Crane".into(),
            brick_library,
            fabric_library,
            control_state,
            set_control_state,
            actions_tx,
            actions_rx,
            #[cfg(not(target_arch = "wasm32"))]
            fabric_library_modified: fabric_library_modified(),
        }
    }

    pub fn handle_actions(&mut self) {
        while let Ok(action) = self.actions_rx.try_recv() {
            match action {
                Action::LoadFabric(fabric_plan_name) => {
                    self.fabric_plan_name = fabric_plan_name;
                    self.reload_fabric();
                }
                Action::Crucible(crucible_action) => {
                    match &crucible_action {
                        CrucibleAction::BuildFabric(fabric_plan) => {
                            self.fabric_plan_name = fabric_plan.name.clone();
                            self.scene.action(SceneAction::Selected(Pick::Nothing));
                            // self.user_interface.message(ControlMessage::Reset);
                        }
                        CrucibleAction::StartPretensing(_) => {
                            // menu: ReturnToRoot
                        }
                        _ => {}
                    }
                    self.crucible.action(crucible_action);
                }
                #[cfg(target_arch = "wasm32")]
                Action::UpdatedLibrary(_) => unreachable!(),
                #[cfg(not(target_arch = "wasm32"))]
                Action::UpdatedLibrary(time) => {
                    let _fabric_library = self.fabric_library.clone();
                    self.fabric_library_modified = time;
                    if !self.fabric_plan_name.is_empty() {
                        self.reload_fabric();
                    }
                }
                Action::Scene(scene_action) => {
                    self.scene.action(scene_action);
                }
                Action::CalibrateStrain => {
                    // let strain_limits =
                    //     self.crucible.fabric().strain_limits(":bow-tie".to_string());
                    // self.user_interface.set_strain_limits(strain_limits);
                }
            }
        }
    }

    fn reload_fabric(&mut self) {
        let fabric_plan = self
            .load_preset(self.fabric_plan_name.clone())
            .expect("unable to load fabric plan");
        self.crucible
            .action(CrucibleAction::BuildFabric(fabric_plan));
    }

    pub fn redraw(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let time = fabric_library_modified();
            if time > self.fabric_library_modified {
                match self.refresh_library(time) {
                    Ok(action) => {
                        self.actions_tx
                            .send(action)
                            .unwrap();
                    }
                    Err(tenscript_error) => {
                        println!("Tenscript\n{tenscript_error}");
                        self.fabric_library_modified = time;
                    }
                }
            }
        }
        if !self.scene.selection_active() {
            self.crucible.iterate(&self.brick_library);
        }
        self.scene.update(self.crucible.fabric());
        let surface_texture = self.scene.surface_texture().expect("surface texture");
        self.render(&surface_texture);
        surface_texture.present();
        // let cursor_icon = self.user_interface.cursor_icon();
        // window.set_cursor_icon(cursor_icon);
    }

    pub fn handle_input(&mut self, input: &WinitInputHelper) {
        if let Some(size) = input.window_resized() {
            self.scene.resize(size.width, size.height);
        }
        self.scene.handle_input(input, self.crucible.fabric());
    }

    pub fn run_fabric(&mut self, fabric_name: &String) {
        let fabric_plan = self
            .fabric_library
            .fabric_plans
            .iter()
            .find(|FabricPlan { name, .. }| name == fabric_name)
            .expect(fabric_name);
        self.actions_tx
            .send(Action::Crucible(CrucibleAction::BuildFabric(
                fabric_plan.clone(),
            )))
            .unwrap();
    }

    pub fn capture_prototype(&mut self, brick_index: usize) {
        let prototype = self
            .brick_library
            .brick_definitions
            .get(brick_index)
            .expect("no such brick")
            .proto
            .clone();
        self.crucible.action(CrucibleAction::BakeBrick(prototype));
    }

    fn render(&mut self, surface_texture: &wgpu::SurfaceTexture) {
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.scene.create_encoder();
        self.scene.render(&mut encoder, &texture_view);
        self.scene.queue().submit(iter::once(encoder.finish()));
    }

    pub fn refresh_library(&mut self, time: SystemTime) -> Result<Action, TenscriptError> {
        self.fabric_library = FabricLibrary::from_source()?;
        Ok(Action::UpdatedLibrary(time))
    }

    pub fn load_preset(&self, plan_name: String) -> Result<FabricPlan, TenscriptError> {
        let plan = self
            .fabric_library
            .fabric_plans
            .iter()
            .find(|plan| plan.name == plan_name);
        match plan {
            None => Err(TenscriptError::Invalid(plan_name)),
            Some(plan) => Ok(plan.clone()),
        }
    }

    pub fn new_brick(&self, search_alias: &FaceAlias) -> Baked {
        self.brick_library.new_brick(search_alias)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn fabric_library_modified() -> SystemTime {
    use std::fs;
    fs::metadata("fabric_library.scm")
        .unwrap()
        .modified()
        .unwrap()
}
