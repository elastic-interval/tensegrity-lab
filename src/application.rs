use crate::build::tenscript::brick_library::BrickLibrary;
use crate::build::tenscript::fabric_library::FabricLibrary;
use crate::build::tenscript::{FabricPlan, TenscriptError};
use crate::crucible::Crucible;
use crate::fabric::Fabric;
use crate::keyboard::Keyboard;
use crate::pointer::PointerHandler;
use crate::scene::Scene;
use crate::wgpu::Wgpu;
use crate::{
    ControlState, CrucibleAction, LabEvent, Radio, RunStyle, StateChange, TestScenario,
    TesterAction,
};
use instant::{Duration, Instant};
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::time::SystemTime;

use winit::event::WindowEvent;
use winit::application::ApplicationHandler;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{WindowAttributes, WindowId};

pub struct Application {
    run_style: RunStyle,
    mobile_device: bool,
    window_attributes: WindowAttributes,
    scene: Option<Scene>,
    keyboard: Keyboard,
    crucible: Crucible,
    fabric_library: FabricLibrary,
    brick_library: BrickLibrary,
    radio: Radio,
    last_update: Instant,
    accumulated_time: Duration,
    frames_count: u32,
    fps_timer: Instant,
    control_state: ControlState,
    pointer_handler: PointerHandler,
    #[cfg(not(target_arch = "wasm32"))]
    fabric_library_modified: Instant,
    #[cfg(not(target_arch = "wasm32"))]
    machine: Option<crate::cord_machine::CordMachine>,
}

impl Application {
    //==================================================
    // Construction and Initialization
    //==================================================
    
    /// Create a new Application instance
    pub fn new(
        window_attributes: WindowAttributes,
        radio: Radio,
    ) -> Result<Application, TenscriptError> {
        let brick_library = BrickLibrary::from_source()?;
        let fabric_library = FabricLibrary::from_source()?;
        Ok(Application {
            run_style: RunStyle::Unknown,
            mobile_device: false,
            window_attributes,
            radio: radio.clone(),
            keyboard: Keyboard::new(radio.clone()).with_actions(),
            brick_library,
            fabric_library,
            scene: None,
            crucible: Crucible::new(radio.clone()),
            last_update: Instant::now(),
            accumulated_time: Duration::default(),
            pointer_handler: PointerHandler::new(radio.clone()),
            frames_count: 0,
            fps_timer: Instant::now(),
            control_state: ControlState::Waiting,
            #[cfg(not(target_arch = "wasm32"))]
            fabric_library_modified: fabric_library_modified(),
            #[cfg(not(target_arch = "wasm32"))]
            machine: None,
        })
    }
    
    //==================================================
    // Public API Methods
    //==================================================
    
    pub fn refresh_library(&mut self, time: Instant) -> Result<LabEvent, TenscriptError> {
        self.fabric_library = FabricLibrary::from_source()?;
        Ok(LabEvent::UpdatedLibrary(time))
    }
    
    pub fn get_fabric_plan(&self, plan_name: &String) -> Result<FabricPlan, TenscriptError> {
        let plan = self
            .fabric_library
            .fabric_plans
            .iter()
            .find(|plan| plan.name == *plan_name);
        match plan {
            None => Err(TenscriptError::InvalidError(plan_name.clone())),
            Some(plan) => Ok(plan.clone()),
        }
    }
    
