//! The parts view, on the P key: every part of the fabric turning in a strip down
//! the left, and the one clicked on large beside them, to look closely at its
//! design. Assembled parts show whole, so a hinge has its rod end. It takes over
//! the window, its own camera and text, while the fabric runs on unseen.

use crate::connector::parts::{parts_list, placements, Part, Placement};
use crate::fabric::Fabric;
use crate::wgpu::connector_renderer::{part_instance_layout, PartInstance};
use crate::wgpu::part_meshes::{PartMesh, PartSizes};
use crate::wgpu::{default_depth_stencil_state, Wgpu};
use crate::units::{Grams, Unit};
use crate::PointerChange;
use glam::{Mat4, Quat, Vec3};
use instant::Instant;
use wgpu::util::DeviceExt;
use wgpu_text::glyph_brush::ab_glyph::FontRef;
use wgpu_text::glyph_brush::{HorizontalAlign, Layout, OwnedSection, OwnedText, VerticalAlign};
use wgpu_text::{BrushBuilder, TextBrush};
use winit::dpi::PhysicalPosition;

// The part turns until it is first dragged, in radians per second.
const TURN_RATE: f32 = 0.4;
const DRAG_RATE: f32 = 0.006;
pub(super) const AZIMUTH: f32 = 0.6;
pub(super) const ELEVATION: f32 = 0.45;
// How much of its viewport's height a part's bounding sphere fills.
pub(super) const FILL: f32 = 0.9;
// Gap from the window's edge to the strip and the text.
const MARGIN: f32 = 50.0;
// Largest square a part gets in the strip, in pixels.
const MAX_SQUARE: f32 = 110.0;
// Space between a square and its text, and between rows.
const GAP: f32 = 16.0;
// Room beside the strip's squares for each part's letter and count, in pixels.
const COUNT_WIDTH: f32 = 180.0;
const TITLE_SCALE: f32 = 44.0;
const COUNT_SCALE: f32 = 30.0;
const CHOSEN_TEXT: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const OTHER_TEXT: [f32; 4] = [0.45, 0.47, 0.5, 1.0];
const BACKGROUND: wgpu::Color = wgpu::Color { r: 0.05, g: 0.06, b: 0.08, a: 1.0 };
// A hinge in this view, its struts this far apart.
const HINGE_OPENING: f32 = 135.0;

