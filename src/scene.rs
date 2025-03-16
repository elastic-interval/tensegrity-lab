use crate::application::OverlayChange::SetControlState;
use crate::camera::Target::*;
use crate::camera::{Camera, Pick};
use crate::fabric::material::interval_material;
use crate::fabric::{Fabric, FabricStats};
use crate::messages::{ControlState, IntervalDetails, JointDetails};
use crate::messages::{LabEvent, PointerChange};
use crate::wgpu::fabric_renderer::FabricRenderer;
use crate::wgpu::fabric_vertex::FabricVertex;
use crate::wgpu::surface_renderer::SurfaceRenderer;
use crate::wgpu::text_renderer::TextRenderer;
use crate::wgpu::Wgpu;
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoopProxy;
use ControlState::{ShowingInterval, ShowingJoint};
use LabEvent::OverlayChanged;
use crate::application::OverlayChange;

pub struct Scene {
    wgpu: Wgpu,
    camera: Camera,
    fabric_renderer: FabricRenderer,
    surface_renderer: SurfaceRenderer,
    text_renderer: TextRenderer,
    event_loop_proxy: EventLoopProxy<LabEvent>,
}

impl Scene {
    pub fn new(wgpu: Wgpu, event_loop_proxy: EventLoopProxy<LabEvent>) -> Self {
        let camera = wgpu.create_camera();
        let fabric_renderer = wgpu.create_fabric_renderer();
        let surface_renderer = wgpu.create_surface_renderer();
        let text_renderer = wgpu.create_text_renderer();
        Self {
            wgpu,
            camera,
            fabric_renderer,
            surface_renderer,
            text_renderer,
            event_loop_proxy,
        }
    }

    pub fn change_happened(&mut self, overlay_change: OverlayChange) {
        self.text_renderer.text_state.change_happened(overlay_change);
    }

    pub fn pick_active(&self)-> bool {
        self.text_renderer.text_state.pick_active
    }

    fn render(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
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
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        self.wgpu.set_bind_group(&mut render_pass);
        self.fabric_renderer.draw(&mut render_pass);
        self.surface_renderer.draw(&mut render_pass);
        self.text_renderer.draw(&mut render_pass, &self.wgpu);
    }

    pub fn redraw(&mut self, fabric: &Fabric) {
        self.wgpu.update_mvp_matrix(self.camera.mvp_matrix());
        let vertexes = fabric.intervals.iter().flat_map(|(interval_id, interval)| {
            FabricVertex::for_interval(interval_id, interval, fabric, &self.camera.current_pick())
        });
        self.fabric_renderer.update(&mut self.wgpu, vertexes);
        let surface_texture = self.wgpu.surface_texture().expect("surface texture");
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.wgpu.create_encoder();
        self.render(&mut encoder, &texture_view);
        self.wgpu.queue.submit(Some(encoder.finish()));
        surface_texture.present();
    }

    pub fn resize(&mut self, PhysicalSize { width, height }: PhysicalSize<u32>) {
        self.wgpu.resize((width, height));
        self.camera.set_size(width as f32, height as f32);
    }

    pub fn pointer_changed(&mut self, pointer_changed: PointerChange, fabric: &Fabric) {
        if let Some(pick) = self.camera.pointer_changed(pointer_changed, fabric) {
            self.camera_pick(pick);
        }
    }

    pub fn current_pick(&self) -> Pick {
        self.camera.current_pick()
    }

    pub fn target_approach(&mut self, fabric: &Fabric) -> bool {
        self.camera.target_approach(fabric)
    }

    pub fn reset(&mut self) {
        self.camera.reset();
        self.camera_pick(self.camera.current_pick());
    }

    pub fn update_fabric_stats(&mut self, fabric_stats: &FabricStats) {
        let display_text = format!("{fabric_stats:?}");
        self.text_renderer.update(display_text)
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
                    .send_event(OverlayChanged(SetControlState(ShowingJoint(details))))
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
                    .send_event(OverlayChanged(SetControlState(ShowingInterval(
                        interval_details,
                    ))))
                    .unwrap();
            }
        }
    }
}
