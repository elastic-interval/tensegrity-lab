use crate::wgpu::Wgpu;
use wgpu::{Device, RenderPass, SurfaceConfiguration};
use wgpu_text::glyph_brush::ab_glyph::FontRef;
use wgpu_text::glyph_brush::{HorizontalAlign, Layout, OwnedSection, OwnedText, VerticalAlign};
use wgpu_text::{BrushBuilder, TextBrush};

pub struct TextRenderer {
    brush: TextBrush<FontRef<'static>>,
    section: OwnedSection,
}

impl TextRenderer {
    pub fn new(device: &Device, surface_configuration: &SurfaceConfiguration) -> Self {
        let width = surface_configuration.width;
        let height = surface_configuration.height;
        let brush =
            BrushBuilder::using_font_bytes(include_bytes!("../../assets/Orbitron Bold.ttf"))
                .unwrap()
                .build(device, width, height, surface_configuration.format);
        let section = OwnedSection::default()
            .add_text(
                OwnedText::new("Building..")
                    .with_color([0.8, 0.8, 0.8, 1.0])
                    .with_scale(40.0),
            )
            .with_layout(
                Layout::default()
                    .v_align(VerticalAlign::Center)
                    .h_align(HorizontalAlign::Center),
            )
            .with_bounds([width as f32 / 2.0, 300.0])
            .with_screen_position([width as f32 / 2.0, 100.0]);
        TextRenderer { brush, section }
    }
    
    pub fn update(&mut self, new_text: String) {
        self.section.text[0].text = new_text;
    }

    pub fn draw<'a>(&'a mut self, render_pass: &mut RenderPass<'a>, wgpu: &Wgpu) {
        self.brush
            .queue(&wgpu.device, &wgpu.queue, [&self.section])
            .unwrap();
        self.brush.draw(render_pass);
    }
}
