use std::sync::mpsc::channel;

use clap::Parser;
#[allow(unused_imports)]
use leptos::{create_signal, view, WriteSignal};
use leptos::create_memo;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use winit::{event_loop::EventLoop, window::WindowBuilder};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit_input_helper::WinitInputHelper;

use tensegrity_lab::application::Application;
use tensegrity_lab::build::tenscript::fabric_library::FabricLibrary;
use tensegrity_lab::graphics::Graphics;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    fabric: Option<String>,

    #[arg(long)]
    prototype: Option<usize>,
}

fn main() {
    let Args { fabric, prototype } = Args::parse();
    if fabric.is_some() {
        run_with(fabric, None);
        return;
    }
    if prototype.is_some() {
        run_with(None, prototype);
        return;
    }
    println!("Give me --fabric <fabric name> or --prototype N");
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run() {
    run_with(None, None);
}

pub fn run_with(fabric_name: Option<String>, prototype: Option<usize>) {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Info).expect("Couldn't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let event_loop = EventLoop::new().unwrap();
    #[allow(unused_mut)]
        let mut window_builder = WindowBuilder::new()
        .with_title("Tensegrity Lab")
        .with_inner_size(PhysicalSize::new(1600, 1200));
    #[allow(unused_variables)]
        let (actions_tx, actions_rx) = channel();

    #[allow(unused_variables)]
        let (control_state, set_control_state) = create_signal(Default::default());
    #[allow(unused_variables)]
        let fabric_list = create_memo(move |_bla| FabricLibrary::from_source().unwrap().fabric_list().unwrap());
    #[cfg(target_arch = "wasm32")]
    {
        use tensegrity_lab::control_overlay::overlay::ControlOverlayApp;
        use winit::platform::web::WindowBuilderExtWebSys;

        let actions_tx = actions_tx.clone();

        let web_sys_window = web_sys::window().expect("no web sys window");
        let document = web_sys_window.document().expect("no document");

        let control_overlay = document
            .get_element_by_id("control_overlay")
            .expect("no control overlay")
            .dyn_into()
            .expect("no html element");
        leptos::mount_to(control_overlay, move || {
            view! {
                <ControlOverlayApp
                    fabric_list={fabric_list}
                    control_state={control_state}
                    set_control_state={set_control_state}
                    actions_tx={actions_tx}/>
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
        window_builder = window_builder
            .with_canvas(Some(canvas))
            .with_inner_size(size);
    }

    let winit_window = window_builder
        .build(&event_loop)
        .expect("Could not build window");
    
    let graphics = pollster::block_on(Graphics::new(&winit_window, 100, 100));
    let mut app = 
        match Application::new(graphics, (control_state, set_control_state), (actions_tx, actions_rx)) {
            Ok(app) => app,
            Err(error) => panic!("Tenscript Error: [{:?}]", error)
        };
    if let Some(fabric_name) = fabric_name {
        app.run_fabric(&fabric_name);
    }
    if let Some(prototype) = prototype {
        app.capture_prototype(prototype);
    }
    let mut input = WinitInputHelper::new();
    event_loop
        .run(move |event, window_target| {
            match &event {
                Event::WindowEvent { event: WindowEvent::RedrawRequested, .. } => app.redraw(),
                _ => winit_window.request_redraw(),
            };
            if input.update(&event) {
                if input.close_requested() {
                    window_target.exit();
                    return;
                }
                app.handle_input(&input);
                app.handle_actions();
            }
        })
        .unwrap();
}

