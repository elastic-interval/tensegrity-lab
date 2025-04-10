use crate::build::tenscript::brick_library::BrickLibrary;
use crate::build::tenscript::fabric_library::FabricLibrary;
use crate::build::tenscript::{FabricPlan, TenscriptError};
use crate::crucible::Crucible;
use crate::keyboard::Keyboard;
use crate::messages::{
    ControlState, CrucibleAction, LabEvent, PhysicsTesterAction, PointerChange, Radio, RunStyle,
    Shot, StateChange, TestScenario,
};
use crate::scene::Scene;
use crate::wgpu::Wgpu;
use instant::{Duration, Instant};
use std::sync::Arc;
use std::time::SystemTime;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent};
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
    active_touch_count: usize,
    frames_count: u32,
    fps_timer: Instant,
    control_state: ControlState,
    #[cfg(not(target_arch = "wasm32"))]
    fabric_library_modified: SystemTime,
    #[cfg(not(target_arch = "wasm32"))]
    machine: Option<crate::cord_machine::CordMachine>,
}

impl Application {
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
            scene: None,
            keyboard: Keyboard::new(radio.clone()).with_actions(),
            crucible: Crucible::new(radio.clone()),
            brick_library,
            fabric_library,
            radio,
            last_update: Instant::now(),
            accumulated_time: Duration::default(),
            active_touch_count: 0,
            frames_count: 0,
            fps_timer: Instant::now(),
            control_state: ControlState::Waiting,
            #[cfg(not(target_arch = "wasm32"))]
            fabric_library_modified: fabric_library_modified(),
            #[cfg(not(target_arch = "wasm32"))]
            machine: None,
        })
    }

    fn redraw(&mut self) -> Result<(), wgpu::SurfaceError> {
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
                if let Some(scene) = &mut self.scene {
                    scene.normal_rendering();
                }
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
                println!("Reloading library");
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let _fabric_library = self.fabric_library.clone();
                    self.fabric_library_modified = time;
                    Run(self.run_style.clone()).send(&self.radio);
                }
            }
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
                        CrucibleAction::PhysicsTesterDo(PhysicsTesterAction::SetPhysicalParameter(
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
                if let Some(scene) = &mut self.scene {
                    scene.update_state(app_change);
                }
            }
            DumpCSV => {
                #[cfg(not(target_arch = "wasm32"))]
                std::fs::write(
                    chrono::Local::now()
                        .format("pretenst-%Y-%m-%d-%H-%M.zip")
                        .to_string(),
                    self.crucible.fabric().to_zip().unwrap(),
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
                } => self
                    .keyboard
                    .handle_key_event(key_event, &self.control_state),
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
                                PointerChange::Zoomed((position.y as f32) * 0.018)
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
            let now = Instant::now();

            // FPS
            self.frames_count += 1;
            let fps_elapsed = now.duration_since(self.fps_timer);
            if fps_elapsed >= Duration::from_secs(1) {
                let frames_per_second = self.frames_count as f32 / fps_elapsed.as_secs_f32();
                let age = self.crucible.fabric().age;
                StateChange::Time {
                    frames_per_second,
                    age,
                }
                .send(&self.radio);
                // Reset counters
                self.frames_count = 0;
                self.fps_timer = now;
            }

            // Check if we've been inactive for too long (e.g., window was minimized)
            let elapsed = now.duration_since(self.last_update);
            if elapsed > Duration::from_millis(100) {
                // We were inactive - reset the timer without accumulating time
                self.last_update = now;
                self.accumulated_time = Duration::from_secs(0);
                return; // Skip this frame entirely
            }
            self.last_update = now;
            let capped_elapsed = std::cmp::min(elapsed, Duration::from_millis(33));
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
