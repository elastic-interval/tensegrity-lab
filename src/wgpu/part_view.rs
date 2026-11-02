//! The parts view: `--part` on the command line, to look closely at a part's
//! design and at each change to it. Every part turns in a strip down the left;
//! clicking one shows it large. Assembled parts show whole, so a hinge has its
//! rod end. Nothing else in the application knows
//! about this view.

use crate::build::dsl::fabric_library::FabricName;
use crate::build::dsl::fabric_plan_executor::FabricPlanExecutor;
use crate::connector::parts::{part_ids, parts_list, placements, Part, Placement};
use crate::wgpu::connector_renderer::{part_instance_layout, PartInstance};
use crate::wgpu::part_meshes::{PartMesh, PartSizes};
use crate::wgpu::parts_panel::{panel_rows, PanelRow};
use crate::wgpu::{default_depth_stencil_state, Wgpu};
use glam::{Mat4, Quat, Vec3};
use std::error::Error;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu_text::glyph_brush::ab_glyph::FontRef;
use wgpu_text::glyph_brush::{HorizontalAlign, Layout, OwnedSection, OwnedText, VerticalAlign};
use wgpu_text::{BrushBuilder, TextBrush};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Fullscreen, Window, WindowAttributes, WindowId};

// The part turns until it is first dragged, in radians per second.
const TURN_RATE: f32 = 0.4;
const DRAG_RATE: f32 = 0.006;
const ZOOM_RATE: f32 = 0.1;
const STRIP_AZIMUTH: f32 = 0.6;
const STRIP_ELEVATION: f32 = 0.45;
// How much of its viewport's height a part's bounding sphere fills.
const FILL: f32 = 0.9;
// Room beside the strip's squares for each part's letter and count, in pixels.
const COUNT_WIDTH: f32 = 180.0;
const TITLE_MARGIN: f32 = 50.0;
const TITLE_SCALE: f32 = 44.0;
const COUNT_SCALE: f32 = 30.0;
const CHOSEN_TEXT: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const OTHER_TEXT: [f32; 4] = [0.45, 0.47, 0.5, 1.0];
const BACKGROUND: wgpu::Color = wgpu::Color { r: 0.05, g: 0.06, b: 0.08, a: 1.0 };
// A hinge in this view, its struts this far apart, as in the parts panel.
const HINGE_OPENING: f32 = 135.0;

/// Build `fabric_name`'s plan without a window and show its parts, beginning
/// with the one called `id`, or the first if `id` is empty. `list`, or an id
/// matching no single part, prints the ids.
pub fn run(fabric_name: FabricName, id: &str) -> Result<(), Box<dyn Error>> {
    let plan = crate::build::dsl::fabric_library::get_fabric_plan(fabric_name);
    let mut executor = FabricPlanExecutor::new(plan);
    while !executor.is_complete() {
        let _ = executor.iterate();
    }
    let mut fabric = executor.fabric;
    fabric.update_all_attachment_connections();
    let Some(connector) = fabric.connector.as_ref() else {
        return Err(format!("{fabric_name} has no connector hardware").into());
    };
    let ids = part_ids(&parts_list(&placements(&fabric)));
    let matching: Vec<usize> = match ids.iter().position(|(name, ..)| name == id) {
        Some(exact) => vec![exact],
        None if id.is_empty() => vec![0],
        None => (0..ids.len()).filter(|&index| ids[index].0.contains(id)).collect(),
    };
    let [chosen] = matching[..] else {
        println!("Parts of {fabric_name}, shown with --part <id>:");
        for (index, (name, part, count)) in ids.iter().enumerate() {
            println!("  {} {name:<16} {count:>4} × {}", letter(index), part.name());
        }
        return Ok(());
    };
    println!("Click a part to show it, drag to orbit, wheel to zoom, Esc to quit.");
    let mut view = PartView {
        parts: ids.into_iter().map(|(_, part, count)| (part, count)).collect(),
        chosen,
        sizes: PartSizes::of(&fabric.dimensions, &connector.dimensions),
        window: None,
        azimuth: 0.6,
        elevation: 0.45,
        zoom: 1.0,
        turning: true,
        dragging: false,
        pointer: None,
        started: instant::Instant::now(),
    };
    EventLoop::new()?.run_app(&mut view)?;
    Ok(())
}

