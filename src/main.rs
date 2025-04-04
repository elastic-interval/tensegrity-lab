#[allow(unused_imports)]
use getrandom;
use std::error::Error;

use clap::Parser;
use winit::event_loop::EventLoop;
use winit::window::WindowAttributes;

use tensegrity_lab::application::Application;
use tensegrity_lab::messages::RunStyle;
use tensegrity_lab::messages::{LabEvent, TestScenario};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    fabric: Option<String>,

    #[arg(long)]
    prototype: Option<usize>,

    #[arg(long)]
    seed: Option<u64>,

    #[arg(long)]
    test: Option<bool>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let Args {
        fabric,
        prototype,
        seed,
        test,
    } = Args::parse();
    let run_style = match (fabric, prototype, seed, test) {
        (Some(fabric_name), None, None, None) => RunStyle::Fabric {
            fabric_name,
            scenario: None,
        },
        (None, Some(prototype), None, None) => RunStyle::Prototype(prototype),
        (None, None, Some(seed), None) => RunStyle::Seeded(seed),
        (Some(fabric_name), None, None, Some(tension)) => RunStyle::Fabric {
            fabric_name,
            scenario: if tension {
                Some(TestScenario::TensionTest)
            } else {
                Some(TestScenario::CompressionTest)
            },
        },
        _ => {
            return Err("use --fabric <name> or --prototype <number> or --seed <seed>".into());
        }
    };
    run_with(run_style)
}

fn run_with(run_style: RunStyle) -> Result<(), Box<dyn Error>> {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Info).expect("Couldn't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let mut builder = EventLoop::<LabEvent>::with_user_event();
    let event_loop: EventLoop<LabEvent> = builder.build()?;
    let event_loop_proxy = event_loop.create_proxy();

    #[cfg(not(target_arch = "wasm32"))]
    let window_attributes = create_window_attributes();
    #[cfg(target_arch = "wasm32")]
    let window_attributes = create_window_attributes();
    let proxy = event_loop_proxy.clone();
    let mut app: Application = match Application::new(window_attributes, proxy) {
        Ok(app) => app,
        Err(error) => panic!("Tenscript Error: [{:?}]", error),
    };
    event_loop_proxy.send_event(LabEvent::Run(run_style))?;
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(start))]
pub fn run() {
    run_with(RunStyle::Fabric {
        fabric_name: "De Twips".to_string(),
        scenario: None,
    })
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
