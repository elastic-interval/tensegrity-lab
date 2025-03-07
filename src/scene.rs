use crate::application::OverlayChange::SetControlState;
use crate::camera::Target::*;
use crate::camera::{Camera, Pick, Shot};
use crate::fabric::material::interval_material;
use crate::fabric::Fabric;
use crate::messages::LabEvent;
use crate::messages::{ControlState, IntervalDetails, JointDetails};
use crate::wgpu::drawing::Drawing;
use crate::wgpu::fabric_vertex::FabricVertex;
use crate::wgpu::surface_vertex::SurfaceVertex;
use crate::wgpu::Wgpu;
use bytemuck::cast_slice;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton};
use winit::event_loop::EventLoopProxy;
use ControlState::{ShowingInterval, ShowingJoint};
use LabEvent::OverlayChanged;

pub struct Scene {
    wgpu: Wgpu,
    camera: Camera,
    fabric_drawing: Drawing<FabricVertex>,
    surface_drawing: Drawing<SurfaceVertex>,
    event_loop_proxy: EventLoopProxy<LabEvent>,
}

impl Scene {
    pub fn new(wgpu: Wgpu, event_loop_proxy: EventLoopProxy<LabEvent>) -> Self {
        let camera = wgpu.create_camera();
        let fabric_drawing = wgpu.create_fabric_drawing();
        let surface_drawing = wgpu.create_surface_drawing();
        Self {
            wgpu,
            camera,
            fabric_drawing,
            surface_drawing,
            event_loop_proxy,
        }
    }

    fn render(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
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
        render_pass.set_bind_group(0, &self.wgpu.uniform_bind_group, &[]);

        render_pass.set_pipeline(&self.fabric_drawing.pipeline);
        render_pass.set_vertex_buffer(0, self.fabric_drawing.buffer.slice(..));
        render_pass.draw(0..self.fabric_drawing.vertices.len() as u32, 0..1);

        render_pass.set_pipeline(&self.surface_drawing.pipeline);
        render_pass.set_vertex_buffer(0, self.surface_drawing.buffer.slice(..));
        render_pass.draw(0..self.surface_drawing.vertices.len() as u32, 0..1);
    }

    pub fn redraw(&mut self, fabric: &Fabric) {
        let intervals = fabric.intervals.iter().flat_map(|(interval_id, interval)| {
            FabricVertex::for_interval(interval_id, interval, fabric, &self.camera.current_pick())
        });
        self.fabric_drawing.vertices.clear();
        self.fabric_drawing.vertices.extend(intervals);
        self.wgpu.update_mvp_matrix(self.camera.mvp_matrix());
        self.wgpu.queue.write_buffer(
            &self.fabric_drawing.buffer,
            0,
            cast_slice(&self.fabric_drawing.vertices),
        );
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

    pub fn mouse_input(
        &mut self,
        state: ElementState,
        button: MouseButton,
        pick_active: bool,
        fabric: &Fabric,
    ) {
        let shot = if pick_active {
            match button {
                MouseButton::Right => Shot::Joint,
                _ => Shot::Interval,
            }
        } else {
            Shot::NoPick
        };
        if let Some(pick) = self.camera.mouse_input(state, shot, fabric) {
            if pick_active {
                self.camera_pick(pick);
            }
        }
    }

    pub fn camera(&mut self) -> &mut Camera {
        &mut self.camera
    }

    pub fn reset(&mut self) {
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
