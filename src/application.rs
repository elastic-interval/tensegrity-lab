use crate::build::algo::klein::generate_klein;
use crate::build::algo::mobius::generate_mobius;
#[cfg(not(target_arch = "wasm32"))]
use crate::build::algo::tensegrity_sphere::generate_sphere;
use crate::build::dsl::fabric_library;
use crate::crucible::Crucible;
use crate::keyboard::Keyboard;
use crate::pointer::PointerHandler;
use crate::scene::Scene;
use crate::wgpu::Wgpu;
use crate::{
    Age, ControlState, CrucibleAction, LabEvent, Radio, RunStyle, StateChange, TesterAction,
};
use instant::{Duration, Instant};
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{WindowAttributes, WindowId};

#[cfg(not(target_arch = "wasm32"))]
use crate::animation_export::AnimationExporter;
use crate::build::dsl::fabric_library::FabricName;
#[cfg(not(target_arch = "wasm32"))]
use crate::physics_gpu::GpuBatch;
#[cfg(not(target_arch = "wasm32"))]
use crate::units::Seconds;

/// State that only exists in the native build: animation export, CSV export
/// recording, and the on-demand GPU compute batch. Grouped here so the
/// `Application` struct doesn't sprout three separate `#[cfg]` fields.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
struct NativeState {
    animation_exporter: Option<AnimationExporter>,
    record_until: Option<Seconds>,
    gpu_batch: Option<GpuBatch>,
}

/// Show-mode state (cycle): walk through every named fabric, pause
/// `CYCLE_DWELL` after each one completes (`FabricBuilt`), then move to
/// the next. Triggered by the native `--cycle` flag; always on in WASM.
struct CycleState {
    names: Vec<FabricName>,
    index: usize,
    advance_at: Option<Instant>,
}

