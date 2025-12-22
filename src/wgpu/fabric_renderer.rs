use crate::camera::Pick;
use crate::fabric::Fabric;
use crate::wgpu::cylinder_renderer::CylinderRenderer;
use crate::wgpu::hinge_renderer::HingeRenderer;
use crate::wgpu::Wgpu;
use crate::RenderStyle;

pub struct FabricRenderer {
    cylinder_renderer: CylinderRenderer,
    hinge_renderer: HingeRenderer,
}

impl FabricRenderer {
    pub fn new(wgpu: &Wgpu) -> Self {
        let cylinder_renderer = CylinderRenderer::new(wgpu);
        let hinge_renderer = HingeRenderer::new(wgpu);

        Self {
            cylinder_renderer,
            hinge_renderer,
        }
    }

    pub fn update(
        &mut self,
        wgpu: &Wgpu,
        fabric: &Fabric,
        pick: &Pick,
        render_style: &mut RenderStyle,
    ) {
        self.cylinder_renderer
            .update(wgpu, fabric, pick, render_style);

        if render_style.show_attachment_points() {
            self.hinge_renderer.update(wgpu, fabric, pick);
        }
    }

    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        bind_group: &'a wgpu::BindGroup,
        render_style: &RenderStyle,
    ) {
        self.cylinder_renderer.render(render_pass, bind_group);

        if render_style.show_attachment_points() {
            self.hinge_renderer.render(render_pass, bind_group);
        }
    }
}
