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

    #[cfg(target_arch = "wasm32")]
    use tensegrity_lab::control_overlay::OverlayState;
    #[cfg(target_arch = "wasm32")]
    let overlay_state = OverlayState::default();

    #[cfg(target_arch = "wasm32")]
    {
        use tensegrity_lab::build::tenscript::fabric_library::FabricLibrary;
        use tensegrity_lab::control_overlay::ControlOverlayApp;
        use leptos::prelude::*;

        let overlay_proxy = event_loop.create_proxy();

        let _ = mount_to_body( move || {
            view! {
                <ControlOverlayApp
                    fabric_list={FabricLibrary::from_source().unwrap().fabric_list()}
                    control_state={overlay_state.control_state}
                    fabric_stats={overlay_state.fabric_stats}
                    fabric_name={overlay_state.fabric_name}
                    set_fabric_name={overlay_state.set_fabric_name}
                    event_loop_proxy={overlay_proxy}/>
            }
        });

        setup_resize_handler(event_loop_proxy.clone());
    }

    #[cfg(not(target_arch = "wasm32"))]
    let window_attributes = create_window_attributes();
    #[cfg(target_arch = "wasm32")]
    let window_attributes = create_window_attributes();

    let proxy = event_loop_proxy.clone();

    let mut app: Application = match Application::new(
        window_attributes,
        proxy,
        #[cfg(target_arch = "wasm32")] overlay_state,
    ) {
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
    run_with(Online).unwrap();
}

#[cfg(not(target_arch = "wasm32"))]
fn create_window_attributes() -> WindowAttributes {
    WindowAttributes::default()
        .with_title("Tensegrity Lab")
        .with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))
}

#[cfg(target_arch = "wasm32")]
fn create_window_attributes() -> WindowAttributes {
    use winit::platform::web::WindowAttributesExtWebSys;
    use winit::dpi::PhysicalSize;
    use wasm_bindgen::JsCast;

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

#[cfg(target_arch = "wasm32")]
fn setup_resize_handler(event_loop_proxy: winit::event_loop::EventLoopProxy<LabEvent>) {
    use winit::dpi::PhysicalSize;
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;

    let window = web_sys::window().expect("no global window exists");
    let resize_closure = Closure::wrap(Box::new(move || {
        let window = web_sys::window().expect("no global window exists");
        let ratio = window.device_pixel_ratio();
        let width = window.inner_width().unwrap().as_f64().unwrap();
        let height = window.inner_height().unwrap().as_f64().unwrap();
        let position = PhysicalSize::new((width * ratio) as u32, (height * ratio) as u32);
        event_loop_proxy.send_event(LabEvent::Resize(position)).unwrap();
    }) as Box<dyn FnMut()>);

    window
        .add_event_listener_with_callback("resize", resize_closure.as_ref().unchecked_ref())
        .expect("failed to add resize listener");

    // Forget the closure to keep it alive (otherwise it will be dropped)
    resize_closure.forget();
}