use clap::Parser;
#[allow(unused_imports)]
use log::info;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit::dpi::PhysicalSize;

use tensegrity_lab::application::Application;
use tensegrity_lab::graphics::GraphicsWindow;
use tensegrity_lab::post_iterate::InsideOutDonut;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the prototype to settle and capture
    #[arg(long)]
    prototype: Option<usize>,
    #[arg(long)]
    experiment: Option<usize>,
}

fn main() {
    let Args { prototype, experiment } = Args::parse();
    run(prototype, experiment);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run(prototype: Option<usize>, experiment: Option<usize>) {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Info).expect("Couldn't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(3000, 1600))
        .build(&event_loop)
        .expect("Could not build window");

    window.set_title("Tensegrity Lab");
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| {
                let [width, height] = [win.inner_width(), win.inner_height()]
                    .map(|x| x.unwrap().as_f64().unwrap() * 2.0);
                window.set_inner_size(PhysicalSize::new(width, height));
                let doc = win.document()?;
                let dst = doc.get_element_by_id("body")?;
                let canvas = web_sys::Element::from(window.canvas());
                canvas.set_id("canvas");
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }
    let graphics = pollster::block_on(GraphicsWindow::new(&window));
    let mut app = Application::new(graphics, &window);
    if let Some(brick_index) = prototype {
        app.capture_prototype(brick_index);
    } else if let Some(_experiment_number) = experiment {
        let fabric = "Ring".to_string();
        app.run_fabric(&fabric, Some(InsideOutDonut::default()))
    } else {
        let fabric = "Halo by Crane".to_string();
        app.run_fabric(&fabric, None)

    }
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent { ref event, window_id } if window_id == window.id() => match event {
                WindowEvent::CloseRequested { .. } => *control_flow = ControlFlow::Exit,
                event => app.handle_window_event(event, &window)
            },
            Event::RedrawRequested(_) => app.redraw(&window),
            Event::MainEventsCleared => app.update(&window),
            _ => {}
        }
    });
}
