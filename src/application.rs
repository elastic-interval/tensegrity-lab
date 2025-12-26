use crate::animation_export::AnimationExporter;
use crate::build::algo::mobius::generate_mobius;
use crate::build::algo::tensegrity_sphere::generate_sphere;
use crate::build::dsl::fabric_library;
use crate::crucible::Crucible;
use crate::keyboard::Keyboard;
use crate::pointer::PointerHandler;
use crate::scene::Scene;
use crate::units::Seconds;
use crate::wgpu::Wgpu;
use crate::SnapshotMoment;
use crate::{
    ControlState, CrucibleAction, LabEvent, Radio, RunStyle, StateChange, TestScenario,
    TesterAction, ITERATION_DURATION,
};
use instant::{Duration, Instant};
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{WindowAttributes, WindowId};

pub struct Application {
    run_style: RunStyle,
    mobile_device: bool,
    window_attributes: WindowAttributes,
    scene: Option<Scene>,
    keyboard: Keyboard,
    crucible: Crucible,
    radio: Radio,
    last_update: Instant,
    accumulated_time: Duration,
    frames_count: u32,
    fps_timer: Instant,
    current_fps: f32,
    control_state: ControlState,
    pointer_handler: PointerHandler,
    time_scale: f32,
    model_scale: Option<f32>,
    animation_exporter: Option<AnimationExporter>,
    record_until: Option<Seconds>,
    snapshot_moment: Option<SnapshotMoment>,
}

impl Application {
    //==================================================
    // Construction and Initialization
    //==================================================

    pub fn new(
        window_attributes: WindowAttributes,
        radio: Radio,
        time_scale: f32,
        model_scale: Option<f32>,
    ) -> Application {
        Application {
            run_style: RunStyle::Unknown,
            mobile_device: false,
            window_attributes,
            radio: radio.clone(),
            keyboard: Keyboard::new(radio.clone()).with_actions(model_scale.map(|n| 1.0 / n)),
            scene: None,
            crucible: Crucible::new(radio.clone()),
            last_update: Instant::now(),
            accumulated_time: Duration::default(),
            pointer_handler: PointerHandler::new(radio.clone()),
            frames_count: 0,
            fps_timer: Instant::now(),
            current_fps: 60.0,
            control_state: ControlState::Waiting,
            time_scale,
            model_scale: model_scale.map(|n| 1.0 / n),
            animation_exporter: None,
            record_until: None,
            snapshot_moment: None,
        }
    }