/// What to call the part at `index` when talking about it.
fn letter(index: usize) -> char {
    (b'A' + index as u8) as char
}

/// The placements that make up `part` whole, in metres.
fn assembled(part: Part) -> Vec<Placement> {
    let place = |part, origin, x, y| Placement { part, origin, x, y, culprit: false };
    let alone = place(part, Vec3::ZERO, Vec3::X, Vec3::Y);
    match part {
        Part::Hinge => {
            let opening = HINGE_OPENING.to_radians();
            let eye = Quat::from_rotation_y(opening) * Vec3::X;
            vec![alone, place(Part::HingeEye, Vec3::ZERO, eye, Vec3::Y)]
        }
        _ => vec![alone],
    }
}

/// Looking at a sphere from `azimuth` and `elevation`, filling `fill` of the
/// height of a viewport `aspect` wide.
fn camera_matrix(center: Vec3, radius: f32, azimuth: f32, elevation: f32, aspect: f32, fill: f32) -> Mat4 {
    let half_height = radius / fill;
    let half_width = half_height * aspect;
    let distance = radius * 4.0;
    let toward = Vec3::new(azimuth.sin() * elevation.cos(), elevation.sin(), azimuth.cos() * elevation.cos());
    Mat4::orthographic_rh(-half_width, half_width, -half_height, half_height, 0.0, distance * 2.0)
        * Mat4::look_at_rh(center + toward * distance, center, Vec3::Y)
}

/// A uniform buffer for a camera matrix and the bind group reaching it.
fn create_camera(wgpu: &Wgpu) -> (wgpu::Buffer, wgpu::BindGroup) {
    let camera = wgpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Part View Camera"),
        contents: bytemuck::cast_slice(&Mat4::IDENTITY.to_cols_array()),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let bind_group = wgpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Part View Bind Group"),
        layout: &wgpu.uniform_bind_group_layout,
        entries: &[wgpu::BindGroupEntry { binding: 0, resource: camera.as_entire_binding() }],
    });
    (camera, bind_group)
}

/// One part ready to draw, with the camera of its square in the strip.
struct ShownPart {
    pieces: Vec<(PartMesh, wgpu::Buffer)>,
    center: Vec3,
    radius: f32,
    strip_camera: wgpu::Buffer,
    strip_bind_group: wgpu::BindGroup,
}

impl ShownPart {
    fn new(wgpu: &Wgpu, part: Part, sizes: PartSizes) -> Self {
        let (mut low, mut high) = (Vec3::MAX, Vec3::MIN);
        let mut pieces = Vec::new();
        for placement in assembled(part) {
            let mesh = wgpu.create_part(placement.part, sizes);
            let z = placement.x.cross(placement.y);
            let c = mesh.center;
            let center = placement.origin + placement.x * c.x + placement.y * c.y + z * c.z;
            low = low.min(center - Vec3::splat(mesh.radius));
            high = high.max(center + Vec3::splat(mesh.radius));
            let instance = wgpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Part View Instance"),
                contents: bytemuck::cast_slice(&[PartInstance::new(&placement)]),
                usage: wgpu::BufferUsages::VERTEX,
            });
            pieces.push((mesh, instance));
        }
        let (strip_camera, strip_bind_group) = create_camera(wgpu);
        let (center, radius) = ((low + high) / 2.0, (high - low).max_element() / 2.0);
        ShownPart { pieces, center, radius, strip_camera, strip_bind_group }
    }

    fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, bind_group: &'a wgpu::BindGroup) {
        pass.set_bind_group(0, bind_group, &[]);
        for (mesh, instance) in &self.pieces {
            pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            pass.set_vertex_buffer(1, instance.slice(..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
        }
    }
}

