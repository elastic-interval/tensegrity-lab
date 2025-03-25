use crate::build::tenscript::brick_library::BrickLibrary;
use crate::build::tenscript::fabric_library::FabricLibrary;
use crate::build::tenscript::{FabricPlan, TenscriptError};
use crate::crucible::{Crucible, CrucibleAction, LabAction};
use crate::fabric::FabricStats;
use crate::messages::{ControlState, LabEvent, PointerChange, Shot};
use crate::scene::Scene;
use crate::wgpu::Wgpu;
use crate::keyboard::Keyboard;
use instant::{Duration, Instant};
use std::sync::Arc;
use std::time::SystemTime;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use winit::window::{WindowAttributes, WindowId};

pub struct Application {
    mobile_device: bool,
    window_attributes: WindowAttributes,
    scene: Option<Scene>,
    keyboard: Keyboard,
    crucible: Crucible,
    fabric_plan_name: String,
    fabric_library: FabricLibrary,
    brick_library: BrickLibrary,
    event_loop_proxy: EventLoopProxy<LabEvent>,
    last_update: Instant,
    accumulated_time: Duration,
    active_touch_count: usize,
    #[cfg(not(target_arch = "wasm32"))]
    fabric_library_modified: SystemTime,
}

#[derive(Clone, Debug)]
pub enum AppStateChange {
    SetIntervalColor {
        key: (usize, usize),
        color: [f32; 4],
        tension: bool,
    },
    SetControlState(ControlState),
    SetFabricStats(Option<FabricStats>),
    SetMusclesActive(bool),
    SetExperimentTitle {
        title: String,
        fabric_stats: FabricStats,
    },
}

