use std::error::Error;

use clap::Parser;
use winit::event_loop::EventLoop;
use winit::window::WindowAttributes;

use crate::RunStyle::{FabricName, Online, Prototype, Seeded};
use tensegrity_lab::application::Application;
use tensegrity_lab::messages::LabEvent;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    fabric: Option<String>,

    #[arg(long)]
    prototype: Option<usize>,

    #[arg(long)]
    seed: Option<u64>,
}

enum RunStyle {
    FabricName(String),
    Prototype(usize),
    Seeded(u64),
    Online,
}

fn main() -> Result<(), Box<dyn Error>> {
    let Args {
        fabric,
        prototype,
        seed,
    } = Args::parse();
    let run_style = match (fabric, prototype, seed) {
        (Some(name), None, None) => FabricName(name),
        (None, Some(prototype), None) => Prototype(prototype),
        (None, None, Some(seed)) => Seeded(seed),
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
    match run_style {
        FabricName(name) => {
            event_loop_proxy
                .send_event(LabEvent::LoadFabric(name))
                .unwrap();
        }
        Prototype(number) => {
            event_loop_proxy
                .send_event(LabEvent::CapturePrototype(number))
                .unwrap();
        }
        Seeded(seed) => {
            event_loop_proxy
                .send_event(LabEvent::EvolveFromSeed(seed))
                .unwrap();
        }
        Online => {}
    }
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(start))]
pub fn run() {
    run_with(FabricName("De Twips".to_string())).unwrap();
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
