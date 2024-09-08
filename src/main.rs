use std::error::Error;
use std::sync::Arc;

use clap::Parser;
#[allow(unused_imports)]
use leptos::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;
use winit::window::WindowAttributes;

use tensegrity_lab::application::Application;
use tensegrity_lab::messages::{ControlState, LabEvent};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    fabric: Option<String>,

    #[arg(long)]
    prototype: Option<usize>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let Args { fabric, prototype } = Args::parse();
    if fabric.is_some() {
        return run_with(fabric, None);
    }
    if prototype.is_some() {
        return run_with(None, prototype);
    }
    println!("Give me --fabric <fabric name> or --prototype N");
    Ok(())
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run() {
    run_with(None, None).unwrap();
}

pub fn run_with(fabric_name: Option<String>, prototype: Option<usize>) -> Result<(), Box<dyn Error>> {
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
    let event_loop_proxy = Arc::new(event_loop.create_proxy());

    #[allow(unused_mut)]
    let mut window_attributes = WindowAttributes::default()
        .with_title("Tensegrity Lab")
        .with_inner_size(PhysicalSize::new(1600, 1200));

    #[allow(unused_variables)]
    let (control_state, set_control_state) = create_signal(ControlState::default());

    #[cfg(target_arch = "wasm32")]
    {
        use tensegrity_lab::control_overlay::ControlOverlayApp;
        use winit::platform::web::WindowAttributesExtWebSys;
        use tensegrity_lab::build::tenscript::fabric_library::FabricLibrary;

        let web_sys_window = web_sys::window().expect("no web sys window");
        let document = web_sys_window.document().expect("no document");
        let overlay_proxy = event_loop.create_proxy();

        let control_overlay = document
            .get_element_by_id("control_overlay")
            .expect("no control overlay")
            .dyn_into()
            .expect("no html element");
        leptos::mount_to(control_overlay, move || {
            view! {
                <ControlOverlayApp
                    fabric_list={FabricLibrary::from_source().unwrap().fabric_list()}
                    control_state={control_state}
                    event_loop_proxy={overlay_proxy}/>
            }
        });

        let canvas = document
            .get_element_by_id("canvas")
            .expect("no element with id 'canvas'")
            .dyn_into()
            .expect("not a canvas");
        let ratio = web_sys_window.device_pixel_ratio();
        let width = web_sys_window.inner_width().unwrap().as_f64().unwrap();
        let height = web_sys_window.inner_height().unwrap().as_f64().unwrap();
        let size = PhysicalSize::new(width * ratio, height * ratio);
        window_attributes = window_attributes
            .with_canvas(Some(canvas))
            .with_inner_size(size);
    }

    let proxy = event_loop_proxy.clone();
    let mut app =
        match Application::new(window_attributes, set_control_state, proxy) {
            Ok(app) => app,
            Err(error) => panic!("Tenscript Error: [{:?}]", error)
        };
    if let Some(fabric_name) = fabric_name {
        event_loop_proxy.send_event(LabEvent::LoadFabric(fabric_name)).unwrap();
    }
    if let Some(prototype) = prototype {
        event_loop_proxy.send_event(LabEvent::CapturePrototype(prototype)).unwrap();
    }
    event_loop.run_app(&mut app)?;
    Ok(())
}

