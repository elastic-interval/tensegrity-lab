use crate::wgpu::text_state::TextState;
use crate::wgpu::Wgpu;
use wgpu::{Device, RenderPass, SurfaceConfiguration};
use wgpu_text::glyph_brush::ab_glyph::FontRef;
use wgpu_text::{BrushBuilder, TextBrush};
use crate::application::OverlayChange;

pub struct TextRenderer {
    text_state: TextState,
    brush: TextBrush<FontRef<'static>>,
}

impl TextRenderer {
    pub fn new(device: &Device, surface_configuration: &SurfaceConfiguration) -> Self {
        let width = surface_configuration.width;
        let height = surface_configuration.height;
        let brush =
            BrushBuilder::using_font_bytes(include_bytes!("../../assets/WorkSans-Regular.ttf"))
                .unwrap()
                .build(device, width, height, surface_configuration.format);
        let text_state = TextState::new("De Twisp".to_string(), width, height);
        TextRenderer { brush, text_state }
    }

    pub fn change_happened(&mut self, overlay_change: OverlayChange) {
        self.text_state.change_happened(overlay_change);
    }

    pub fn draw<'a>(&'a mut self, render_pass: &mut RenderPass<'a>, wgpu: &Wgpu) {
        self.brush
            .queue(&wgpu.device, &wgpu.queue, self.text_state.sections().clone())
            .unwrap();
        self.brush.draw(render_pass);
    }
}
