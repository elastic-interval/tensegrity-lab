use clap::Parser;
#[allow(unused_imports)]
use leptos::{create_signal, view, WriteSignal};
use std::sync::mpsc::channel;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use winit::dpi::PhysicalSize;
use winit::{event_loop::EventLoop, window::WindowBuilder};
use winit_input_helper::WinitInputHelper;

use tensegrity_lab::application::Application;
use tensegrity_lab::control_overlay;
use tensegrity_lab::graphics::Graphics;
use tensegrity_lab::user_interface::Action;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the prototype to settle and capture
    #[arg(long)]
    prototype: Option<usize>,
}

fn main() {
    run();
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run() {
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
    let (actions_tx, actions_rx) = channel::<Action>();

    let set_control_state: Option<WriteSignal<control_overlay::ControlState>>;
    #[cfg(target_arch = "wasm32")]
    {
        use tensegrity_lab::control_overlay::app::ControlOverlayApp;
        use winit::platform::web::WindowBuilderExtWebSys;

        let web_sys_window = web_sys::window().expect("no web sys window");
        let document = web_sys_window.document().expect("no document");

        let (control_state, set_control_state_signal) = create_signal(Default::default());
        set_control_state = Some(set_control_state_signal);

        let control_overlay = document
            .get_element_by_id("control_overlay")
            .expect("no control overlay")
            .dyn_into()
            .expect("no html element");
        leptos::mount_to(control_overlay, move || {
            view! {
                <ControlOverlayApp
                    control_state={control_state}
                    actions_tx={actions_tx}/>
            }
        });

        let canvas = document
            .get_element_by_id("canvas")
            .expect("no element with id 'canvas'")
            .dyn_into()
            .expect("not a canvas");
        let width = web_sys_window.inner_width().unwrap().as_f64().unwrap() * 2.0;
        let height = web_sys_window.inner_height().unwrap().as_f64().unwrap() * 2.0;
        window_builder = window_builder
            .with_canvas(Some(canvas))
            .with_inner_size(PhysicalSize::new(width, height));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        set_control_state = None;
    }

    let winit_window = window_builder
        .build(&event_loop)
        .expect("Could not build window");

    let graphics = pollster::block_on(Graphics::new(&winit_window));
    let mut app = Application::new(graphics, set_control_state, actions_rx);
    let mut input = WinitInputHelper::new();
    if let Some(brick_index) = None {
        app.capture_prototype(brick_index);
    } else {
        let fabric = "Tommy Torque".to_string();
        app.run_fabric(&fabric)
    }
    event_loop
        .run(move |event, window_target| {
            if input.update(&event) {
                if input.close_requested() {
                    window_target.exit();
                    return;
                }
                app.handle_input(&input);
                app.update();
                app.redraw();
            } else {
                winit_window.request_redraw();
            }
        })
        .unwrap();
}