impl Application {
    pub fn new(
        window_attributes: WindowAttributes,
        event_loop_proxy: EventLoopProxy<LabEvent>,
    ) -> Result<Application, TenscriptError> {
        let brick_library = BrickLibrary::from_source()?;
        let fabric_library = FabricLibrary::from_source()?;
        Ok(Application {
            mobile_device: false,
            window_attributes,
            scene: None,
            keyboard: Keyboard::new(event_loop_proxy.clone()),
            crucible: Crucible::new(event_loop_proxy.clone()),
            fabric_plan_name: Default::default(),
            brick_library,
            fabric_library,
            event_loop_proxy,
            last_update: Instant::now(),
            accumulated_time: Duration::default(),
            active_touch_count: 0,
            #[cfg(not(target_arch = "wasm32"))]
            fabric_library_modified: fabric_library_modified(),
        })
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

    fn redraw(&mut self) -> Result<(), wgpu::SurfaceError> {
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
            scene.redraw(self.crucible.fabric())?;
        }
        Ok(())
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

    #[cfg(target_arch = "wasm32")]
    fn initialize_wgpu_when_ready(
        &self,
        window: Arc<winit::window::Window>,
        event_loop_proxy: EventLoopProxy<LabEvent>,
    ) {
        use std::cell::RefCell;
        use std::rc::Rc;
        use wasm_bindgen::prelude::*;
        use web_sys::console;

        // Create a recursive frame checking closure
        struct FrameChecker {
            window: Arc<winit::window::Window>,
            event_loop_proxy: EventLoopProxy<LabEvent>,
            closure: Option<Closure<dyn FnMut()>>,
        }

        let checker = Rc::new(RefCell::new(FrameChecker {
            window,
            event_loop_proxy,
            closure: None,
        }));

        // Create the closure that will check window size on each frame
        let checker_clone = checker.clone();
        let closure = Closure::wrap(Box::new(move || {
            let checker_ref = checker_clone.borrow();
            let size = checker_ref.window.inner_size();

            if size.width > 0 && size.height > 0 {
                // Window is ready, initialize WGPU
                console::log_1(&"Window initialized with valid dimensions, starting WGPU".into());
                let mobile_device = size.height > size.width;
                Wgpu::create_and_send(
                    mobile_device,
                    checker_ref.window.clone(),
                    checker_ref.event_loop_proxy.clone(),
                );
            } else {
                // Window not ready, check again next frame
                console::log_1(&"Window not ready yet, checking again...".into());
                let window = web_sys::window().expect("no global window");

                if let Some(closure_ref) = &checker_ref.closure {
                    let _ = window.request_animation_frame(closure_ref.as_ref().unchecked_ref());
                }
            }
        }) as Box<dyn FnMut()>);

        // Store the closure in the checker
        checker.borrow_mut().closure = Some(closure);

        // Start the checking process
        if let Some(closure_ref) = &checker.borrow().closure {
            let window = web_sys::window().expect("no global window");
            let _ = window.request_animation_frame(closure_ref.as_ref().unchecked_ref());
        }

        // The checker will keep itself alive through the Rc cycle until WGPU is initialized
        std::mem::forget(checker);
    }
}

impl ApplicationHandler<LabEvent> for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(self.window_attributes.clone())
                .unwrap(),
        );

        #[cfg(target_arch = "wasm32")]
        self.initialize_wgpu_when_ready(window, self.event_loop_proxy.clone());

        #[cfg(not(target_arch = "wasm32"))]
        Wgpu::create_and_send(false, window, self.event_loop_proxy.clone());
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: LabEvent) {
        match event {
            LabEvent::ContextCreated {
                wgpu,
                mobile_device,
            } => {
                self.mobile_device = mobile_device;
                let proxy = self.event_loop_proxy.clone();
                self.scene = Some(Scene::new(self.mobile_device, wgpu, proxy));
            }
            LabEvent::LoadFabric(fabric_plan_name) => {
                self.fabric_plan_name = fabric_plan_name.clone();
                self.build_current_fabric();
            }
            LabEvent::FabricBuilt(fabric_stats) => {
                self.event_loop_proxy
                    .send_event(LabEvent::AppStateChanged(AppStateChange::SetFabricStats(
                        Some(fabric_stats),
                    )))
                    .unwrap();
                if self.mobile_device {
                    self.event_loop_proxy
                        .send_event(LabEvent::Crucible(CrucibleAction::Experiment(
                            LabAction::MusclesActive(true),
                        )))
                        .unwrap();
                }
            }
            LabEvent::Crucible(crucible_action) => {
                match &crucible_action {
                    // side effect
                    CrucibleAction::BuildFabric(_) => {
                        self.event_loop_proxy
                            .send_event(LabEvent::AppStateChanged(AppStateChange::SetFabricStats(
                                None,
                            )))
                            .unwrap();
                        if let Some(scene) = &mut self.scene {
                            scene.reset();
                        }
                    }
                    _ => {}
                }
                self.crucible.action(crucible_action);
            }
            LabEvent::UpdatedLibrary(time) => {
                println!("{time:?}");
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let _fabric_library = self.fabric_library.clone();
                    self.fabric_library_modified = time;
                    if !self.fabric_plan_name.is_empty() {
                        self.build_current_fabric();
                    }
                }
            }
            LabEvent::CalibrateStrain => {
                // let strain_limits =
                //     self.crucible.fabric().strain_limits(":bow-tie".to_string());
                // self.user_interface.set_strain_limits(strain_limits);
            }
            LabEvent::CapturePrototype(brick_index) => {
                let prototype = self
                    .brick_library
                    .brick_definitions
                    .get(brick_index)
                    .expect("no such brick")
                    .proto
                    .clone();
                self.crucible.action(CrucibleAction::BakeBrick(prototype));
            }
            LabEvent::EvolveFromSeed(seed) => {
                let _ = self.crucible.action(CrucibleAction::Evolve(seed));
            }
            LabEvent::AppStateChanged(app_change) => {
                if let Some(scene) = &mut self.scene {
                    scene.change_happened(app_change);
                }
            }
            LabEvent::DumpCSV => {
                #[cfg(not(target_arch = "wasm32"))]
                std::fs::write(
                    chrono::Local::now()
                        .format("pretenst-%Y-%m-%d-%H-%M.csv")
                        .to_string(),
                    self.crucible.fabric().csv(),
                )
                .unwrap();
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
                } => self.keyboard.handle_key_event(key_event),
                WindowEvent::Touch(touch_event) => match touch_event.phase {
                    TouchPhase::Started => {
                        self.active_touch_count += 1;
                        if self.active_touch_count == 1 {
                            scene.pointer_changed(
                                PointerChange::Pressed,
                                &mut self.crucible.fabric(),
                            );
                        }
                    }
                    TouchPhase::Moved => {
                        if self.active_touch_count == 1 {
                            scene.pointer_changed(
                                PointerChange::Moved(touch_event.location),
                                &mut self.crucible.fabric(),
                            );
                        }
                    }
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        if self.active_touch_count > 0 {
                            self.active_touch_count -= 1;
                        }
                        scene.pointer_changed(
                            PointerChange::Released(Shot::NoPick),
                            &mut self.crucible.fabric(),
                        );
                    }
                },
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
                                let shot = if scene.pick_allowed() {
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
                WindowEvent::Resized(physical_size) => scene.resize(physical_size),
                _ => {}
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(scene) = &mut self.scene {
            // Get current time
            let now = Instant::now();

            // Check if we've been inactive for too long (e.g., window was minimized)
            let elapsed = now.duration_since(self.last_update);
            if elapsed > Duration::from_millis(100) {
                // We were inactive - reset the timer without accumulating time
                self.last_update = now;
                self.accumulated_time = Duration::from_secs(0);
                return; // Skip this frame entirely
            }

            // Normal update for active window
            let capped_elapsed = std::cmp::min(elapsed, Duration::from_millis(33)); // Max ~30 FPS worth of time
            self.last_update = now;

            // Fixed time step logic
            self.accumulated_time += capped_elapsed;
            let update_interval = Duration::from_millis(10);
            let animate = scene.animate(self.crucible.fabric());

            // Limit updates per frame
            let mut updates_this_frame = 0;
            let max_updates_per_frame = 3;

            while self.accumulated_time >= update_interval
                && updates_this_frame < max_updates_per_frame
            {
                self.accumulated_time -= update_interval;
                updates_this_frame += 1;

                if animate {
                    self.crucible.iterate(&self.brick_library);
                }
            }

            // Only redraw if we updated
            if updates_this_frame > 0 {
                self.redraw().expect("Problem redrawing");
            }

            // Set consistent control flow
            event_loop.set_control_flow(if animate {
                ControlFlow::wait_duration(Duration::from_millis(16))
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
