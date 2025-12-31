#[allow(unused_imports)]
use getrandom;
use std::error::Error;

use clap::Parser;
use winit::event_loop::EventLoop;
use winit::window::WindowAttributes;

use tensegrity_lab::application::Application;
use tensegrity_lab::build::dsl::fabric_library::FabricName;
use tensegrity_lab::units::Seconds;
use tensegrity_lab::SnapshotMoment;
use tensegrity_lab::{LabEvent, RunStyle};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    fabric: Option<FabricName>,

    #[arg(long)]
    bake_bricks: bool,

    /// Run evolutionary tensegrity
    #[arg(long)]
    evolve: bool,

    /// Evolution scenario name (default, aggressive, conservative, tall-towers)
    #[arg(long)]
    scenario: Option<String>,

    /// Generate an algorithmic tensegrity sphere with given frequency (1, 2, or 3+)
    #[arg(long)]
    sphere: Option<usize>,

    /// Radius of the sphere in internal units
    #[arg(long, default_value_t = 10.0)]
    radius: f32,

    /// Generate an algorithmic Möbius strip with given number of segments
    #[arg(long)]
    mobius: Option<usize>,

    /// Record animation for specified duration (seconds) from start of fabric construction
    #[arg(long)]
    record: Option<f32>,

    /// FPS for animation export (default 100)
    #[arg(long, default_value_t = 100.0)]
    fps: f64,

    /// Time scale multiplier (default 1.0, use higher values for faster simulation)
    #[arg(long, default_value_t = 1.0)]
    time_scale: f32,

    /// Export CSV snapshot at specified moment (slack, pretenst, settled, or all)
    #[arg(long)]
    snapshot: Option<SnapshotMoment>,

    /// Display dimensions at model scale (e.g., 18.5 for 18.5:1 scale)
    /// Only affects displayed measurements, not simulation
    #[arg(long)]
    model_scale: Option<f32>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let record_duration = args.record.map(Seconds);

    let run_style = if let Some(frequency) = args.sphere {
        RunStyle::Sphere {
            frequency,
            radius: args.radius,
        }
    } else if let Some(segments) = args.mobius {
        RunStyle::Mobius { segments }
    } else if args.bake_bricks {
        RunStyle::BakeBricks
    } else if args.evolve || args.scenario.is_some() {
        RunStyle::Evolving {
            scenario_name: args.scenario,
        }
    } else if let Some(fabric_name) = args.fabric {
        RunStyle::Fabric {
            fabric_name,
            record: record_duration,
            export_fps: args.fps,
            snapshot: args.snapshot,
        }
    } else {
        // Default: Triped with model-scale 18
        RunStyle::Fabric {
            fabric_name: FabricName::Triped,
            record: None,
            export_fps: 100.0,
            snapshot: None,
        }
    };

    // Use model_scale from args, or default to 18 if running with default fabric
    let model_scale = args.model_scale.or_else(|| {
        if matches!(run_style, RunStyle::Fabric { fabric_name: FabricName::Triped, .. }) && args.fabric.is_none() {
            Some(18.0)
        } else {
            None
        }
    });

    run_with(run_style, args.time_scale, model_scale)
}

fn run_with(run_style: RunStyle, time_scale: f32, model_scale: Option<f32>) -> Result<(), Box<dyn Error>> {
    let mut builder = EventLoop::<LabEvent>::with_user_event();
    let event_loop: EventLoop<LabEvent> = builder.build()?;
    let radio = event_loop.create_proxy();

    #[cfg(not(target_arch = "wasm32"))]
    let window_attributes = create_window_attributes();
    #[cfg(target_arch = "wasm32")]
    let window_attributes = create_window_attributes();
    let mut application = Application::new(window_attributes, radio.clone(), time_scale, model_scale);
    LabEvent::Run(run_style).send(&radio);
    event_loop.run_app(&mut application)?;
    Ok(())
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(start))]
pub fn run() {
    run_with(
        RunStyle::Fabric {
            fabric_name: FabricName::Triped,
            record: None,
            export_fps: 100.0,
            snapshot: None,
        },
        1.0,
        None, // No model scale for WASM
    )
    .unwrap();
}

#[cfg(not(target_arch = "wasm32"))]
fn create_window_attributes() -> WindowAttributes {
    WindowAttributes::default()
        .with_title("Tensegrity Lab")
        .with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))
}

#[cfg(target_arch = "wasm32")]
fn create_window_attributes() -> WindowAttributes {
    use wasm_bindgen::JsCast;
    use winit::dpi::PhysicalSize;
    use winit::platform::web::WindowAttributesExtWebSys;

    let web_sys_window = web_sys::window().expect("no web sys window");
    let document = web_sys_window.document().expect("no document");
    let ratio = web_sys_window.device_pixel_ratio();
    let width = web_sys_window.inner_width().unwrap().as_f64().unwrap();
    let height = web_sys_window.inner_height().unwrap().as_f64().unwrap();
    let size = PhysicalSize::new(width * ratio, height * ratio);
    let canvas = document
        .get_element_by_id("canvas")
        .expect("no element with id 'canvas'")
        .dyn_into()
        .expect("not a canvas");
    WindowAttributes::default()
        .with_canvas(Some(canvas))
        .with_inner_size(size)
}
