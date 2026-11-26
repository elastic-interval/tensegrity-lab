use crate::camera::Pick;
use crate::fabric::Fabric;
use crate::wgpu::cylinder_renderer::CylinderRenderer;
use crate::wgpu::connector_renderer::ConnectorRenderer;
use crate::wgpu::highlight_renderer::HighlightRenderer;
use crate::wgpu::Wgpu;
use crate::RenderStyle;

pub struct FabricRenderer {
    cylinder_renderer: CylinderRenderer,
    connector_renderer: ConnectorRenderer,
    highlight_renderer: HighlightRenderer,
}

impl FabricRenderer {
    pub fn new(wgpu: &Wgpu) -> Self {
        let cylinder_renderer = CylinderRenderer::new(wgpu);
        let connector_renderer = ConnectorRenderer::new(wgpu);
        let highlight_renderer = HighlightRenderer::new(wgpu);

        Self {
            cylinder_renderer,
            connector_renderer,
            highlight_renderer,
        }
    }

    pub fn update(
        &mut self,
        wgpu: &Wgpu,
        fabric: &Fabric,
        pick: &Pick,
        render_style: &RenderStyle,
        show_attachment_points: bool,
    ) {
        self.cylinder_renderer
            .update(wgpu, fabric, pick, render_style, show_attachment_points);

        if show_attachment_points {
            self.connector_renderer.update(wgpu, fabric, pick);
        }
    }

    /// Several fabrics, each placed where it stands, drawn plainly.
    pub fn update_placed(&mut self, wgpu: &Wgpu, fabrics: &[(&Fabric, glam::Mat4)]) {
        self.cylinder_renderer.update_placed(wgpu, fabrics);
    }

    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        bind_group: &'a wgpu::BindGroup,
        show_attachment_points: bool,
    ) {
        self.cylinder_renderer.render(render_pass, bind_group);

        if show_attachment_points {
            self.connector_renderer.render(render_pass, bind_group);
        }
    }

    pub fn update_highlight(
        &mut self,
        wgpu: &Wgpu,
        fabric: &Fabric,
        pick: &Pick,
        preview: Option<(&Pick, f32)>,
        light_background: bool,
        show_attachment_points: bool,
    ) {
        self.highlight_renderer.update(wgpu, fabric, pick, preview, light_background, show_attachment_points);
    }

    /// The selection's see-through highlight, drawn last so everything
    /// behind it shows through.
    pub fn render_highlight<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        bind_group: &'a wgpu::BindGroup,
    ) {
        self.highlight_renderer.render(render_pass, bind_group);
    }
}
