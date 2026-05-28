#[allow(unused_imports)]
use getrandom;
use std::error::Error;

use clap::Parser;
use strum::IntoEnumIterator;
use winit::event_loop::EventLoop;
use winit::window::WindowAttributes;

use tensegrity_lab::application::Application;
use tensegrity_lab::build::dsl::fabric_library::FabricName;
use tensegrity_lab::units::Seconds;
use tensegrity_lab::{LabEvent, RunStyle};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    fabric: Option<FabricName>,

    #[arg(long)]
    bake_bricks: bool,

    #[arg(long)]
    evolve: Option<u64>,

    /// Generate an algorithmic tensegrity sphere with given frequency (1, 2, or 3+)
    #[arg(long)]
    sphere: Option<usize>,

    /// Radius of the sphere in internal units
    #[arg(long, default_value_t = 10.0)]
    radius: f32,

    /// Generate an algorithmic Möbius strip with given number of segments
    #[arg(long)]
    mobius: Option<usize>,

    /// Generate an algorithmic Klein bottle tensegrity
    #[arg(long)]
    klein: bool,

    /// Record animation for specified duration (seconds) from start of fabric construction
    #[arg(long)]
    record: Option<f32>,

    /// FPS for animation export (default 100)
    #[arg(long, default_value_t = 100.0)]
    fps: f64,

    /// Time scale multiplier (default 1.0, use higher values for faster simulation)
    #[arg(long, default_value_t = 1.0)]
    time_scale: f32,

    /// Display dimensions at model scale (e.g., 18.5 for 18.5:1 scale)
    /// Only affects displayed measurements, not simulation
    #[arg(long)]
    model_scale: Option<f32>,

    /// Cycle through every named fabric, pausing briefly after each one
    /// finishes. Loops forever; Ctrl+C to exit. Overrides `--fabric`
    /// (selects the first fabric for you).
    #[arg(long)]
    cycle: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let record_duration = args.record.map(Seconds);

    let cycle_names: Option<Vec<FabricName>> = args
        .cycle
        .then(|| FabricName::iter().collect());

    let run_style = if let Some(frequency) = args.sphere {
        RunStyle::Sphere {
            frequency,
            radius: args.radius,
        }
    } else if let Some(segments) = args.mobius {
        RunStyle::Mobius { segments }
    } else if args.klein {
        RunStyle::Klein {
            width: 10,
            height: 31,
            shift: 0,
        }
    } else if args.bake_bricks {
        RunStyle::BakeBricks
    } else if let Some(seed) = args.evolve {
        RunStyle::Evolution(seed)
    } else if let Some(names) = &cycle_names {
        RunStyle::Fabric {
            fabric_name: names[0],
            record: None,
            export_fps: 100.0,
        }
    } else if let Some(fabric_name) = args.fabric {
        RunStyle::Fabric {
            fabric_name,
            record: record_duration,
            export_fps: args.fps,
        }
    } else {
        // Default: OpenClaw
        RunStyle::Fabric {
            fabric_name: FabricName::OpenClaw,
            record: None,
            export_fps: 100.0,
        }
    };

    let model_scale = args.model_scale;

    run_with(run_style, args.time_scale, model_scale, cycle_names)
}

fn run_with(
    run_style: RunStyle,
    time_scale: f32,
    model_scale: Option<f32>,
    cycle: Option<Vec<FabricName>>,
) -> Result<(), Box<dyn Error>> {
    let mut builder = EventLoop::<LabEvent>::with_user_event();
    let event_loop: EventLoop<LabEvent> = builder.build()?;
    let radio = event_loop.create_proxy();

    #[cfg(not(target_arch = "wasm32"))]
    let window_attributes = create_window_attributes();
    #[cfg(target_arch = "wasm32")]
    let window_attributes = create_window_attributes();
    let mut application =
        Application::new(window_attributes, radio.clone(), time_scale, model_scale);
    if let Some(names) = cycle {
        application.set_cycle(names);
    }
    LabEvent::Run(run_style).send(&radio);
    event_loop.run_app(&mut application)?;
    Ok(())
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(start))]
pub fn run() {
    // WASM always runs in Show mode — there's no CLI, so this is the
    // standard public experience.
    let cycle: Option<Vec<FabricName>> = Some(FabricName::iter().collect());
    let initial_fabric = cycle
        .as_ref()
        .map(|names| names[0])
        .unwrap_or(FabricName::OpenClaw);

    run_with(
        RunStyle::Fabric {
            fabric_name: initial_fabric,
            record: None,
            export_fps: 100.0,
        },
        1.0,
        None, // No model scale for WASM
        cycle,
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
