//! The gallery's signs: each piece's name lettered on the plate at the front
//! of its table, in the room, hidden by whatever stands before it.

use crate::wgpu::{default_depth_stencil_state, Wgpu};
use glam::Mat4;
use wgpu::RenderPass;
use wgpu_text::glyph_brush::ab_glyph::FontRef;
use wgpu_text::glyph_brush::{HorizontalAlign, Layout, OwnedSection, OwnedText, VerticalAlign};
use wgpu_text::{BrushBuilder, TextBrush};

/// Letters are laid out this many units tall, and a sign's frame scales
/// them to their size in the room.
pub const LETTER_UNITS: f32 = 64.0;
const LETTERING: [f32; 4] = [0.85, 0.82, 0.74, 1.0];

/// One brush for each sign, each drawing through its own sign's frame.
pub struct SignRenderer {
    brushes: Vec<TextBrush<FontRef<'static>>>,
}

impl SignRenderer {
    pub fn new(wgpu: &Wgpu, count: usize) -> SignRenderer {
        let (width, height) = (wgpu.surface_configuration.width, wgpu.surface_configuration.height);
        let brushes = (0..count)
            .map(|_| {
                BrushBuilder::using_font_bytes(include_bytes!("../../assets/WorkSans-Regular.ttf"))
                    .unwrap()
                    .with_depth_stencil(Some(default_depth_stencil_state()))
                    .build(&wgpu.device, width, height, wgpu.surface_configuration.format)
            })
            .collect();
        SignRenderer { brushes }
    }

    /// Each of `signs` lettered: its name, centred on its frame's origin,
    /// the frame taking layout units, x along the line and y down it, into
    /// the room, seen through `view`.
    pub fn render<'a>(&'a mut self, render_pass: &mut RenderPass<'a>, wgpu: &Wgpu, view: Mat4, signs: &[(Mat4, String)]) {
        for (brush, (frame, name)) in self.brushes.iter_mut().zip(signs) {
            brush.update_matrix((view * *frame).to_cols_array_2d(), &wgpu.queue);
            let section = OwnedSection::default()
                .with_layout(Layout::default().h_align(HorizontalAlign::Center).v_align(VerticalAlign::Center))
                .with_screen_position((0.0, 0.0))
                .with_bounds((f32::INFINITY, f32::INFINITY))
                .add_text(OwnedText::new(name.as_str()).with_scale(LETTER_UNITS).with_color(LETTERING));
            brush.queue(&wgpu.device, &wgpu.queue, [&section]).unwrap();
            brush.draw(render_pass);
        }
    }
}