struct Showing {
    window: Arc<Window>,
    wgpu: Wgpu,
    pipeline: wgpu::RenderPipeline,
    camera: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    shown: Vec<ShownPart>,
    brush: TextBrush<FontRef<'static>>,
    sections: Vec<OwnedSection>,
    rows: Vec<PanelRow>,
    // Where the large view begins, to the right of the strip.
    large_left: f32,
}

struct PartView {
    parts: Vec<(Part, usize)>,
    chosen: usize,
    sizes: PartSizes,
    window: Option<Showing>,
    azimuth: f32,
    elevation: f32,
    zoom: f32,
    turning: bool,
    dragging: bool,
    pointer: Option<PhysicalPosition<f64>>,
    started: instant::Instant,
}

impl PartView {
    fn show(&self, window: Arc<Window>) -> Showing {
        let wgpu = futures::executor::block_on(Wgpu::new_async(window.clone()));
        let pipeline = wgpu.create_plate_pipeline("Part View Pipeline", part_instance_layout());
        let (camera, bind_group) = create_camera(&wgpu);
        let shown = self.parts.iter().map(|&(part, _)| ShownPart::new(&wgpu, part, self.sizes)).collect();
        let (width, height) = wgpu.size();
        let brush = BrushBuilder::using_font_bytes(include_bytes!("../../assets/WorkSans-Regular.ttf"))
            .unwrap()
            .with_depth_stencil(Some(default_depth_stencil_state()))
            .build(&wgpu.device, width as u32, height as u32, wgpu.surface_configuration.format);
        let mut showing = Showing {
            window,
            wgpu,
            pipeline,
            camera,
            bind_group,
            shown,
            brush,
            sections: Vec::new(),
            rows: Vec::new(),
            large_left: 0.0,
        };
        self.lay_out(&mut showing);
        showing
    }

    /// The strip's rows and all the text, for the window's size and the part
    /// chosen.
    fn lay_out(&self, showing: &mut Showing) {
        let (width, height) = showing.wgpu.size();
        showing.rows = panel_rows(width, height, self.parts.len());
        showing.large_left = showing.rows.first().map_or(0.0, |row| row.text_at[0] + COUNT_WIDTH);
        let left_aligned = Layout::default().v_align(VerticalAlign::Center).h_align(HorizontalAlign::Left);
        let (chosen_part, chosen_count) = self.parts[self.chosen];
        let title = OwnedSection::default()
            .with_layout(left_aligned)
            .with_bounds([width - showing.large_left, height])
            .with_screen_position([showing.large_left, TITLE_MARGIN])
            .add_text(
                OwnedText::new(format!("{}   {chosen_count} × {}", letter(self.chosen), chosen_part.name()))
                    .with_color(CHOSEN_TEXT)
                    .with_scale(TITLE_SCALE),
            );
        let counts = self.parts.iter().zip(&showing.rows).enumerate().map(|(index, (&(_, count), row))| {
            OwnedSection::default()
                .with_layout(left_aligned)
                .with_bounds([COUNT_WIDTH, height])
                .with_screen_position(row.text_at)
                .add_text(
                    OwnedText::new(format!("{}   {count} ×", letter(index)))
                        .with_color(if index == self.chosen { CHOSEN_TEXT } else { OTHER_TEXT })
                        .with_scale(COUNT_SCALE),
                )
        });
        showing.sections = std::iter::once(title).chain(counts).collect();
    }

    /// The part whose row in the strip is at `position`.
    fn row_at(&self, position: PhysicalPosition<f64>) -> Option<usize> {
        let showing = self.window.as_ref()?;
        let (x, y) = (position.x as f32, position.y as f32);
        showing.rows.iter().position(|row| {
            let [left, top, side] = row.thumbnail;
            (left..showing.large_left).contains(&x) && (top..top + side).contains(&y)
        })
    }