/// A part's mass, in grams below a kilogram.
fn weight(grams: f32) -> String {
    if grams < 1000.0 {
        format!("{grams:.0} g")
    } else {
        format!("{:.2} kg", grams / 1000.0)
    }
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
pub(super) fn camera_matrix(center: Vec3, radius: f32, azimuth: f32, elevation: f32, aspect: f32, fill: f32) -> Mat4 {
    let half_height = radius / fill;
    let half_width = half_height * aspect;
    let distance = radius * 4.0;
    let toward = Vec3::new(azimuth.sin() * elevation.cos(), elevation.sin(), azimuth.cos() * elevation.cos());
    Mat4::orthographic_rh(-half_width, half_width, -half_height, half_height, 0.0, distance * 2.0)
        * Mat4::look_at_rh(center + toward * distance, center, Vec3::Y)
}

/// A uniform buffer for a camera matrix and the bind group reaching it.
pub(super) fn create_camera(wgpu: &Wgpu) -> (wgpu::Buffer, wgpu::BindGroup) {
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
pub(super) struct ShownPart {
    pub(super) part: Part,
    pub(super) count: usize,
    mass: Grams,
    pieces: Vec<(PartMesh, wgpu::Buffer)>,
    pub(super) center: Vec3,
    pub(super) radius: f32,
    strip_camera: wgpu::Buffer,
    strip_bind_group: wgpu::BindGroup,
}

impl ShownPart {
    pub(super) fn new(wgpu: &Wgpu, part: Part, count: usize, mass: Grams, sizes: PartSizes) -> Self {
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
        ShownPart { part, count, mass, pieces, center, radius, strip_camera, strip_bind_group }
    }

    pub(super) fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, bind_group: &'a wgpu::BindGroup) {
        pass.set_bind_group(0, bind_group, &[]);
        for (mesh, instance) in &self.pieces {
            pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            pass.set_vertex_buffer(1, instance.slice(..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
        }
    }
}

/// A square in the strip, as left, top and side, in physical pixels.
type Square = [f32; 3];

/// Squares for `count` parts in a window `width` × `height`, top down and shrunk
/// to fit when there are many.
fn strip_squares(width: f32, height: f32, count: usize) -> Vec<Square> {
    if count == 0 {
        return Vec::new();
    }
    let pitch = ((height - 2.0 * MARGIN) / count as f32).min(MAX_SQUARE + GAP);
    let side = (pitch - GAP).max(0.0).min(width / 4.0);
    let top = (height - pitch * count as f32) / 2.0;
    (0..count).map(|row| [MARGIN, top + pitch * row as f32, side]).collect()
}

pub struct PartsView {
    pipeline: wgpu::RenderPipeline,
    camera: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    shown: Vec<ShownPart>,
    brush: TextBrush<FontRef<'static>>,
    sections: Vec<OwnedSection>,
    squares: Vec<Square>,
    // Where the large view begins, to the right of the strip.
    large_left: f32,
    chosen: usize,
    azimuth: f32,
    elevation: f32,
    zoom: f32,
    turning: bool,
    dragging: bool,
    pointer: Option<PhysicalPosition<f64>>,
    started: Instant,
}

impl PartsView {
    /// The view of `fabric`'s parts, showing the one at `chosen`, if it has any.
    pub fn new(wgpu: &Wgpu, fabric: &Fabric, chosen: usize) -> Option<Self> {
        let connector = fabric.connector.as_ref()?;
        let sizes = PartSizes::of(&fabric.dimensions, &connector.dimensions);
        let list = parts_list(&placements(fabric));
        let ring_radius = fabric.dimensions.push_radius.f32();
        let shown: Vec<ShownPart> = list
            .iter()
            .map(|(&part, &count)| {
                let mass = part.mass(ring_radius, &connector.dimensions);
                ShownPart::new(wgpu, part, count, mass, sizes)
            })
            .collect();
        if shown.is_empty() {
            return None;
        }
        let pipeline = wgpu.create_plate_pipeline("Part View Pipeline", part_instance_layout());
        let (camera, bind_group) = create_camera(wgpu);
        let (width, height) = wgpu.size();
        let brush = BrushBuilder::using_font_bytes(include_bytes!("../../assets/WorkSans-Regular.ttf"))
            .unwrap()
            .with_depth_stencil(Some(default_depth_stencil_state()))
            .build(&wgpu.device, width as u32, height as u32, wgpu.surface_configuration.format);
        let chosen = chosen.min(shown.len() - 1);
        let mut view = PartsView {
            pipeline,
            camera,
            bind_group,
            shown,
            brush,
            sections: Vec::new(),
            squares: Vec::new(),
            large_left: 0.0,
            chosen,
            azimuth: AZIMUTH,
            elevation: ELEVATION,
            zoom: 1.0,
            turning: true,
            dragging: false,
            pointer: None,
            started: Instant::now(),
        };
        view.lay_out(wgpu);
        Some(view)
    }

    /// The part showing large, to come back to next time.
    pub fn chosen(&self) -> usize {
        self.chosen
    }

    pub fn resize(&mut self, wgpu: &Wgpu) {
        let (width, height) = wgpu.size();
        self.brush.resize_view(width, height, &wgpu.queue);
        self.lay_out(wgpu);
    }

    /// The strip's squares and all the text, for the window's size and the part
    /// chosen.
    fn lay_out(&mut self, wgpu: &Wgpu) {
        let (width, height) = wgpu.size();
        self.squares = strip_squares(width, height, self.shown.len());
        self.large_left = self.squares.first().map_or(0.0, |[left, _, side]| left + side + GAP + COUNT_WIDTH);
        let layout = Layout::default().v_align(VerticalAlign::Center).h_align(HorizontalAlign::Left);
        let text = |text: String, at: [f32; 2], color: [f32; 4], scale: f32| {
            OwnedSection::default()
                .with_layout(layout)
                .with_bounds([width, height])
                .with_screen_position(at)
                .add_text(OwnedText::new(text).with_color(color).with_scale(scale))
        };
        let large = &self.shown[self.chosen];
        let title = format!("{}   {} × {}", letter(self.chosen), large.count, large.part.name());
        let each = large.mass.f32();
        let mass = format!("{} each, {:.1} kg in all", weight(each), each * large.count as f32 / 1000.0);
        let hardware: f32 = self.shown.iter().map(|part| part.mass.f32() * part.count as f32).sum();
        self.sections = vec![
            text(title, [self.large_left, MARGIN], CHOSEN_TEXT, TITLE_SCALE),
            text(mass, [self.large_left, MARGIN + TITLE_SCALE * 1.2], OTHER_TEXT, COUNT_SCALE),
            text(
                format!("All the parts on the list: {:.1} kg", hardware / 1000.0),
                [self.large_left, height - MARGIN - COUNT_SCALE * 1.4],
                OTHER_TEXT,
                COUNT_SCALE,
            ),
            text("P or Esc to return".to_string(), [self.large_left, height - MARGIN], OTHER_TEXT, COUNT_SCALE),
        ];
        for (index, (part, [left, top, side])) in self.shown.iter().zip(&self.squares).enumerate() {
            let color = if index == self.chosen { CHOSEN_TEXT } else { OTHER_TEXT };
            let at = [left + side + GAP, top + side / 2.0];
            self.sections.push(text(format!("{}   {} ×", letter(index), part.count), at, color, COUNT_SCALE));
        }
    }

    /// The part whose row in the strip is under the pointer.
    fn row_under_pointer(&self) -> Option<usize> {
        let position = self.pointer?;
        let (x, y) = (position.x as f32, position.y as f32);
        self.squares.iter().position(|[left, top, side]| {
            (*left..self.large_left).contains(&x) && (*top..top + side).contains(&y)
        })
    }

    /// Show the part under the pointer large, or drag if it's on none of them.
    fn press(&mut self, wgpu: &Wgpu) {
        let Some(row) = self.row_under_pointer() else {
            self.dragging = true;
            if self.turning {
                self.azimuth += self.started.elapsed().as_secs_f32() * TURN_RATE;
                self.turning = false;
            }
            return;
        };
        self.chosen = row;
        self.zoom = 1.0;
        self.lay_out(wgpu);
    }

    pub fn pointer_changed(&mut self, change: PointerChange, wgpu: &Wgpu) {
        match change {
            PointerChange::Moved(position) => {
                if let (true, Some(last)) = (self.dragging, self.pointer) {
                    self.azimuth -= (position.x - last.x) as f32 * DRAG_RATE;
                    self.elevation = (self.elevation + (position.y - last.y) as f32 * DRAG_RATE).clamp(-1.5, 1.5);
                }
                self.pointer = Some(position);
            }
            PointerChange::Pressed => self.press(wgpu),
            PointerChange::TouchPressed(position) => {
                self.pointer = Some(position);
                self.press(wgpu);
            }
            PointerChange::Released(_) | PointerChange::TouchReleased(_) => self.dragging = false,
            PointerChange::Zoomed(amount) => self.zoom = (self.zoom * (1.0 + amount)).clamp(0.2, 20.0),
            PointerChange::NoChange => {}
        }
    }

    pub fn render(&mut self, wgpu: &Wgpu) -> Result<(), wgpu::SurfaceError> {
        let elapsed = self.started.elapsed().as_secs_f32();
        let azimuth = if self.turning { self.azimuth + elapsed * TURN_RATE } else { self.azimuth };
        let (width, height) = wgpu.size();
        let large = &self.shown[self.chosen];
        let aspect = (width - self.large_left) / height;
        let matrix = camera_matrix(large.center, large.radius, azimuth, self.elevation, aspect, FILL * self.zoom);
        wgpu.queue.write_buffer(&self.camera, 0, bytemuck::cast_slice(&matrix.to_cols_array()));
        let strip_azimuth = AZIMUTH + elapsed * TURN_RATE;
        for part in &self.shown {
            let matrix = camera_matrix(part.center, part.radius, strip_azimuth, ELEVATION, 1.0, FILL);
            wgpu.queue.write_buffer(&part.strip_camera, 0, bytemuck::cast_slice(&matrix.to_cols_array()));
        }
        self.brush.queue(&wgpu.device, &wgpu.queue, self.sections.iter().collect::<Vec<_>>()).unwrap();
        let surface_texture = wgpu.get_surface_texture()?;
        let target = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth = wgpu.create_depth_view();
        let mut encoder = wgpu.create_encoder();
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
            pass.set_pipeline(&self.pipeline);
            pass.set_viewport(self.large_left, 0.0, width - self.large_left, height, 0.0, 1.0);
            large.draw(&mut pass, &self.bind_group);
            for (part, [left, top, side]) in self.shown.iter().zip(&self.squares) {
                pass.set_viewport(*left, *top, *side, *side, 0.0, 1.0);
                part.draw(&mut pass, &part.strip_bind_group);
            }
            pass.set_viewport(0.0, 0.0, width, height, 0.0, 1.0);
            self.brush.draw(&mut pass);
        }
        wgpu.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
        Ok(())
    }
}
