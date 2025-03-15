use crate::wgpu::Wgpu;
use wgpu::{Device, RenderPass, SurfaceConfiguration};
use wgpu_text::glyph_brush::ab_glyph::FontRef;
use wgpu_text::glyph_brush::{HorizontalAlign, Layout, Section, Text, VerticalAlign};
use wgpu_text::{BrushBuilder, TextBrush};

pub struct TextRenderer {
    brush: TextBrush<FontRef<'static>>,
    section: Section<'static>,
}

impl TextRenderer {
    pub fn new(device: &Device, surface_configuration: &SurfaceConfiguration) -> Self {
        let width = surface_configuration.width;
        let height = surface_configuration.height;
        let brush =
            BrushBuilder::using_font_bytes(include_bytes!("../../assets/Orbitron Bold.ttf"))
                .unwrap()
                .build(device, width, height, surface_configuration.format);
        let section = Section::default()
            .add_text(
                Text::new("yoloswag")
                    .with_color([1.0, 0.0, 0.0, 1.0])
                    .with_scale(40.0),
            )
            .with_layout(
                Layout::default()
                    .v_align(VerticalAlign::Center)
                    .h_align(HorizontalAlign::Center),
            )
            .with_bounds([width as f32, height as f32])
            .with_screen_position([width as f32 / 2.0, height as f32 / 2.0]);
        TextRenderer { brush, section }
    }

    pub fn draw<'a>(&'a mut self, render_pass: &mut RenderPass<'a>, wgpu: &Wgpu) {
        self.brush
            .queue(&wgpu.device, &wgpu.queue, [&self.section])
            .unwrap();
        self.brush.draw(render_pass);
    }
}