/// Wall-clock time each fabric stays visible in Show mode before
/// advancing to the next.
const CYCLE_DWELL: Duration = Duration::from_secs(5);

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
    last_frame_secs: f32,
    control_state: ControlState,
    pointer_handler: PointerHandler,
    time_scale: f32,
    model_scale: Option<f32>,
    cycle: Option<CycleState>,
    #[cfg(not(target_arch = "wasm32"))]
    native: NativeState,
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
            last_frame_secs: 0.016,
            control_state: ControlState::Waiting,
            time_scale,
            model_scale: model_scale.map(|n| 1.0 / n),
            cycle: None,
            #[cfg(not(target_arch = "wasm32"))]
            native: NativeState::default(),
        }
    }

    /// Adjust time scale by a factor
    pub fn adjust_time_scale(&mut self, factor: f32) {
        self.time_scale = (self.time_scale * factor).clamp(0.1, 100.0);
    }

    /// Enable Show mode: after each fabric finishes, wait `CYCLE_DWELL`
    /// and advance to the next one (loops forever). Triggered by the
    /// native `--cycle` flag; always on in WASM.
    pub fn set_cycle(&mut self, names: Vec<FabricName>) {
        self.cycle = Some(CycleState {
            names,
            index: 0,
            advance_at: None,
        });
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
        let delta = self.last_frame_secs;
        if let Some(scene) = &mut self.scene {
            if scene.needs_camera_init() {
                scene.jump_to_fabric(&self.crucible.fabric);
            }
            if let Err(error) = scene.redraw(&self.crucible.fabric, has_surface, delta) {
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
                self.scene = Some(Scene::new(
                    self.mobile_device,
                    wgpu,
                    self.radio.clone(),
                    self.model_scale,
                ));
                // A `Run(Sphere)` queued from main.rs fires before this
                // event, so the Sphere handler saw scene=None and bailed.
                // Retry now that the scene (and its wgpu device/queue) is
                // available. Sphere is GPU-only and cannot run otherwise.
                if let RunStyle::Sphere { .. } = &self.run_style {
                    LabEvent::Run(self.run_style.clone()).send(&self.radio);
                }
                if self.cycle.is_some() {
                    StateChange::SetShowMode(true).send(&self.radio);
                }
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
                        #[cfg(not(target_arch = "wasm32"))]
                        record,
                        #[cfg(not(target_arch = "wasm32"))]
                        export_fps,
                        ..
                    } => {
                        #[cfg(not(target_arch = "wasm32"))]
                        if let Some(duration) = record {
                            self.native.record_until = Some(*duration);
                            let mut exporter =
                                AnimationExporter::new("animation_export", *export_fps);
                            exporter.start();
                            self.native.animation_exporter = Some(exporter);
                        }
                        let fabric_plan = fabric_library::get_fabric_plan(*fabric_name);
                        CrucibleAction::BuildFabric(fabric_plan).send(&self.radio);
                    }
                    RunStyle::BakeBricks => {
                        StateChange::SetStageLabel("Baking".to_string()).send(&self.radio);
                        ControlState::Baking.send(&self.radio);
                        self.crucible.action(CrucibleAction::StartBaking);
                    }
                    RunStyle::Evolution(seed) => {
                        self.crucible.action(CrucibleAction::ToEvolving(*seed));
                    }
                    #[cfg(target_arch = "wasm32")]
                    RunStyle::Sphere { .. } => {
                        StateChange::SetStageLabel(
                            "Sphere fabric needs native GPU compute".to_string(),
                        )
                        .send(&self.radio);
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    RunStyle::Sphere { frequency, radius } => {
                        use crate::fabric::interval::{Role, Span};
                        use crate::fabric::physics::{presets, Surface, SurfaceCharacter};
                        use crate::fabric::physics_tester::PhysicsTester;
                        use crate::crucible::Stage;
                        use crate::units::{Grams, GramsPerMeter, Meters, Seconds, Unit};

                        // GPU-only algorithmic sphere: no CPU iteration anywhere.
                        // CPU path freezes at frequency ≥ 10 when MAX_SPEED_SQUARED
                        // is exceeded; GPU handles much higher frequencies with a
                        // 1/√f pretension scaling law.
                        let Some(scene) = &mut self.scene else {
                            eprintln!("[sphere] scene not yet initialised; skipping");
                            return;
                        };
                        let device = scene.wgpu.device.clone();
                        let queue = scene.wgpu.queue.clone();

                        let mut fabric = generate_sphere(*frequency, *radius);
                        fabric.dimensions = fabric
                            .dimensions
                            .with_joint_mass(Grams(2.0))
                            .with_push_density(GramsPerMeter(3.0));
                        println!(
                            "[sphere-gpu] freq={} joints={} intervals={} bound_r(initial)={:.3}",
                            frequency,
                            fabric.joints.len(),
                            fabric.intervals.len(),
                            fabric.bounding_radius()
                        );

                        // 1/√f pretension scaling — keeps per-joint strain energy
                        // roughly constant as frequency rises.
                        let f = *frequency as f32;
                        let push_pretension = 1.0 + 0.10 / f.sqrt();
                        let pull_pretension = 1.0 - 0.05 / f.sqrt();
                        let approach_duration = Seconds(2.0);
                        let settle_duration = Seconds(1.0);

                        let cable_actuals: Vec<(crate::fabric::IntervalKey, f32)> = fabric
                            .intervals
                            .iter()
                            .filter(|(_, i)| i.role != Role::Pushing)
                            .map(|(k, i)| {
                                let a = fabric.joints[i.alpha_key].location;
                                let o = fabric.joints[i.omega_key].location;
                                (k, (o - a).length())
                            })
                            .collect();
                        let age = fabric.age;
                        for interval in fabric.intervals.values_mut() {
                            if interval.role == Role::Pushing {
                                if let Span::Fixed { length } = interval.span {
                                    interval.span = Span::Approaching {
                                        start_length: length,
                                        target_length: Meters(length.f32() * push_pretension),
                                        start_age: age,
                                        duration: approach_duration,
                                    };
                                }
                            }
                        }
                        for (key, actual) in cable_actuals {
                            if let Some(interval) = fabric.intervals.get_mut(key) {
                                if let Span::Fixed { length } = interval.span {
                                    interval.span = Span::Approaching {
                                        start_length: length,
                                        target_length: Meters(actual * pull_pretension),
                                        start_age: age,
                                        duration: approach_duration,
                                    };
                                }
                            }
                        }

                        // Material stiffness is k_at_1m / L. Edge length shrinks
                        // as 1/f, so effective stiffness grows as f; natural
                        // frequency ω ∝ √f; Verlet needs dt·ω < 2 for stability.
                        // Scaling rigidity_multiplier as 1/f cancels the f term,
                        // keeping the integrator in the same stability regime at
                        // every frequency (and also softening the spheres
                        // visibly, which is what we want for the drop demo).
                        let rigidity_scale = 1.0 / f;
                        let mut build_physics = presets::CONSTRUCTION;
                        build_physics.tweak.rigidity_multiplier = rigidity_scale;
                        let build_batch = GpuBatch::parallelize(
                            &device,
                            &queue,
                            &[&fabric],
                            &build_physics,
                        );
                        let dt = Age::iteration_duration();
                        let approach_iters = (approach_duration.0 / dt) as u32;
                        let settle_iters = (settle_duration.0 / dt) as u32;
                        let rounds = 40u32;
                        let chunk = (approach_iters / rounds).max(1);
                        for _ in 0..rounds {
                            fabric.age = fabric.age.advanced(chunk as usize);
                            build_batch.update_ideals(&queue, &[&fabric], &build_physics);
                            build_batch.step(&device, &queue, chunk);
                        }
                        if settle_iters > 0 {
                            build_batch.step(&device, &queue, settle_iters);
                        }

                        // Check whether the approach survived on GPU.
                        let frozen_flags = build_batch.read_frozen(&device, &queue);
                        let approach_frozen = frozen_flags.first().copied().unwrap_or(false);
                        if approach_frozen {
                            eprintln!(
                                "[sphere-gpu] freq={} APPROACH FROZE on GPU (speed > {} m/s)",
                                frequency,
                                crate::physics_gpu::params::GpuPhysicsConfig::from_fabric(
                                    &fabric,
                                    &build_physics,
                                )
                                .speed_limit
                            );
                        }

                        // Read settled positions back into the fabric.
                        let settled = build_batch.read_positions(&device, &queue);
                        for (joint, pos) in fabric.joints.values_mut().zip(settled.iter()) {
                            joint.location = *pos;
                            joint.velocity = glam::Vec3::ZERO;
                        }
                        fabric.update_bounding_radius();

                        // Convert Approaching spans to Fixed so the renderer
                        // stops drawing intervals in red (the "approaching"
                        // highlight) and so any future CPU logic sees a
                        // settled fabric.
                        for interval in fabric.intervals.values_mut() {
                            if let Span::Approaching { target_length, .. } = interval.span {
                                interval.span = Span::Fixed { length: target_length };
                            }
                        }
                        println!(
                            "[sphere-gpu] after settle: bound_r={:.3} centroid={:?}",
                            fabric.bounding_radius(),
                            fabric.centroid()
                        );

                        // Raise to drop altitude and switch to falling physics.
                        let r = *radius;
                        let translation = fabric.centralize_translation(Some(r));
                        fabric.apply_translation(translation);
                        fabric.update_bounding_radius();

                        let mut drop_physics = presets::FALLING;
                        drop_physics.surface = Some(Surface::new(SurfaceCharacter::Bouncy, 1.0));
                        drop_physics.tweak.rigidity_multiplier = rigidity_scale;

                        // Fresh GpuBatch for the drop phase (positions changed;
                        // physics preset changed). Store as the live-GPU batch.
                        let drop_batch = GpuBatch::parallelize(
                            &device,
                            &queue,
                            &[&fabric],
                            &drop_physics,
                        );
                        self.native.gpu_batch = Some(drop_batch);

                        let tester = PhysicsTester::new(
                            fabric.clone(),
                            drop_physics.clone(),
                            self.radio.clone(),
                        );
                        self.crucible.fabric = fabric;
                        self.crucible.physics = drop_physics;
                        self.crucible.stage = Stage::PhysicsTesting(tester);
                        self.control_state = ControlState::PhysicsTesting;
                        StateChange::SetControlState(ControlState::PhysicsTesting)
                            .send(&self.radio);
                        let initial_label = if approach_frozen {
                            "Sphere Drop (GPU — APPROACH FROZE)"
                        } else {
                            "Sphere Drop (GPU)"
                        };
                        StateChange::SetStageLabel(initial_label.to_string())
                            .send(&self.radio);
                        if let Some(scene) = &mut self.scene {
                            scene.position_camera_for_drop(r);
                        }
                    }
                    RunStyle::Mobius { segments } => {
                        let fabric = generate_mobius(*segments);
                        self.crucible.action(CrucibleAction::LoadAlgoFabric(fabric));
                    }
                    RunStyle::Klein { width, height, shift } => {
                        let fabric = generate_klein(*width, *height, *shift);
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
                if self.mobile_device && self.crucible.animation_available() {
                    // Auto-start animation on mobile devices with actuators
                    CrucibleAction::ToAnimating.send(&self.radio);
                } else {
                    self.crucible.viewing_state().send(&self.radio);
                }
                if let Some(cycle) = &mut self.cycle {
                    cycle.advance_at = Some(Instant::now() + CYCLE_DWELL);
                    // No explicit refit needed — about_to_wait re-tends
                    // the camera every frame to the current fabric.
                }
            }
            Crucible(crucible_action) => {
                self.crucible.action(crucible_action);
            }
            RebuildFabric => {
                #[cfg(not(target_arch = "wasm32"))]
                { self.native.gpu_batch = None; }
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
                // In Show mode, ignore RestartApproach so the crucible's
                // mid-construction stage transitions (Pretensing/Falling)
                // don't yank the orbiting camera back to the default angle.
                if self.cycle.is_some()
                    && matches!(app_change, StateChange::RestartApproach)
                {
                    return;
                }
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
                        if !matches!(self.control_state, ControlState::PhysicsTesting) {
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
                    // Connector-less fabrics have no attachment points to show
                    if self.crucible.fabric.connector.is_some() {
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
                    }
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
                if let Some(exporter) = &mut self.native.animation_exporter {
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
                    self.native.animation_exporter = None;
                } else {
                    // Start recording
                    let mut exporter = AnimationExporter::new("animation_export", 100.0);
                    exporter.start();
                    StateChange::SetStageLabel("Recording...".to_string()).send(&self.radio);
                    self.native.animation_exporter = Some(exporter);
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            ExportSnapshot => {
                let exporter = self
                    .native
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
            #[cfg(not(target_arch = "wasm32"))]
            ToGpuPhysics => {
                if self.native.gpu_batch.is_none() {
                    if let Some(scene) = &self.scene {
                        let physics = self.crucible.physics.clone();
                        let batch = GpuBatch::parallelize(
                            &scene.wgpu.device,
                            &scene.wgpu.queue,
                            &[&self.crucible.fabric],
                            &physics,
                        );
                        self.native.gpu_batch = Some(batch);
                        StateChange::SetStageLabel("GPU Physics".to_string()).send(&self.radio);
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
                // In Show mode the bottom legend is hidden, so we
                // suppress all key bindings too — only window-level keys
                // (e.g. CloseRequested via Cmd+W / Alt+F4) still work.
                if self.cycle.is_some() {
                    return;
                }
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

        // Show mode: continuously re-tend the camera so it tracks the
        // bounding sphere as the structure grows during Build/Pretense,
        // rotate it slowly around the vertical, and when the dwell
        // timer fires, send Run() for the next fabric.
        if let Some(cycle) = &mut self.cycle {
            let elapsed = now.duration_since(self.last_update).as_secs_f32();
            // 2 rpm = 2 * 2π rad / 60 s = π / 15 rad/s.
            let rate = std::f32::consts::TAU * 2.0 / 60.0;
            if let Some(scene) = &mut self.scene {
                scene.refit_camera_to_fabric(&self.crucible.fabric);
                scene.orbit_camera_y(rate * elapsed);
            }
            if cycle.advance_at.map_or(false, |when| now >= when) {
                cycle.advance_at = None;
                cycle.index = (cycle.index + 1) % cycle.names.len();
                let next = cycle.names[cycle.index];
                LabEvent::Run(RunStyle::Fabric {
                    fabric_name: next,
                    record: None,
                    export_fps: 100.0,
                })
                .send(&self.radio);
            }
        }

        // Handle elapsed time since last update
        let elapsed = now.duration_since(self.last_update);

        // If too much time has passed, reset accumulated time to avoid spiral of death
        if elapsed > Duration::from_millis(100) {
            self.last_update = now;
            self.accumulated_time = Duration::from_secs(0);
            self.last_frame_secs = 0.016; // Reset to nominal
            return;
        }

        self.last_frame_secs = elapsed.as_secs_f32();
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
            let iterations_per_second = Age::iterations_per_second();
            let iterations_per_frame = if self.current_fps > 0.0 && animate {
                (self.time_scale * iterations_per_second / self.current_fps).round() as usize
            } else {
                0
            };

            if iterations_per_frame > 0 {
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(batch) = &self.native.gpu_batch {
                    if let Some(scene) = &self.scene {
                        batch.step(&scene.wgpu.device, &scene.wgpu.queue, iterations_per_frame as u32);
                        let positions = batch.read_positions(&scene.wgpu.device, &scene.wgpu.queue);
                        // Periodically check the GPU's frozen flag; if set, surface
                        // it once as a stage label so the user can tell "frozen"
                        // apart from "slow". Also print a y-range once per second
                        // so we can see whether the sphere is actually moving.
                        use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};
                        static FRAME_COUNT: AtomicU32 = AtomicU32::new(0);
                        static FROZEN_REPORTED: AtomicBool = AtomicBool::new(false);
                        let n = FRAME_COUNT.fetch_add(1, Ordering::Relaxed);
                        if n % 60 == 0 {
                            if !FROZEN_REPORTED.load(Ordering::Relaxed) {
                                let frozen = batch.read_frozen(&scene.wgpu.device, &scene.wgpu.queue);
                                if frozen.iter().any(|&f| f) {
                                    FROZEN_REPORTED.store(true, Ordering::Relaxed);
                                    StateChange::SetStageLabel(
                                        "GPU Frozen (speed limit)".to_string(),
                                    )
                                    .send(&self.radio);
                                }
                            }
                            if !positions.is_empty() {
                                let y_max = positions.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
                                let y_min = positions.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
                                eprintln!(
                                    "[gpu-live] frame={} iters={} joints={} y=[{:.3},{:.3}]",
                                    n, iterations_per_frame, positions.len(), y_min, y_max,
                                );
                            }
                        }
                        for (joint, pos) in self.crucible.fabric.joints.values_mut().zip(positions.iter()) {
                            joint.location = *pos;
                        }
                    }
                } else {
                    self.crucible.iterate(iterations_per_frame);
                }
                #[cfg(target_arch = "wasm32")]
                self.crucible.iterate(iterations_per_frame);
            }

            // Capture frame for animation export if enabled (works in all states)
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(exporter) = &mut self.native.animation_exporter {
                let dominated = self.native.record_until.is_some_and(|Seconds(limit)| {
                    self.crucible.fabric.age.as_duration().as_secs_f32() >= limit
                });
                if dominated {
                    let frame_count = exporter.frame_count();
                    match exporter.stop() {
                        Ok(_) => eprintln!("Recording complete: {} frames", frame_count),
                        Err(e) => eprintln!("Error stopping animation export: {}", e),
                    }
                    self.native.record_until = None;
                    self.native.animation_exporter = None;
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
