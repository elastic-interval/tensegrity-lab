use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use std::time::SystemTime;

use winit::application::ApplicationHandler;
use winit::event::{KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopClosed, EventLoopProxy};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{WindowAttributes, WindowId};

use crate::build::tenscript::{FabricPlan, TenscriptError};
use crate::build::tenscript::brick_library::BrickLibrary;
use crate::build::tenscript::fabric_library::FabricLibrary;
use crate::control_overlay::menu::{EventMap, MenuContent, MenuItem};
use crate::crucible::{Crucible, CrucibleAction};
use crate::messages::{LabEvent, SceneAction};
use crate::scene::Scene;
use crate::wgpu::Wgpu;

pub struct Application {
    window_attributes: WindowAttributes,
    scene: Option<Scene>,
    crucible: Crucible,
    fabric_plan_name: String,
    fabric_library: FabricLibrary,
    #[cfg(not(target_arch = "wasm32"))]
    fabric_library_modified: SystemTime,
    brick_library: BrickLibrary,
    event_map: EventMap,
    menu_tx: Sender<MenuItem>,
    menu_rx: Receiver<MenuItem>,
    event_loop_proxy: Arc<EventLoopProxy<LabEvent>>,
}

impl ApplicationHandler<LabEvent> for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(event_loop.create_window(self.window_attributes.clone()).unwrap());
        let event_loop_proxy = self.event_loop_proxy.clone();
        Wgpu::create_and_send(window, event_loop_proxy);
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: LabEvent) {
        match event {
            LabEvent::ControlState(control_state) => {
                self.event_loop_proxy.send_event(LabEvent::ControlState(control_state)).unwrap()
            },
            LabEvent::MenuChoice(menu_item) => {
                match menu_item.content {
                    MenuContent::Event(lab_event_key) => {
                        let event = self.event_map.get(&lab_event_key).unwrap();
                        self.event_loop_proxy.send_event(event.clone()).unwrap()
                    }
                    MenuContent::Submenu(_) => {}
                }
            }
            LabEvent::ContextCreated(wgpu) => {
                self.scene = Some(Scene::new(wgpu, self.event_loop_proxy.clone()))
            }
            LabEvent::LoadFabric(fabric_plan_name) => {
                self.fabric_plan_name = fabric_plan_name;
                self.reload_fabric();
            }
            LabEvent::Crucible(crucible_action) => {
                if let CrucibleAction::BuildFabric(fabric_plan) = &crucible_action {
                    self.fabric_plan_name = fabric_plan.name.clone();
                    if let Some(scene) = &mut self.scene {
                        scene.reset();
                    }
                }
                self.crucible.action(crucible_action);
            }
            #[cfg(target_arch = "wasm32")]
            LabEvent::UpdatedLibrary(_) => unreachable!(),
            #[cfg(not(target_arch = "wasm32"))]
            LabEvent::UpdatedLibrary(time) => {
                let _fabric_library = self.fabric_library.clone();
                self.fabric_library_modified = time;
                if !self.fabric_plan_name.is_empty() {
                    self.reload_fabric();
                }
            }
            LabEvent::Scene(scene_action) => {
                if let Some(scene) = &mut self.scene {
                    match scene_action {
                        SceneAction::ForcePick(pick) => scene.do_pick(pick),
                        SceneAction::EscapeHappens => scene.escape_happens(),
                    }
                }
            }
            LabEvent::CalibrateStrain => {
                // let strain_limits =
                //     self.crucible.fabric().strain_limits(":bow-tie".to_string());
                // self.user_interface.set_strain_limits(strain_limits);
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        if let Some(scene) = &mut self.scene {
            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::KeyboardInput { event: key_event, .. } => self.handle_key_event(key_event),
                WindowEvent::CursorMoved { position, .. } => scene.camera().cursor_moved(position),
                WindowEvent::MouseInput { state, button, .. } => {
                    scene.camera().mouse_input(state, button, self.crucible.fabric());
                }
                WindowEvent::MouseWheel { delta, .. } => scene.camera().mouse_wheel(delta),
                WindowEvent::TouchpadPressure { .. } => {}
                _ => println!("Unhandled Event {:?}", event),
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Ok(item) = self.menu_rx.try_recv() {
            self.event_loop_proxy.send_event(LabEvent::MenuChoice(item)).unwrap();
        }
        self.redraw();
    }
}

impl Application {
    pub fn new(
        window_attributes: WindowAttributes,
        event_loop_proxy: Arc<EventLoopProxy<LabEvent>>,
        (menu_tx, menu_rx): (Sender<MenuItem>, Receiver<MenuItem>),
        event_map: EventMap,
    ) -> Result<Application, TenscriptError> {
        let brick_library = BrickLibrary::from_source()?;
        let fabric_library = FabricLibrary::from_source()?;
        Ok(Application {
            window_attributes,
            scene: None,
            crucible: Crucible::default(),
            fabric_plan_name: "Halo by Crane".into(),
            brick_library,
            fabric_library,
            menu_tx,
            menu_rx,
            event_loop_proxy,
            #[cfg(not(target_arch = "wasm32"))]
            fabric_library_modified: fabric_library_modified(),
            event_map,
        })
    }

    fn handle_key_event(&self, key_event: KeyEvent) {
        if !key_event.state.is_pressed() {
            return;
        }
        if let KeyEvent { physical_key: PhysicalKey::Code(code), .. } = key_event {
            if code == KeyCode::Escape {
                self.event_loop_proxy.send_event(LabEvent::Scene(SceneAction::EscapeHappens)).unwrap();
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
                        self.event_loop_proxy
                            .send_event(action)
                            .unwrap_or_else(|_| panic!("unable to send"));
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
            scene.redraw(self.crucible.fabric());
        }
    }

    pub fn build_fabric(&mut self, fabric_name: &String) -> Result<(), EventLoopClosed<LabEvent>> {
        let fabric_plan = self
            .fabric_library
            .fabric_plans
            .iter()
            .find(|FabricPlan { name, .. }| name == fabric_name)
            .expect(fabric_name);
        self.event_loop_proxy
            .send_event(LabEvent::Crucible(CrucibleAction::BuildFabric(fabric_plan.clone())))
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

    pub fn refresh_library(&mut self, time: SystemTime) -> Result<LabEvent, TenscriptError> {
        self.fabric_library = FabricLibrary::from_source()?;
        Ok(LabEvent::UpdatedLibrary(time))
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
