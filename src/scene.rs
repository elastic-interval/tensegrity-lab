use crate::application::AppStateChange;
use crate::camera::Target::*;
use crate::camera::{Camera, Pick};
use crate::fabric::material::interval_material;
use crate::fabric::Fabric;
use crate::messages::{ControlState, IntervalDetails, JointDetails, Scenario};
use crate::messages::{LabEvent, PointerChange};
use crate::scene::RenderStyle::WithColoring;
use crate::wgpu::fabric_renderer::FabricRenderer;
use crate::wgpu::surface_renderer::SurfaceRenderer;
use crate::wgpu::text_renderer::TextRenderer;
use crate::wgpu::Wgpu;
use std::collections::HashMap;
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoopProxy;

#[derive(Clone, Debug)]
pub enum IntervalFilter {
    ShowAll,
    ShowPush,
    ShowPull,
}

#[derive(Clone, Debug)]
pub enum RenderStyle {
    Normal,
    WithColoring {
        color_map: HashMap<(usize, usize), [f32; 4]>,
        filter: IntervalFilter,
    },
}

pub struct Scene {
    wgpu: Wgpu,
    camera: Camera,
    fabric_renderer: FabricRenderer,
    surface_renderer: SurfaceRenderer,
    text_renderer: TextRenderer,
    event_loop_proxy: EventLoopProxy<LabEvent>,
    render_style: RenderStyle,
    pick_allowed: bool,
}

impl Scene {
    pub fn new(
        mobile_device: bool,
        wgpu: Wgpu,
        event_loop_proxy: EventLoopProxy<LabEvent>,
    ) -> Self {
        let camera = wgpu.create_camera();
        let fabric_renderer = wgpu.create_fabric_renderer();
        let surface_renderer = wgpu.create_surface_renderer();
        let text_renderer = wgpu.create_text_renderer(mobile_device);
        Self {
            wgpu,
            camera,
            fabric_renderer,
            surface_renderer,
            text_renderer,
            event_loop_proxy,
            render_style: RenderStyle::Normal,
            pick_allowed: false,
        }
    }

    pub fn change_happened(&mut self, app_state_change: AppStateChange) {
        use AppStateChange::*;
        use ControlState::*;
        self.text_renderer.change_happened(&app_state_change);
        match app_state_change {
            SetControlState(control_state) => match control_state {
                Waiting | UnderConstruction | Animating => self.reset(),
                Viewing => {
                    self.reset();
                    self.pick_allowed = true;
                }
                ShowingJoint(_) | ShowingInterval(_) => {
                    self.pick_allowed = true;
                }
                Testing(scenario) => {
                    self.reset();
                    match scenario {
                        Scenario::TensionTest => {
                            self.render_style = WithColoring {
                                color_map: HashMap::new(),
                                filter: IntervalFilter::ShowPull,
                            };
                        }
                        Scenario::CompressionTest => {
                            self.render_style = WithColoring {
                                color_map: HashMap::new(),
                                filter: IntervalFilter::ShowPush,
                            };
                        }
                        _ => {}
                    }
                }
            },
            SetAnimating(active) => self.pick_allowed = !active,
            SetIntervalColor { key, color } => {
                if let WithColoring { color_map, .. } = &mut self.render_style {
                    color_map.insert(key, color);
                }
            }
            _ => {}
        }
    }

    pub fn pick_allowed(&self) -> bool {
        self.pick_allowed
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
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
        self.fabric_renderer
            .render(&mut render_pass, &self.wgpu.uniform_bind_group);
        self.surface_renderer.render(&mut render_pass);
        self.text_renderer.render(&mut render_pass, &self.wgpu);
        drop(render_pass);
        self.wgpu.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
        Ok(())
    }

    pub fn redraw(&mut self, fabric: &Fabric) -> Result<(), wgpu::SurfaceError> {
        self.wgpu.update_mvp_matrix(self.camera.mvp_matrix());
        self.fabric_renderer.update_from_fabric(
            &mut self.wgpu,
            fabric,
            &self.camera.current_pick(),
            &mut self.render_style,
        );
        self.render()?;
        Ok(())
    }

    pub fn resize(&mut self, PhysicalSize { width, height }: PhysicalSize<u32>) {
        self.wgpu.resize((width, height));
        self.camera.set_size(width as f32, height as f32);
        // the texture!
    }

    pub fn pointer_changed(&mut self, pointer_changed: PointerChange, fabric: &Fabric) {
        if let Some(pick) = self.camera.pointer_changed(pointer_changed, fabric) {
            self.camera_pick(pick);
        }
    }

    pub fn animate(&mut self, fabric: &Fabric) -> bool {
        self.camera.target_approach(fabric) || matches!(self.camera.current_pick(), Pick::Nothing)
    }

    pub fn reset(&mut self) {
        self.pick_allowed = false;
        self.camera.reset();
        self.camera_pick(self.camera.current_pick());
    }

    fn camera_pick(&mut self, pick: Pick) {
        match pick {
            Pick::Nothing => {
                self.camera.set_target(FabricMidpoint);
            }
            Pick::Joint { index, joint } => {
                self.camera.set_target(AroundJoint(index));
                let details = JointDetails {
                    index,
                    location: joint.location,
                };
                self.event_loop_proxy
                    .send_event(LabEvent::AppStateChanged(AppStateChange::SetControlState(
                        ControlState::ShowingJoint(details),
                    )))
                    .unwrap();
            }
            Pick::Interval {
                joint,
                id,
                interval,
                length,
            } => {
                self.camera.set_target(AroundInterval(id));
                let role = interval_material(interval.material).role;
                let near_joint = if interval.alpha_index == joint {
                    interval.alpha_index
                } else {
                    interval.omega_index
                };
                let far_joint = if interval.omega_index == joint {
                    interval.alpha_index
                } else {
                    interval.omega_index
                };
                let interval_details = IntervalDetails {
                    near_joint,
                    far_joint,
                    length,
                    role,
                };
                self.event_loop_proxy
                    .send_event(LabEvent::AppStateChanged(AppStateChange::SetControlState(
                        ControlState::ShowingInterval(interval_details),
                    )))
                    .unwrap();
            }
        }
    }
}