    /// Adjust time scale by a factor
    pub fn adjust_time_scale(&mut self, factor: f32) {
        self.time_scale = (self.time_scale * factor).clamp(0.1, 100.0);
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

    fn redraw(&mut self) {
        // Update keyboard legend
        StateChange::SetKeyboardLegend(self.keyboard.legend(&self.control_state).join(", "))
            .send(&self.radio);

        let has_surface = self.crucible.physics.surface.is_some();
        if let Some(scene) = &mut self.scene {
            if scene.needs_camera_init() {
                scene.jump_to_fabric(&self.crucible.fabric);
            }
            if let Err(error) = scene.redraw(&self.crucible.fabric, has_surface) {
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
                self.scene = Some(Scene::new(self.mobile_device, wgpu, self.radio.clone(), self.model_scale));
            }
            Run(run_style) => {
                self.run_style = run_style;
                // Use the new with_scene method to handle the scene existence check
                self.with_scene(|scene| scene.normal_rendering());
                match &self.run_style {
                    RunStyle::Unknown => {
                        unreachable!()
                    }
                    RunStyle::Fabric {
                        fabric_name,
                        record,
                        export_fps,
                        snapshot,
                        ..
                    } => {
                        if let Some(duration) = record {
                            self.record_until = Some(*duration);
                            let mut exporter =
                                AnimationExporter::new("animation_export", *export_fps);
                            exporter.start();
                            self.animation_exporter = Some(exporter);
                        }
                        self.snapshot_moment = *snapshot;
                        let fabric_plan = fabric_library::get_fabric_plan(*fabric_name);
                        CrucibleAction::BuildFabric(fabric_plan).send(&self.radio);
                    }
                    RunStyle::BakeBricks => {
                        StateChange::SetStageLabel("Baking".to_string()).send(&self.radio);
                        ControlState::Baking.send(&self.radio);
                        self.crucible.action(CrucibleAction::StartBaking);
                    }
                    RunStyle::Seeded(seed) => {
                        self.crucible.action(CrucibleAction::ToEvolving(*seed));
                    }
                    RunStyle::Sphere { frequency, radius } => {
                        let fabric = generate_sphere(*frequency, *radius);
                        self.crucible.action(CrucibleAction::LoadAlgoFabric(fabric));
                    }
                    RunStyle::Mobius { segments } => {
                        let fabric = generate_mobius(*segments);
                        self.crucible.action(CrucibleAction::LoadAlgoFabric(fabric));
                    }
                };
            }
            FabricBuilt(fabric_stats) => {
                // Reset time scale to normal when construction completes
                self.time_scale = 1.0;
                StateChange::SetFabricName(fabric_stats.name.clone()).send(&self.radio);
                StateChange::SetFabricStats(Some(fabric_stats)).send(&self.radio);
                StateChange::SetControlState(self.crucible.viewing_state()).send(&self.radio);
                StateChange::SetStageLabel("Viewing".to_string()).send(&self.radio);
                // Handle test scenarios first
                if let RunStyle::Fabric {
                    scenario: Some(TestScenario::PhysicsTest),
                    ..
                } = &self.run_style
                {
                    CrucibleAction::ToPhysicsTesting(TestScenario::PhysicsTest).send(&self.radio);
                } else if self.mobile_device && self.crucible.animation_available() {
                    // Auto-start animation on mobile devices with actuators
                    CrucibleAction::ToAnimating.send(&self.radio);
                } else {
                    self.crucible.viewing_state().send(&self.radio);
                }
            }
            Crucible(crucible_action) => {
                self.crucible.action(crucible_action);
            }
            RebuildFabric => {
                // Rebuild the current fabric with updated physics parameters
                Run(self.run_style.clone()).send(&self.radio);
            }
            NextBrick => {
                if let RunStyle::BakeBricks = &self.run_style {
                    self.crucible.action(CrucibleAction::CycleBrick);
                    if let Some(scene) = &mut self.scene {
                        scene.jump_to_fabric(&self.crucible.fabric);
                    }
                }
            }
            DumpCSV => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let name = format!("{}.csv", self.crucible.fabric.name);
                    if let Err(e) = self.crucible.fabric.snapshot_csv(&name) {
                        eprintln!("Failed to export CSV: {}", e);
                    }
                }
            }
            RequestRedraw => {
                // Force a redraw to update the visualization immediately
                if let Some(_) = &self.scene {
                    self.redraw();
                }
            }
            AdjustTimeScale(factor) => {
                self.adjust_time_scale(factor);
            }
            SetTimeScale(scale) => {
                self.time_scale = scale.clamp(0.1, 100.0);
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
                    StateChange::SetTweakParameter(parameter) => {
                        self.keyboard.set_tweak_parameter(parameter);
                        self.crucible.physics.accept_tweak(parameter.clone());

                        // Trigger fabric rebuild (but not in PhysicsTesting mode)
                        if !matches!(self.control_state, ControlState::PhysicsTesting(_)) {
                            RebuildFabric.send(&self.radio);
                        }

                        CrucibleAction::TesterDo(TesterAction::SetTweakParameter(
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
                if let StateChange::JumpToFabric = &app_change {
                    if let Some(scene) = &mut self.scene {
                        scene.jump_to_fabric(&self.crucible.fabric);
                    }
                } else if let StateChange::ToggleAttachmentPoints = &app_change {
                    // First toggle the state
                    self.with_scene(|scene| {
                        scene.update_state(app_change.clone());
                    });

                    // Then check if we toggled ON (not OFF)
                    let is_now_on = self
                        .with_scene(|scene| scene.render_style_shows_attachment_points())
                        .unwrap_or(false);

                    // Always recalculate attachment connections when toggling ON
                    // because the structure may have deformed since last time
                    if is_now_on {
                        self.crucible.update_attachment_connections();
                    }

                    RequestRedraw.send(&self.radio);
                } else {
                    self.with_scene(|scene| scene.update_state(app_change.clone()));
                }
            }
            PointerChanged(pointer_change) => {
                if let Some(scene) = &mut self.scene {
                    scene.pointer_changed(pointer_change, &self.crucible.fabric);
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            ToggleAnimationExport => {
                if let Some(exporter) = &mut self.animation_exporter {
                    // Stop recording
                    let frame_count = exporter.frame_count();
                    match exporter.stop() {
                        Ok(_) => {
                            let label = format!("Saved {} frames", frame_count);
                            StateChange::SetStageLabel(label).send(&self.radio);
                        }
                        Err(e) => {
                            eprintln!("Animation export error: {}", e);
                            StateChange::SetStageLabel("Export error".to_string())
                                .send(&self.radio);
                        }
                    }
                    self.animation_exporter = None;
                } else {
                    // Start recording
                    let mut exporter = AnimationExporter::new("animation_export", 100.0);
                    exporter.start();
                    StateChange::SetStageLabel("Recording...".to_string()).send(&self.radio);
                    self.animation_exporter = Some(exporter);
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            ExportSnapshot => {
                let exporter = self
                    .animation_exporter
                    .get_or_insert_with(|| AnimationExporter::new("animation_export", 100.0));
                match exporter.snapshot(&self.crucible.fabric) {
                    Ok(path) => {
                        let label = format!(
                            "Snapshot: {}",
                            path.file_name().unwrap_or_default().to_string_lossy()
                        );
                        StateChange::SetStageLabel(label).send(&self.radio);
                    }
                    Err(e) => {
                        eprintln!("Snapshot error: {}", e);
                        StateChange::SetStageLabel("Snapshot error".to_string()).send(&self.radio);
                    }
                }
            }
            SnapshotReached(moment) => {
                // Check if this moment matches our snapshot setting (handles All)
                if let Some(target) = self.snapshot_moment {
                    if target.matches(moment) {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            let filename =
                                format!("{}-{}.csv", self.crucible.fabric.name, moment.suffix());
                            if let Err(e) = self
                                .crucible
                                .fabric
                                .snapshot_csv_with_phase(&filename, Some(moment.suffix()))
                            {
                                eprintln!("Failed to export snapshot {}: {}", filename, e);
                            }
                        }
                        // Clear if not All (All continues to match future moments)
                        if target != SnapshotMoment::All {
                            self.snapshot_moment = None;
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
        // Handle events that don't need scene access
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                return;
            }
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                self.keyboard
                    .handle_key_event(key_event, &self.control_state);
                return;
            }
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
        self.with_scene(|scene| match event {
            WindowEvent::Resized(physical_size) => scene.resize(physical_size),
            _ => {}
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
            let raw_frames_per_second = {
                // In WASM, we need to cap the reported FPS to avoid absurd values
                // This happens because the browser's requestAnimationFrame timing can be inconsistent
                let raw_fps = self.frames_count as f32 / fps_elapsed.as_secs_f32();
                f32::min(raw_fps, 120.0) // Cap at 120 FPS for display purposes
            };

            #[cfg(not(target_arch = "wasm32"))]
            let raw_frames_per_second = self.frames_count as f32 / fps_elapsed.as_secs_f32();

            // Store current FPS for dynamic iteration calculation with exponential smoothing
            // This prevents oscillation by gradually adapting to FPS changes
            let alpha = 0.15; // Smoothing factor (lower = more gradual, 0.1-0.2 works well)
            self.current_fps = alpha * raw_frames_per_second + (1.0 - alpha) * self.current_fps;

            // For display purposes, use the smoothed value
            let frames_per_second = self.current_fps;

            // Get fabric age and time scale
            let age = self.crucible.fabric.age;

            // Send the FPS update event
            StateChange::Time {
                frames_per_second,
                age,
                time_scale: self.time_scale,
            }
            .send(&self.radio);

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

        // Check if animation/physics should be active
        // Always call scene.animate() to update camera, then check if physics should run
        let camera_animating = self
            .scene
            .as_mut()
            .map(|scene| scene.animate(&self.crucible.fabric))
            .unwrap_or(false);
        let animate = !matches!(
            self.control_state,
            ControlState::Waiting | ControlState::Baking
        ) || camera_animating;

        // Limit updates per frame
        let mut updates_this_frame = 0;
        let max_updates_per_frame = 3;

        while self.accumulated_time >= update_interval && updates_this_frame < max_updates_per_frame
        {
            self.accumulated_time -= update_interval;
            updates_this_frame += 1;

            // Calculate iterations needed to maintain time scale
            // iterations_per_second = 1.0 / ITERATION_DURATION.secs (e.g., 20000 for 50µs)
            let iterations_per_second = 1.0 / ITERATION_DURATION.secs;
            let iterations_per_frame = if self.current_fps > 0.0 && animate {
                (self.time_scale * iterations_per_second / self.current_fps).round() as usize
            } else {
                0
            };

            if iterations_per_frame > 0 {
                self.crucible.iterate(iterations_per_frame);
            }

            // Capture frame for animation export if enabled (works in all states)
            if let Some(exporter) = &mut self.animation_exporter {
                let dominated = self.record_until.is_some_and(|Seconds(limit)| {
                    self.crucible.fabric.age.as_duration().as_secs_f32() >= limit
                });
                if dominated {
                    let frame_count = exporter.frame_count();
                    match exporter.stop() {
                        Ok(_) => eprintln!("Recording complete: {} frames", frame_count),
                        Err(e) => eprintln!("Error stopping animation export: {}", e),
                    }
                    self.record_until = None;
                    self.animation_exporter = None;
                } else {
                    exporter.tick(&self.crucible.fabric, iterations_per_frame);
                }
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