    fn redraw(&mut self) {
        let elapsed = self.started.elapsed().as_secs_f32();
        let azimuth = if self.turning { self.azimuth + elapsed * TURN_RATE } else { self.azimuth };
        let Some(showing) = self.window.as_mut() else {
            return;
        };
        let (width, height) = showing.wgpu.size();
        let large = &showing.shown[self.chosen];
        let aspect = (width - showing.large_left) / height;
        let matrix = camera_matrix(large.center, large.radius, azimuth, self.elevation, aspect, FILL * self.zoom);
        showing.wgpu.queue.write_buffer(&showing.camera, 0, bytemuck::cast_slice(&matrix.to_cols_array()));
        for part in &showing.shown {
            let strip_azimuth = STRIP_AZIMUTH + elapsed * TURN_RATE;
            let matrix = camera_matrix(part.center, part.radius, strip_azimuth, STRIP_ELEVATION, 1.0, FILL);
            showing.wgpu.queue.write_buffer(&part.strip_camera, 0, bytemuck::cast_slice(&matrix.to_cols_array()));
        }
        let sections: Vec<&OwnedSection> = showing.sections.iter().collect();
        showing.brush.queue(&showing.wgpu.device, &showing.wgpu.queue, sections).unwrap();
        let Ok(surface_texture) = showing.wgpu.get_surface_texture() else {
            return;
        };
        let target = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth = showing.wgpu.create_depth_view();
        let mut encoder = showing.wgpu.create_encoder();
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Part View Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(BACKGROUND), store: wgpu::StoreOp::Store },
                    depth_slice: None,
                })],
                multiview_mask: None,
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&showing.pipeline);
            pass.set_viewport(showing.large_left, 0.0, width - showing.large_left, height, 0.0, 1.0);
            large.draw(&mut pass, &showing.bind_group);
            for (part, row) in showing.shown.iter().zip(&showing.rows) {
                let [left, top, side] = row.thumbnail;
                pass.set_viewport(left, top, side, side, 0.0, 1.0);
                part.draw(&mut pass, &part.strip_bind_group);
            }
            pass.set_viewport(0.0, 0.0, width, height, 0.0, 1.0);
            showing.brush.draw(&mut pass);
        }
        showing.wgpu.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
        showing.window.request_redraw();
    }

    /// Show the part in `row` large, or drag if the press missed the strip.
    fn press(&mut self, row: Option<usize>) {
        let Some(row) = row else {
            self.dragging = true;
            if self.turning {
                self.azimuth += self.started.elapsed().as_secs_f32() * TURN_RATE;
                self.turning = false;
            }
            return;
        };
        self.chosen = row;
        self.zoom = 1.0;
        if let Some(mut showing) = self.window.take() {
            self.lay_out(&mut showing);
            self.window = Some(showing);
        }
    }
}

impl ApplicationHandler for PartView {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attributes = WindowAttributes::default()
            .with_title("Tensegrity Lab parts")
            .with_fullscreen(Some(Fullscreen::Borderless(None)));
        let window = Arc::new(event_loop.create_window(attributes).expect("create window"));
        self.window = Some(self.show(window));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } if event.state.is_pressed() => {
                if event.logical_key == Key::Named(NamedKey::Escape) {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(mut showing) = self.window.take() {
                    showing.wgpu.resize((size.width, size.height));
                    showing.brush.resize_view(size.width as f32, size.height as f32, &showing.wgpu.queue);
                    self.lay_out(&mut showing);
                    self.window = Some(showing);
                }
            }
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                if state == ElementState::Pressed {
                    self.press(self.pointer.and_then(|pointer| self.row_at(pointer)));
                } else {
                    self.dragging = false;
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let (true, Some(last)) = (self.dragging, self.pointer) {
                    self.azimuth -= (position.x - last.x) as f32 * DRAG_RATE;
                    self.elevation = (self.elevation + (position.y - last.y) as f32 * DRAG_RATE).clamp(-1.5, 1.5);
                }
                self.pointer = Some(position);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let notches = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(position) => position.y as f32 / 50.0,
                };
                self.zoom = (self.zoom * (1.0 + notches * ZOOM_RATE)).clamp(0.2, 20.0);
            }
            _ => {}
        }
    }
}
