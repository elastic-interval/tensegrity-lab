use crate::camera::{Camera, Pick};
use crate::fabric::Fabric;
use crate::wgpu::fabric_renderer::FabricRenderer;
use crate::wgpu::sky_renderer::SkyRenderer;
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
    pub wgpu: Wgpu,
    camera: Camera,
    sky_renderer: SkyRenderer,
    fabric_renderer: FabricRenderer,
    surface_renderer: SurfaceRenderer,
    text_renderer: TextRenderer,
    render_style: RenderStyle,
    show_attachment_points: bool,
    pick_allowed: bool,
    model_scale: Option<f32>,
}

impl Scene {
    pub fn new(mobile_device: bool, wgpu: Wgpu, radio: Radio, model_scale: Option<f32>) -> Self {
        let camera = wgpu.create_camera(radio);
        let sky_renderer = wgpu.create_sky_renderer();
        let fabric_renderer = wgpu.create_fabric_renderer();
        let surface_renderer = wgpu.create_surface_renderer();
        let text_renderer = wgpu.create_text_renderer(mobile_device, model_scale);
        let render_style = RenderStyle::Normal;
        SHOW_ATTACHMENT_POINTS.with(|cell| {
            *cell.borrow_mut() = false;
        });

        Self {
            wgpu,
            camera,
            sky_renderer,
            fabric_renderer,
            surface_renderer,
            text_renderer,
            render_style,
            show_attachment_points: false,
            pick_allowed: false,
            model_scale,
        }
    }

    fn toggle_attachment_points(&mut self) {
        self.show_attachment_points = !self.show_attachment_points;
        SHOW_ATTACHMENT_POINTS.with(|cell| {
            *cell.borrow_mut() = self.show_attachment_points;
        });
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
                    self.toggle_attachment_points();
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
                    }
                }
            },
            SetAnimating(_) => {}
            ResetView => {
                self.render_style = Normal;
            }
            RestartApproach => {
                self.camera.restart_approach();
            }
            ToggleColorByRole => {
                self.render_style = match &self.render_style {
                    ColorByRole => Normal,
                    _ => ColorByRole,
                };
            }
            SetAppearanceFunction(appearance) => match &mut self.render_style {
                WithAppearanceFunction { .. } => {
                    self.render_style = WithAppearanceFunction {
                        function: appearance.clone(),
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
        self.show_attachment_points
    }

    fn render(&mut self, show_surface: bool) -> Result<(), wgpu::SurfaceError> {
        let surface_texture = self.wgpu.get_surface_texture()?;
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = self.wgpu.create_depth_view();
        let mut encoder = self.wgpu.create_encoder();

        // First pass: render sky (no depth testing, clears the screen)
        {
            let mut sky_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Sky Pass"),
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
                    depth_slice: None,
                })],
                multiview_mask: None,
                depth_stencil_attachment: None, // No depth for sky
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.sky_renderer.render(&mut sky_pass);
        }

        // Second pass: render everything else with depth testing
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Main Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Keep sky background
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            multiview_mask: None,
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
            self.show_attachment_points,
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

    pub fn redraw(
        &mut self,
        fabric: &Fabric,
        has_surface: bool,
        delta_seconds: f32,
    ) -> Result<(), wgpu::SurfaceError> {
        self.wgpu.update_mvp_matrix(self.camera.mvp_matrix());
        self.sky_renderer
            .update_time(&self.wgpu.queue, delta_seconds);
        self.fabric_renderer.update(
            &mut self.wgpu,
            fabric,
            &self.camera.current_pick(),
            &self.render_style,
            self.show_attachment_points,
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
        // When picking is not allowed (or always on the web — pure mouse-driven, no
        // selection), convert pick intents to Reset (release without picking). Camera
        // orbit/zoom (Moved/Pressed/Zoomed) still pass through.
        let pointer_changed = if !self.pick_allowed || cfg!(target_arch = "wasm32") {
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
        self.render_style = RenderStyle::Normal;
        self.show_attachment_points = false;
    }

    pub fn reset(&mut self) {
        self.pick_allowed = false;
        self.camera.reset();
    }

    /// Jump camera to ideal viewing position for the given fabric
    pub fn jump_to_fabric(&mut self, fabric: &Fabric) {
        self.camera.jump_to_fabric(fabric);
    }

    /// Refit camera radius to the new fabric while keeping the current
    /// orbit angle (Show mode).
    pub fn refit_camera_to_fabric(&mut self, fabric: &Fabric) {
        self.camera.refit_to_fabric(fabric);
    }

    pub fn restart_approach(&mut self) {
        self.camera.restart_approach();
    }

    /// Slowly rotate the camera around the vertical axis (Show mode).
    pub fn orbit_camera_y(&mut self, angle_rad: f32) {
        self.camera.orbit_around_y(angle_rad);
    }

    /// Check if camera needs initialization
    pub fn needs_camera_init(&self) -> bool {
        !self.camera.is_initialized()
    }

    /// Position camera for watching a sphere drop. Places the camera
    /// at ground level, far enough back to see the full drop.
    pub fn position_camera_for_drop(&mut self, radius: f32) {
        use glam::Vec3;
        // Sphere center starts at ~2*radius. Camera at ground level,
        // far enough to see the whole sphere + its drop path.
        let distance = radius * 5.0;
        let camera_pos = Vec3::new(distance, radius * 0.2, 0.0);
        let look_at = Vec3::new(0.0, radius, 0.0);
        self.camera.set_position_and_hold(camera_pos, look_at);
    }

    /// Get camera view for export (position, look_at)
    pub fn export_view(&self) -> (glam::Vec3, glam::Vec3) {
        self.camera.export_view()
    }
}
