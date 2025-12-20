#[allow(unused_imports)]
use getrandom;
use std::error::Error;

use clap::Parser;
use winit::event_loop::EventLoop;
use winit::window::WindowAttributes;

use tensegrity_lab::application::Application;
use tensegrity_lab::SnapshotMoment;
use tensegrity_lab::units::Seconds;
use tensegrity_lab::{LabEvent, RunStyle, TestScenario};
use tensegrity_lab::build::dsl::fabric_library::FabricName;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    fabric: Option<FabricName>,

    #[arg(long)]
    bake_bricks: bool,

    #[arg(long)]
    seed: Option<u64>,

    #[arg(long)]
    test: Option<String>,

    #[arg(long)]
    machine: Option<String>,

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
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let record_duration = args.record.map(Seconds);

    let run_style = if let Some(frequency) = args.sphere {
        RunStyle::Sphere { frequency, radius: args.radius }
    } else if let Some(segments) = args.mobius {
        RunStyle::Mobius { segments }
    } else if args.bake_bricks {
        RunStyle::BakeBricks
    } else if let Some(seed) = args.seed {
        RunStyle::Seeded(seed)
    } else if let Some(fabric_name) = args.fabric {
        let scenario = match (args.test.as_deref(), args.machine) {
            (Some("physics"), None) => Some(TestScenario::PhysicsTest),
            (Some(test), None) => panic!("unknown test: \"{test}\""),
            (None, Some(ip)) => Some(TestScenario::MachineTest(ip)),
            (None, None) => None,
            _ => return Err("cannot combine --test and --machine".into()),
        };
        RunStyle::Fabric {
            fabric_name,
            scenario,
            record: record_duration,
            export_fps: args.fps,
            snapshot: args.snapshot,
        }
    } else {
        return Err("use --fabric <name> or --bake-bricks or --seed <seed> or --sphere <freq> or --mobius <segments>".into());
    };

    run_with(run_style, args.time_scale)
}

fn run_with(run_style: RunStyle, time_scale: f32) -> Result<(), Box<dyn Error>> {
    let mut builder = EventLoop::<LabEvent>::with_user_event();
    let event_loop: EventLoop<LabEvent> = builder.build()?;
    let radio = event_loop.create_proxy();

    #[cfg(not(target_arch = "wasm32"))]
    let window_attributes = create_window_attributes();
    #[cfg(target_arch = "wasm32")]
    let window_attributes = create_window_attributes();
    let mut application = Application::new(window_attributes, radio.clone(), time_scale);
    LabEvent::Run(run_style).send(&radio);
    event_loop.run_app(&mut application)?;
    Ok(())
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(start))]
pub fn run() {
    run_with(RunStyle::Fabric {
        fabric_name: FabricName::Triped,
        scenario: None,
        record: None,
        export_fps: 100.0,
        snapshot: None,
    }, 1.0)
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