    //==================================================
    // Private Helper Methods
    //==================================================
    

    
    /// Access the scene if it exists, executing the provided closure
    /// Returns Some(R) if the scene exists and the closure was executed
    /// Returns None if the scene doesn't exist
    fn with_scene<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut Scene) -> R,
    {
        self.scene.as_mut().map(f)
    }
    
    /// Access both scene and fabric at the same time if scene exists
    /// Returns Some(R) if the scene exists and the closure was executed
    /// Returns None if the scene doesn't exist
    fn with_scene_and_fabric<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut Scene, &mut Fabric) -> R,
    {
        self.scene.as_mut().map(|scene| {
            let fabric = &mut self.crucible.fabric;
            f(scene, fabric)
        })
    }
    
    fn redraw(&mut self) {
        // Update keyboard legend
        StateChange::SetKeyboardLegend(self.keyboard.legend(&self.control_state).join(", "))
            .send(&self.radio);

        // Check if fabric library has been modified
        #[cfg(not(target_arch = "wasm32"))]
        {
            let time = fabric_library_modified();
            if time > self.fabric_library_modified {
                match self.refresh_library(time) {
                    Ok(action) => {
                        action.send(&self.radio);
                    }
                    Err(tenscript_error) => {
                        println!("Tenscript\n{tenscript_error}");
                        self.fabric_library_modified = Instant::now();
                    }
                }
            }
        }

        // Use with_scene_and_fabric and handle the Option return type
        if let Some(result) = self.with_scene_and_fabric(|scene, fabric| scene.redraw(fabric)) {
            if let Err(error) = result {
                eprintln!("Error redrawing scene: {:?}", error);
            }
        }
    }

    //==================================================
    // Initialization Helpers
    //==================================================

    #[cfg(target_arch = "wasm32")]
    fn initialize_wgpu_when_ready(&self, window: Arc<winit::window::Window>, radio: Radio) {
        use std::cell::RefCell;
        use std::rc::Rc;
        use wasm_bindgen::prelude::*;
        use web_sys::console;

        // Create a recursive frame checking closure
        struct FrameChecker {
            window: Arc<winit::window::Window>,
            radio: Radio,
            closure: Option<Closure<dyn FnMut()>>,
        }

        let checker = Rc::new(RefCell::new(FrameChecker {
            window,
            radio,
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
                    checker_ref.radio.clone(),
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
        let window = web_sys::window().expect("no global window");
        
        let borrow = checker.borrow();
        
        if let Some(closure_ref) = &borrow.closure {
            let _ = window.request_animation_frame(closure_ref.as_ref().unchecked_ref());
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

        #[cfg(target_arch = "wasm32")]
        self.initialize_wgpu_when_ready(window, self.radio.clone());

        #[cfg(not(target_arch = "wasm32"))]
        Wgpu::create_and_send(false, window, self.radio.clone());
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: LabEvent) {
        use LabEvent::*;
        match event {
            ContextCreated {
                wgpu,
                mobile_device,
            } => {
                self.mobile_device = mobile_device;
                self.scene = Some(Scene::new(self.mobile_device, wgpu, self.radio.clone()));
                ControlState::Waiting.send(&self.radio);
            }
            Run(run_style) => {
                self.run_style = run_style;
                // Use the new with_scene method to handle the scene existence check
                self.with_scene(|scene| scene.normal_rendering());
                match &self.run_style {
                    RunStyle::Unknown => {
                        unreachable!()
                    }
                    RunStyle::Fabric { fabric_name, .. } => {
                        match self.get_fabric_plan(&fabric_name) {
                            Ok(fabric_plan) => {
                                CrucibleAction::BuildFabric(fabric_plan).send(&self.radio);
                            }
                            Err(error) => {
                                panic!("Error loading fabric [{fabric_name}]: {error}");
                            }
                        }
                    }
                    RunStyle::Prototype(brick_index) => {
                        let prototype = self
                            .brick_library
                            .brick_definitions
                            .get(*brick_index)
                            .expect("no such brick")
                            .proto
                            .clone();
                        ControlState::Baking.send(&self.radio);
                        self.crucible.action(CrucibleAction::BakeBrick(prototype));
                    }
                    RunStyle::Seeded(seed) => {
                        let _ = self.crucible.action(CrucibleAction::ToEvolving(*seed));
                    }
                };
            }
            FabricBuilt(fabric_stats) => {
                StateChange::SetFabricName(fabric_stats.name.clone()).send(&self.radio);
                StateChange::SetFabricStats(Some(fabric_stats)).send(&self.radio);
                if self.mobile_device {
                    CrucibleAction::ToAnimating.send(&self.radio);
                } else {
                    if let RunStyle::Fabric {
                        scenario: Some(scenario),
                        ..
                    } = &self.run_style
                    {
                        match scenario {
                            TestScenario::TensionTest | TestScenario::CompressionTest => {
                                CrucibleAction::ToFailureTesting(scenario.clone())
                                    .send(&self.radio);
                            }
                            TestScenario::PhysicsTest => {
                                CrucibleAction::ToPhysicsTesting(scenario.clone())
                                    .send(&self.radio);
                            }
                            TestScenario::BoxingTest => {
                                CrucibleAction::ToBoxingProcess(scenario.clone()).send(&self.radio);
                            }
                            TestScenario::MachineTest(ip_address) => {
                                println!("Running machine test at {ip_address}");
                                #[cfg(not(target_arch = "wasm32"))]
                                match crate::cord_machine::CordMachine::new(ip_address) {
                                    Ok(machine) => self.machine = Some(machine),
                                    Err(error) => {
                                        panic!("Machine [{ip_address}]: {error}");
                                    }
                                }
                                ControlState::Viewing.send(&self.radio);
                            }
                        }
                    } else {
                        ControlState::Viewing.send(&self.radio);
                    }
                }
            }
            Crucible(crucible_action) => {
                self.crucible.action(crucible_action);
            }
            UpdatedLibrary(time) => {
                println!("Reloading library at {time:?}");
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let _fabric_library = self.fabric_library.clone();
                    self.fabric_library_modified = Instant::now();
                    Run(self.run_style.clone()).send(&self.radio);
                }
            }
            ExportAttachmentPoints => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let export_content = self.crucible.fabric.export_attachment_points();
                    let filename = chrono::Local::now()
                        .format("attachment-points-%Y-%m-%d-%H-%M.txt")
                        .to_string();
                    std::fs::write(&filename, export_content)
                        .unwrap_or_else(|e| eprintln!("Failed to write {}: {}", filename, e));
                    println!("Exported attachment points to: {}", filename);
                }
            }
            DumpCSV => {
                #[cfg(not(target_arch = "wasm32"))]
                std::fs::write(
                    chrono::Local::now()
                        .format("pretenst-%Y-%m-%d-%H-%M.zip")
                        .to_string(),
                    self.crucible.fabric.to_zip().unwrap(),
                )
                .unwrap();
            }
            PrintCord(length) => {
                println!("Print cord {length:?}");
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if let Some(machine) = &self.machine {
                        match machine.make_wire(length) {
                            Ok(_) => {
                                println!("Printed!")
                            }
                            Err(error) => {
                                panic!("Machine: {error}");
                            }
                        }
                    }
                }
            }
            RequestRedraw => {
                // Force a redraw to update the visualization immediately
                if let Some(_) = &self.scene {
                    self.redraw();
                }
            },
            UpdateState(app_change) => {
                match &app_change {
                    StateChange::SetControlState(control_state) => {
                        self.control_state = control_state.clone();
                        StateChange::SetKeyboardLegend(
                            self.keyboard.legend(control_state).join(", "),
                        )
                        .send(&self.radio);
                    }
                    StateChange::SetPhysicsParameter(parameter) => {
                        self.keyboard.set_float_parameter(parameter);
                        CrucibleAction::TesterDo(TesterAction::SetPhysicalParameter(
                            parameter.clone(),
                        ))
                        .send(&self.radio);
                        StateChange::SetKeyboardLegend(
                            self.keyboard.legend(&self.control_state).join(", "),
                        )
                        .send(&self.radio);
                    }
                    _ => {}
                }
                if let StateChange::ToggleAttachmentPoints = &app_change {
                    let should_update = self.with_scene(|scene| {
                        scene.update_state(app_change.clone());
                        scene.render_style_shows_attachment_points()
                    }).unwrap_or(false);
                    
                    if should_update {
                        self.crucible.update_attachment_connections();
                    }
                    
                    RequestRedraw.send(&self.radio);
                } else {
                    self.with_scene(|scene| scene.update_state(app_change.clone()));
                }
            }
            PointerChanged(pointer_change) => {
                // Forward pointer events to the scene
                self.with_scene_and_fabric(|scene, _fabric| {
                    scene.pointer_changed(pointer_change, _fabric);
                });
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // Handle events that don't need scene access
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                return;
            },
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                self.keyboard.handle_key_event(key_event, &self.control_state);
                return;
            },
            _ => {}
        }
        
        // Early return if no scene
        if self.scene.is_none() {
            return;
        }
        
        // Let the pointer handler process the event first
        if self.pointer_handler.process_window_event(&event) {
            return; // Event was handled by the pointer handler
        }
        
        // Handle other window events that need scene access
        self.with_scene(|scene| {
            match event {
                WindowEvent::Resized(physical_size) => scene.resize(physical_size),
                _ => {}
            }
        });
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Process time-related updates regardless of scene existence
        let now = Instant::now();

        // FPS calculation with platform-specific adjustments
        self.frames_count += 1;
        let fps_elapsed = now.duration_since(self.fps_timer);
        
        // Only update FPS display once per second
        if fps_elapsed >= Duration::from_secs(1) {
            // Calculate frames per second with platform-specific adjustments
            #[cfg(target_arch = "wasm32")]
            let frames_per_second = {
                // In WASM, we need to cap the reported FPS to avoid absurd values
                // This happens because the browser's requestAnimationFrame timing can be inconsistent
                let raw_fps = self.frames_count as f32 / fps_elapsed.as_secs_f32();
                f32::min(raw_fps, 120.0) // Cap at 120 FPS for display purposes
            };
            
            #[cfg(not(target_arch = "wasm32"))]
            let frames_per_second = self.frames_count as f32 / fps_elapsed.as_secs_f32();
            
            // Get fabric age
            let age = self.crucible.fabric.age;
            
            // Send the FPS update event
            StateChange::Time {
                frames_per_second,
                age,
            }.send(&self.radio);
            
            // Reset counters
            self.frames_count = 0;
            self.fps_timer = now;
        }

        // Handle elapsed time since last update
        let elapsed = now.duration_since(self.last_update);
        
        // If too much time has passed, reset accumulated time to avoid spiral of death
        if elapsed > Duration::from_millis(100) {
            self.last_update = now;
            self.accumulated_time = Duration::from_secs(0);
            return;
        }
        
        self.last_update = now;
        
        // Cap elapsed time to avoid large time steps
        #[cfg(target_arch = "wasm32")]
        let capped_elapsed = std::cmp::min(elapsed, Duration::from_millis(16)); // ~60 FPS cap for WASM
        
        #[cfg(not(target_arch = "wasm32"))]
        let capped_elapsed = std::cmp::min(elapsed, Duration::from_millis(33)); // ~30 FPS cap for native
        
        self.accumulated_time += capped_elapsed;
        
        // Define update interval (how often physics steps are taken)
        let update_interval = Duration::from_millis(10);
        
        // Check if animation is active
        let animate = self.with_scene_and_fabric(|scene, fabric| scene.animate(fabric)).unwrap_or(false);
        
        // Limit updates per frame
        let mut updates_this_frame = 0;
        let max_updates_per_frame = 3;
        
        while self.accumulated_time >= update_interval && updates_this_frame < max_updates_per_frame {
            self.accumulated_time -= update_interval;
            updates_this_frame += 1;

            if animate {
                self.crucible.iterate(&self.brick_library);
            }
        }

        if updates_this_frame > 0 {
            let _ = self.redraw();
        }

        // Set platform-specific control flow
        #[cfg(target_arch = "wasm32")]
        event_loop.set_control_flow(ControlFlow::Poll);
        
        #[cfg(not(target_arch = "wasm32"))]
        event_loop.set_control_flow(if animate {
            ControlFlow::wait_duration(Duration::from_millis(16)) // ~60 FPS when animating
        } else {
            ControlFlow::Wait // Wait for events when not animating
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn fabric_library_modified() -> Instant {
    use std::fs;
    use std::cell::RefCell;
    
    // Use thread_local storage to track the last modification time
    thread_local! {
        static LAST_MOD_TIME: RefCell<Option<SystemTime>> = RefCell::new(None);
    }
    
    // Get the current file modification time
    let current_mod_time = fs::metadata("fabric_library.tenscript")
        .and_then(|metadata| metadata.modified())
        .ok();
    
    // Check if the file has been modified
    let file_changed = LAST_MOD_TIME.with(|last_time| {
        let mut last = last_time.borrow_mut();
        let changed = match (current_mod_time, *last) {
            (Some(current), Some(previous)) => current > previous,
            (Some(_), None) => true, // First time checking
            _ => false, // Error reading file or no change
        };
        
        // Update the last modification time if we have a valid time
        if let Some(current) = current_mod_time {
            *last = Some(current);
        }
        
        changed
    });
    
    // Only return a new Instant if the file has changed
    if file_changed {
        Instant::now()
    } else {
        // Return a zero instant to indicate no change
        // This will never be greater than any previous Instant
        Instant::now() - Duration::from_secs(60 * 60 * 24) // 24 hours ago
    }
}
