use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crate::application::OverlayChange::SetFabricStats;
use crate::build::tenscript::brick_library::BrickLibrary;
use crate::build::tenscript::fabric_library::FabricLibrary;
use crate::build::tenscript::{FabricPlan, TenscriptError};
use crate::camera::Pick;
#[cfg(target_arch = "wasm32")]
use crate::control_overlay::OverlayState;
use crate::crucible::{Crucible, CrucibleAction, LabAction};
use crate::fabric::FabricStats;
use crate::messages::{ControlState, LabEvent, PointerChange, Shot};
use crate::scene::Scene;
use crate::wgpu::Wgpu;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{WindowAttributes, WindowId};

pub struct Application {
    window_attributes: WindowAttributes,
    scene: Option<Scene>,
    crucible: Crucible,
    fabric_plan_name: String,
    fabric_library: FabricLibrary,
    #[cfg(not(target_arch = "wasm32"))]
    fabric_library_modified: SystemTime,
    brick_library: BrickLibrary,
    #[cfg(target_arch = "wasm32")]
    overlay_state: OverlayState,
    event_loop_proxy: EventLoopProxy<LabEvent>,
    fabric_alive: bool,
    pick_active: bool,
}

#[derive(Clone, Debug)]
pub enum OverlayChange {
    SetControlState(ControlState),
    SetFabricStats(Option<FabricStats>),
}

impl Application {
    pub fn new(
        window_attributes: WindowAttributes,
        #[cfg(target_arch = "wasm32")] overlay_state: OverlayState,
        event_loop_proxy: EventLoopProxy<LabEvent>,
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
            #[cfg(target_arch = "wasm32")]
            overlay_state,
            event_loop_proxy,
            #[cfg(not(target_arch = "wasm32"))]
            fabric_library_modified: fabric_library_modified(),
            fabric_alive: true,
            pick_active: false,
        })
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if !key_event.state.is_pressed() {
            return;
        }
        if let KeyEvent {
            physical_key: PhysicalKey::Code(code),
            ..
        } = key_event
        {
            if code == KeyCode::Escape {
                if let Some(scene) = &mut self.scene {
                    scene.reset();
                }
            }
            if code == KeyCode::KeyX {
                println!("Export:\n\n{}", self.crucible.fabric().csv());
            }
            if code == KeyCode::KeyM {
                self.event_loop_proxy
                    .send_event(LabEvent::Crucible(CrucibleAction::Experiment(
                        LabAction::MuscleToggle,
                    )))
                    .unwrap();
            }
        }
    }

    fn build_current_fabric(&mut self) {
        let fabric_plan = if self.fabric_plan_name.is_empty() {
            None
        } else {
            self.get_fabric_plan(self.fabric_plan_name.clone()).ok()
        };
        self.event_loop_proxy
            .send_event(LabEvent::Crucible(CrucibleAction::BuildFabric(fabric_plan)))
            .unwrap();
    }

    fn redraw(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let time = fabric_library_modified();
            if time > self.fabric_library_modified {
                match self.refresh_library(time) {
                    Ok(action) => {
                        self.event_loop_proxy.send_event(action).unwrap();
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
        let window = Arc::new(
            event_loop
                .create_window(self.window_attributes.clone())
                .unwrap(),
        );
        let event_loop_proxy = self.event_loop_proxy.clone();
        Wgpu::create_and_send(window, event_loop_proxy);
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: LabEvent) {
        match event {
            LabEvent::ContextCreated(wgpu) => {
                let proxy = self.event_loop_proxy.clone();
                self.scene = Some(Scene::new(wgpu, proxy));
            }
            LabEvent::LoadFabric(fabric_plan_name) => {
                self.fabric_plan_name = fabric_plan_name.clone();
                self.build_current_fabric();
                self.fabric_alive = !self.fabric_plan_name.is_empty();
            }
            LabEvent::FabricBuilt(fabric_stats) => {
                self.event_loop_proxy
                    .send_event(LabEvent::OverlayChanged(SetFabricStats(Some(fabric_stats))))
                    .unwrap();
                self.pick_active = true;
            }
            LabEvent::Crucible(crucible_action) => {
                match &crucible_action {
                    // side effect
                    CrucibleAction::BuildFabric(_) => {
                        self.event_loop_proxy
                            .send_event(LabEvent::OverlayChanged(SetFabricStats(None)))
                            .unwrap();
                        self.pick_active = false;
                        if let Some(scene) = &mut self.scene {
                            scene.reset();
                        }
                    }
                    _ => {}
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
            LabEvent::EvolveFromSeed(seed) => self.crucible.action(CrucibleAction::Evolve(seed)),
            LabEvent::OverlayChanged(app_change) => {
                println!("Overlay changed {:?}", app_change);
                #[cfg(target_arch = "wasm32")]
                self.overlay_state.change_happened(app_change);
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(scene) = &mut self.scene {
            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::KeyboardInput {
                    event: key_event, ..
                } => self.handle_key_event(key_event),
                WindowEvent::CursorMoved { position, .. } => {
                    scene.pointer_changed(
                        PointerChange::Moved(position),
                        &mut self.crucible.fabric(),
                    );
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    scene.pointer_changed(
                        match state {
                            ElementState::Pressed => PointerChange::Pressed,
                            ElementState::Released => {
                                let shot = if self.pick_active {
                                    match button {
                                        MouseButton::Right => Shot::Joint,
                                        _ => Shot::Interval,
                                    }
                                } else {
                                    Shot::NoPick
                                };
                                PointerChange::Released(shot)
                            }
                        },
                        &mut self.crucible.fabric(),
                    );
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    scene.pointer_changed(
                        match delta {
                            MouseScrollDelta::LineDelta(_, y) => PointerChange::Zoomed(y * 0.5),
                            MouseScrollDelta::PixelDelta(position) => {
                                PointerChange::Zoomed((position.y as f32) * 0.1)
                            }
                        },
                        &mut self.crucible.fabric(),
                    );
                }
                WindowEvent::CursorEntered { .. } => {
                    self.fabric_alive = true;
                }
                WindowEvent::CursorLeft { .. } => {
                    #[cfg(target_arch = "wasm32")]
                    {
                        self.fabric_alive = false;
                    }
                }
                WindowEvent::Resized(physical_size) => {
                    if let Some(scene) = &mut self.scene {
                        scene.resize(physical_size)
                    }
                }
                _ => {}
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(scene) = &mut self.scene {
            let approaching = scene.target_approach(self.crucible.fabric());
            let pick_active = !matches!(scene.current_pick(), Pick::Nothing);
            let iterating = self.fabric_alive && !pick_active;
            if iterating {
                if let Some(lab_event) = self.crucible.iterate(&self.brick_library) {
                    self.event_loop_proxy.send_event(lab_event).unwrap();
                }
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
    fs::metadata("fabric_library.tenscript")
        .unwrap()
        .modified()
        .unwrap()
}
