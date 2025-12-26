use crate::camera::{Camera, Pick};
use crate::fabric::Fabric;
use crate::wgpu::fabric_renderer::FabricRenderer;
use crate::wgpu::surface_renderer::SurfaceRenderer;
use crate::wgpu::text_renderer::TextRenderer;
use crate::wgpu::Wgpu;
use crate::{
    ControlState, PickIntent, PointerChange, Radio, RenderStyle, StateChange,
    SHOW_ATTACHMENT_POINTS,
};
use std::rc::Rc;
use winit::dpi::PhysicalSize;

pub struct Scene {
    wgpu: Wgpu,
    camera: Camera,
    fabric_renderer: FabricRenderer,
    surface_renderer: SurfaceRenderer,
    text_renderer: TextRenderer,
    render_style: RenderStyle,
    pick_allowed: bool,
    model_scale: Option<f32>,
}

impl Scene {
    pub fn new(mobile_device: bool, wgpu: Wgpu, radio: Radio, model_scale: Option<f32>) -> Self {
        let camera = wgpu.create_camera(radio);
        let fabric_renderer = wgpu.create_fabric_renderer();
        let surface_renderer = wgpu.create_surface_renderer();
        let text_renderer = wgpu.create_text_renderer(mobile_device, model_scale);
        // Initialize the render style with attachment points hidden
        let render_style = RenderStyle::Normal {
            show_attachment_points: false,
        };

        // Initialize the thread-local state for joint text formatting
        SHOW_ATTACHMENT_POINTS.with(|cell| {
            *cell.borrow_mut() = false;
        });

        Self {
            wgpu,
            camera,
            fabric_renderer,
            surface_renderer,
            text_renderer,
            render_style,
            pick_allowed: false,
            model_scale,
        }
    }

    pub fn update_state(&mut self, state_change: StateChange) {
        use ControlState::*;
        use RenderStyle::*;
        use StateChange::*;
        self.text_renderer.update_state(&state_change);
        match state_change {
            ToggleProjection => {
                self.camera.toggle_projection();
            }
            ToggleAttachmentPoints => {
                // In model-scale mode, attachment points are not available
                if self.model_scale.is_none() {
                    self.render_style.toggle_attachment_points();
                }
            }
            SetControlState(control_state) => match control_state {
                Waiting | Building => self.reset(),
                Animating => {
                    self.reset();
                    self.pick_allowed = true;
                }
                Baking => {
                    self.render_style = WithAppearanceFunction {
                        function: Rc::new(|_| None),
                        show_attachment_points: false,
                    }
                }
                Viewing { .. } => {
                    self.reset();
                    self.pick_allowed = true;
                }
                ShowingJoint(_) => {
                    self.pick_allowed = true;
                }
                ShowingInterval(_) => {
                    self.pick_allowed = true;
                }
                PhysicsTesting => {
                    self.reset();
                    self.render_style = WithAppearanceFunction {
                        function: Rc::new(|_| None),
                        show_attachment_points: false,
                    }
                }
            },
            SetAnimating(_) => {}
            ResetView => {
                self.render_style = Normal {
                    show_attachment_points: false,
                };
            }
            RestartApproach => {
                self.camera.restart_approach();
            }
            ToggleColorByRole => {
                let show_attachment_points = self.render_style.show_attachment_points();
                self.render_style = match &self.render_style {
                    ColorByRole { .. } => Normal {
                        show_attachment_points,
                    },
                    _ => ColorByRole {
                        show_attachment_points,
                    },
                };
            }
            SetAppearanceFunction(appearance) => match &mut self.render_style {
                WithAppearanceFunction {
                    show_attachment_points,
                    ..
                } => {
                    self.render_style = WithAppearanceFunction {
                        function: appearance.clone(),
                        show_attachment_points: *show_attachment_points,
                    }
                }
                _ => {
                    panic!("Cannot set color function")
                }
            },
            SetIntervalColor { key, color } => match &mut self.render_style {
                WithPullMap { map, .. } => {
                    map.insert(key, color);
                }
                WithPushMap { map, .. } => {
                    map.insert(key, color);
                }
                _ => {
                    panic!("Cannot set interval color")
                }
            },
            _ => {}
        }
    }

    pub fn pick_allowed(&self) -> bool {
        self.pick_allowed
    }

    /// Returns the current pick state from the camera
    pub fn current_pick(&self) -> &Pick {
        &self.camera.current_pick()
    }

    pub fn render_style_shows_attachment_points(&self) -> bool {
        self.render_style.show_attachment_points()
    }

    fn render(&mut self, show_surface: bool) -> Result<(), wgpu::SurfaceError> {
        let surface_texture = self.wgpu.get_surface_texture()?;
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = self.wgpu.create_depth_view();
        let mut encoder = self.wgpu.create_encoder();
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        self.wgpu.set_bind_group(&mut render_pass);
        self.fabric_renderer.render(
            &mut render_pass,
            &self.wgpu.uniform_bind_group,
            &self.render_style,
        );
        // Only render surface when gravity is present
        if show_surface {
            self.surface_renderer.render(&mut render_pass);
        }
        self.text_renderer.render(&mut render_pass, &self.wgpu);
        drop(render_pass);
        self.wgpu.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
        Ok(())
    }

    pub fn redraw(&mut self, fabric: &Fabric, has_surface: bool) -> Result<(), wgpu::SurfaceError> {
        self.wgpu.update_mvp_matrix(self.camera.mvp_matrix());
        self.fabric_renderer.update(
            &mut self.wgpu,
            fabric,
            &self.camera.current_pick(),
            &mut self.render_style,
        );
        // Update surface size based on fabric bounding radius
        if has_surface {
            self.surface_renderer
                .update_radius(&self.wgpu.queue, fabric.bounding_radius());
        }
        self.render(has_surface)?;
        Ok(())
    }

    pub fn resize(&mut self, PhysicalSize { width, height }: PhysicalSize<u32>) {
        self.wgpu.resize((width, height));
        self.camera.set_size(width as f32, height as f32);
        // the texture!
    }

    pub fn pointer_changed(&mut self, pointer_changed: PointerChange, fabric: &Fabric) {
        // When picking is not allowed, convert pick intents to Reset (release without picking)
        let pointer_changed = if !self.pick_allowed {
            match pointer_changed {
                PointerChange::Released(_) => PointerChange::Released(PickIntent::Reset),
                PointerChange::TouchReleased(_) => PointerChange::TouchReleased(PickIntent::Reset),
                other => other,
            }
        } else {
            pointer_changed
        };

        self.camera.pointer_changed(pointer_changed, fabric);
    }

    pub fn animate(&mut self, fabric: &Fabric) -> bool {
        self.camera.target_approach(fabric) || matches!(self.camera.current_pick(), Pick::Nothing)
    }

    pub fn normal_rendering(&mut self) {
        self.render_style = RenderStyle::Normal {
            show_attachment_points: false,
        };
    }

    pub fn reset(&mut self) {
        self.pick_allowed = false;
        self.camera.reset();
    }

    /// Jump camera to ideal viewing position for the given fabric
    pub fn jump_to_fabric(&mut self, fabric: &Fabric) {
        self.camera.jump_to_fabric(fabric);
    }

    pub fn restart_approach(&mut self) {
        self.camera.restart_approach();
    }

    /// Check if camera needs initialization
    pub fn needs_camera_init(&self) -> bool {
        !self.camera.is_initialized()
    }

    /// Get camera view for export (position, look_at)
    pub fn export_view(&self) -> (cgmath::Point3<f32>, cgmath::Point3<f32>) {
        self.camera.export_view()
    }
}
