use crate::camera::Pick;
use crate::fabric::Fabric;
use crate::wgpu::attachment_renderer::AttachmentRenderer;
use crate::wgpu::cylinder_renderer::CylinderRenderer;
use crate::wgpu::knot_renderer::KnotRenderer;
use crate::wgpu::Wgpu;
use crate::RenderStyle;

pub struct FabricRenderer {
    // Cylinder renderer for intervals
    cylinder_renderer: CylinderRenderer,

    // Attachment point renderer for push intervals
    attachment_renderer: AttachmentRenderer,

    // Knot renderer for pull interval ends at attachment points
    knot_renderer: KnotRenderer,
}

impl FabricRenderer {
    pub fn new(wgpu: &Wgpu) -> Self {
        // Create the cylinder renderer for intervals
        let cylinder_renderer = CylinderRenderer::new(wgpu);

        // Create the attachment point renderer for push intervals
        let attachment_renderer = AttachmentRenderer::new(wgpu);

        // Create the knot renderer for pull interval ends
        let knot_renderer = KnotRenderer::new(wgpu);

        Self {
            cylinder_renderer,
            attachment_renderer,
            knot_renderer,
        }
    }

    pub fn update(
        &mut self,
        wgpu: &Wgpu,
        fabric: &Fabric,
        pick: &Pick,
        render_style: &mut RenderStyle,
    ) {
        // Update the cylinder renderer with the new instances
        self.cylinder_renderer
            .update(wgpu, fabric, pick, render_style);

        // Update the attachment point renderer only if attachment points should be shown
        if render_style.show_attachment_points() {
            self.attachment_renderer.update(wgpu, fabric, pick);
            self.knot_renderer.update(wgpu, fabric, pick);
        }
    }

    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        bind_group: &'a wgpu::BindGroup,
        render_style: &RenderStyle,
    ) {
        // Render the cylinders for intervals
        self.cylinder_renderer.render(render_pass, bind_group);

        // Render the attachment points and knots only if they should be shown
        if render_style.show_attachment_points() {
            self.attachment_renderer.render(render_pass, bind_group);
            self.knot_renderer.render(render_pass, bind_group);
        }
    }
}
