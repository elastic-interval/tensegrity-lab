use clap::Parser;
#[allow(unused_imports)]
use log::info;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use winit::{event_loop::EventLoop, window::WindowBuilder};
use winit::dpi::PhysicalSize;
use winit_input_helper::WinitInputHelper;

use tensegrity_lab::application::Application;
use tensegrity_lab::graphics::Graphics;

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
    let winit_window = WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(1600, 1200))
        .build(&event_loop)
        .expect("Could not build window");

    winit_window.set_title("Tensegrity Lab");
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|web_sys_window| {
                let [width, height] = [web_sys_window.inner_width(), web_sys_window.inner_height()]
                    .map(|x| x.unwrap().as_f64().unwrap() * 2.0);
                info!("window swag {width} {height}");
                winit_window.set_min_inner_size(Some(PhysicalSize::new(width, height)));
                let doc = web_sys_window.document()?;
                let dst = doc.get_element_by_id("body")?;
                let wgpu_canvas = winit_window.canvas()?;
                let html_canvas = web_sys::Element::from(wgpu_canvas);
                html_canvas.set_id("canvas");
                dst.append_child(&html_canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }
    let graphics = pollster::block_on(Graphics::new(&winit_window));
    let mut app = Application::new(graphics);
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
