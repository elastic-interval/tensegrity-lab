use crate::camera::Pick;
use crate::fabric::Fabric;
use crate::wgpu::cylinder_renderer::CylinderRenderer;
use crate::wgpu::connector_renderer::ConnectorRenderer;
use crate::wgpu::Wgpu;
use crate::RenderStyle;

pub struct FabricRenderer {
    cylinder_renderer: CylinderRenderer,
    connector_renderer: ConnectorRenderer,
}

impl FabricRenderer {
    pub fn new(wgpu: &Wgpu) -> Self {
        let cylinder_renderer = CylinderRenderer::new(wgpu);
        let connector_renderer = ConnectorRenderer::new(wgpu);

        Self {
            cylinder_renderer,
            connector_renderer,
        }
    }

    pub fn update(
        &mut self,
        wgpu: &Wgpu,
        fabric: &Fabric,
        pick: &Pick,
        render_style: &RenderStyle,
        show_attachment_points: bool,
        color_approaching_cables: bool,
    ) {
        self.cylinder_renderer.update(
            wgpu,
            fabric,
            pick,
            render_style,
            show_attachment_points,
            color_approaching_cables,
        );

        if show_attachment_points {
            self.connector_renderer.update(wgpu, fabric, pick);
        }
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
}
