use std::error::Error;

use clap::Parser;
#[allow(unused_imports)]
#[cfg(target_arch = "wasm32")]
use leptos::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;
use winit::window::WindowAttributes;

use crate::RunStyle::{FabricName, Online, Prototype, Seeded};
use tensegrity_lab::application::{AppChange, Application};
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

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run() {
    run_with(Online).unwrap();
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

    #[allow(unused_mut)]
    let mut window_attributes = WindowAttributes::default()
        .with_title("Tensegrity Lab")
        .with_inner_size(PhysicalSize::new(1600, 1200));

    #[cfg(target_arch = "wasm32")]
    use tensegrity_lab::messages::ControlState;
    #[cfg(target_arch = "wasm32")]
    #[allow(unused_variables)]
    let (control_state, set_control_state) = create_signal(ControlState::default());
    #[cfg(target_arch = "wasm32")]
    #[allow(unused_variables)]
    let (lab_control, set_lab_control) = create_signal(false);
    #[cfg(target_arch = "wasm32")]
    use tensegrity_lab::fabric::FabricStats;
    #[cfg(target_arch = "wasm32")]
    #[allow(unused_variables)]
    let (fabric_stats, set_fabric_stats) = create_signal::<Option<FabricStats>>(None);

    #[cfg(target_arch = "wasm32")]
    {
        use tensegrity_lab::build::tenscript::fabric_library::FabricLibrary;
        use tensegrity_lab::control_overlay::ControlOverlayApp;
        use winit::platform::web::WindowAttributesExtWebSys;

        let web_sys_window = web_sys::window().expect("no web sys window");
        let document = web_sys_window.document().expect("no document");
        let overlay_proxy = event_loop.create_proxy();

        let control_overlay = document
            .get_element_by_id("control_overlay")
            .expect("no control overlay")
            .dyn_into()
            .expect("no html element");
        mount_to(control_overlay, move || {
            view! {
                <ControlOverlayApp
                    fabric_list={FabricLibrary::from_source().unwrap().fabric_list()}
                    control_state={control_state}
                    lab_control=lab_control
                    fabric_stats=fabric_stats
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

    #[cfg(not(target_arch = "wasm32"))]
    fn app_change(app_change: AppChange) {
        match app_change {
            AppChange::SetControlState(_) => {}
            AppChange::SetLabControl(_) => {}
            AppChange::SetFabricStats(_) => {}
        }
    }
    #[cfg(target_arch = "wasm32")]
    fn app_change(app_change: AppChange) {
        match app_change {
            AppChange::SetControlState(_control_state) => {
                // set_control_state.set(_control_state);
            }
            AppChange::SetLabControl(_lab_control) => {
                // set_lab_control.set(lab_control);
            }
            AppChange::SetFabricStats(_fabric_stats) => {
                // set_fabric_stats.set(fabric_stats);
            }
        }
    }

    let mut app: Application = match Application::new(window_attributes, app_change, proxy) {
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
