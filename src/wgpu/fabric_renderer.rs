use crate::camera::Pick;
use crate::fabric::Fabric;
use crate::wgpu::attachment_renderer::AttachmentRenderer;
use crate::wgpu::cylinder_renderer::CylinderRenderer;
use crate::wgpu::joint_renderer::JointRenderer;
use crate::wgpu::Wgpu;
use crate::RenderStyle;

pub struct FabricRenderer {
    // Cylinder renderer for intervals
    cylinder_renderer: CylinderRenderer,

    // Joint renderer for selected joints
    joint_renderer: JointRenderer,

    // Attachment point renderer for push intervals
    attachment_renderer: AttachmentRenderer,
}

impl FabricRenderer {
    pub fn new(wgpu: &Wgpu) -> Self {
        // Create the cylinder renderer for intervals
        let cylinder_renderer = CylinderRenderer::new(wgpu);

        // Create the joint renderer for selected joints
        let joint_renderer = JointRenderer::new(wgpu);

        // Create the attachment point renderer for push intervals
        let attachment_renderer = AttachmentRenderer::new(wgpu);

        Self {
            cylinder_renderer,
            joint_renderer,
            attachment_renderer,
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

        // Enable joint renderer for joints that don't have connected push intervals
        self.joint_renderer.update(wgpu, fabric, pick);

        // Update the attachment point renderer only if attachment points should be shown
        if render_style.show_attachment_points() {
            self.attachment_renderer.update(wgpu, fabric, pick);
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

        // Render joint markers for selected joints without push intervals
        self.joint_renderer.render(render_pass, bind_group);

        // Render the attachment points only if they should be shown
        if render_style.show_attachment_points() {
            self.attachment_renderer.render(render_pass, bind_group);
        }
    }
}
