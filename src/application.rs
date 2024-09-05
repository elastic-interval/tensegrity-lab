use std::sync::Arc;
use std::time::{Duration, SystemTime};

use leptos::WriteSignal;
use winit::application::ApplicationHandler;
use winit::event::{KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use winit::window::{WindowAttributes, WindowId};

use crate::build::tenscript::{FabricPlan, TenscriptError};
use crate::build::tenscript::brick_library::BrickLibrary;
use crate::build::tenscript::fabric_library::FabricLibrary;
use crate::camera::Pick;
use crate::control_overlay::menu::{EventMap, MenuContent};
use crate::crucible::{Crucible, CrucibleAction};
use crate::messages::{ControlState, LabEvent};
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
    set_control_state: WriteSignal<ControlState>,
    event_loop_proxy: Arc<EventLoopProxy<LabEvent>>,
    fabric_alive: bool,
}

impl Application {
    pub fn new(
        window_attributes: WindowAttributes,
        set_control_state: WriteSignal<ControlState>,
        event_loop_proxy: Arc<EventLoopProxy<LabEvent>>,
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
            set_control_state,
            event_loop_proxy,
            #[cfg(not(target_arch = "wasm32"))]
            fabric_library_modified: fabric_library_modified(),
            event_map,
            fabric_alive: true,
        })
    }

    fn handle_key_event(&self, key_event: KeyEvent) {
        if !key_event.state.is_pressed() {
            return;
        }
        // if let KeyEvent { physical_key: PhysicalKey::Code(code), .. } = key_event {
        //     if code == KeyCode::Escape {
        //     }
        // }
    }

    fn build_current_fabric(&mut self) {
        let fabric_plan = self
            .get_fabric_plan(self.fabric_plan_name.clone())
            .expect("unable to load fabric plan");
        self.crucible
            .action(CrucibleAction::BuildFabric(fabric_plan));
    }

    fn redraw(&mut self) {
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
            scene.redraw(self.crucible.fabric());
        }
    }

    pub fn refresh_library(&mut self, time: SystemTime) -> Result<LabEvent, TenscriptError> {
        self.fabric_library = FabricLibrary::from_source()?;
        Ok(LabEvent::UpdatedLibrary(time))
    }

    pub fn get_fabric_plan(&self, plan_name: String) -> Result<FabricPlan, TenscriptError> {
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

impl ApplicationHandler<LabEvent> for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(event_loop.create_window(self.window_attributes.clone()).unwrap());
        let event_loop_proxy = self.event_loop_proxy.clone();
        Wgpu::create_and_send(window, event_loop_proxy);
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: LabEvent) {
        match event {
            LabEvent::ContextCreated(wgpu) => {
                self.scene = Some(Scene::new(wgpu, self.set_control_state));
            }
            LabEvent::SendMenuEvent(menu_item) => {
                if let MenuContent::Event(lab_event_key) = menu_item.content {
                    let event = self.event_map.get(&lab_event_key).unwrap();
                    self.event_loop_proxy.send_event(event.clone()).unwrap()
                }
            }
            LabEvent::LoadFabric(fabric_plan_name) => {
                self.fabric_plan_name = fabric_plan_name.clone();
                self.build_current_fabric();
                self.fabric_alive = true;
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
                    self.build_current_fabric();
                }
            }
            LabEvent::CalibrateStrain => {
                // let strain_limits =
                //     self.crucible.fabric().strain_limits(":bow-tie".to_string());
                // self.user_interface.set_strain_limits(strain_limits);
            }
            LabEvent::CapturePrototype(brick_index) => {
                self.fabric_alive = true;
                let prototype = self
                    .brick_library
                    .brick_definitions
                    .get(brick_index)
                    .expect("no such brick")
                    .proto
                    .clone();
                self.crucible.action(CrucibleAction::BakeBrick(prototype));
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
                    if let Some(scene) = &mut self.scene {
                        scene.mouse_input(state, button, self.crucible.fabric());
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => scene.camera().mouse_wheel(delta),
                WindowEvent::TouchpadPressure { .. } => {}
                WindowEvent::CursorEntered { .. } => {
                    self.fabric_alive = true;
                }
                WindowEvent::CursorLeft { .. } => {
                    self.fabric_alive = false;
                }
                _ => println!("Unhandled Event {:?}", event),
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(scene) = &mut self.scene {
            let approaching = scene.camera().target_approach(self.crucible.fabric());
            let pick_active = !matches!(scene.camera().current_pick(), Pick::Nothing);
            let iterating = self.fabric_alive && !pick_active;
            if iterating {
                self.crucible.iterate(&self.brick_library);
            }
            self.redraw();
            event_loop.set_control_flow(if iterating || approaching {
                ControlFlow::wait_duration(Duration::from_millis(2))
            } else {
                ControlFlow::Wait
            });
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
