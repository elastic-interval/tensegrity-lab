use std::iter;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use std::time::SystemTime;

use leptos::{ReadSignal, WriteSignal};
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::Key;
use winit::window::{WindowAttributes, WindowId};

use control_overlay::ControlState;

use crate::build::tenscript::{FabricPlan, TenscriptError};
use crate::build::tenscript::brick_library::BrickLibrary;
use crate::build::tenscript::fabric_library::FabricLibrary;
use crate::camera::Pick;
use crate::control_overlay;
use crate::control_overlay::action::Action;
use crate::crucible::{Crucible, CrucibleAction};
use crate::scene::Scene;
use crate::wgpu_context::WgpuContext;

pub struct Application<'a> {
    window_attributes: WindowAttributes,
    scene: Option<Scene<'a>>,
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

impl<'a> ApplicationHandler for Application<'a> {
    
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(event_loop.create_window(self.window_attributes.clone()).unwrap());
        let wgpu_context = WgpuContext::new(window);
        let scene = Scene::new(wgpu_context, (self.control_state, self.set_control_state));
        self.scene = Some(scene)
    }

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::ActivationTokenDone { .. } => {}
            WindowEvent::Resized(size) => {
                println!("Resized {:?}", size);
            }
            WindowEvent::CloseRequested => {}
            WindowEvent::KeyboardInput { event: KeyEvent { physical_key, logical_key, text, .. }, .. } => {
                println!("KeyEvent phy={:?} log={:?}, text={:?}", physical_key, logical_key, text)
            }
            WindowEvent::RedrawRequested => {
                panic!("------Redraw!!!");
            }
            _ => {
                println!("Event {:?}", event);
                // if let Some(scene) = &mut self.scene {
                //     scene.graphics.window.request_redraw();
                // }
            }
        }
        // self.handle_actions();
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _device_id: DeviceId, _event: DeviceEvent) {
        // println!("device {:?}", event);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.redraw();
    }
}

impl<'a> Application<'a> {
    pub fn new(
        window_attributes: WindowAttributes,
        (control_state, set_control_state): (ReadSignal<ControlState>, WriteSignal<ControlState>),
        (actions_tx, actions_rx): (Sender<Action>, Receiver<Action>),
    ) -> Result<Application<'a>, TenscriptError> {
        let brick_library = BrickLibrary::from_source()?;
        let fabric_library = FabricLibrary::from_source()?;
        Ok(Application {
            window_attributes,
            scene: None,
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
        })
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
                            if let Some(scene) = &mut self.scene {
                                scene.do_pick(Pick::Nothing);
                            }
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
                Action::Scene(_pick) => {
                    // if let Some(mut scene) = &self.scene {
                    //     scene.do_pick(pick);
                    // }
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

        if let Some(scene) = &mut self.scene {
            if !scene.selection_active() {
                self.crucible.iterate(&self.brick_library);
            }
            scene.update(self.crucible.fabric());
            let surface_texture = scene.wgpu_context.surface_texture().expect("surface texture");
            let texture_view = surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = scene.wgpu_context.create_encoder();
            scene.render(&mut encoder, &texture_view);
            scene.wgpu_context.queue.submit(iter::once(encoder.finish()));
            surface_texture.present();
        }
        // let cursor_icon = self.user_interface.cursor_icon();
        // window.set_cursor_icon(cursor_icon);
    }

    pub fn handle_input(&mut self, _input: &Key) {
        // if let Some(size) = input.window_resized() {
        //     self.scene.resize(size.width, size.height);
        // }
        // self.scene.handle_input(input, self.crucible.fabric());
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
            None => Err(TenscriptError::InvalidError(plan_name)),
            Some(plan) => Ok(plan.clone()),
        }
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
