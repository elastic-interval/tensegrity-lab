use std::iter;

use bytemuck::cast_slice;
use leptos::{SignalSet, WriteSignal};
use winit::event::{ElementState, MouseButton};

use crate::camera::{Camera, Pick};
use crate::camera::Target::*;
use crate::fabric::Fabric;
use crate::fabric::interval::Span;
use crate::fabric::material::interval_material;
use crate::messages::{ControlState, IntervalDetails};
use crate::wgpu::drawing::Drawing;
use crate::wgpu::fabric_vertex::FabricVertex;
use crate::wgpu::surface_vertex::SurfaceVertex;
use crate::wgpu::Wgpu;

pub struct Scene {
    wgpu: Wgpu,
    camera: Camera,
    fabric_drawing: Drawing<FabricVertex>,
    surface_drawing: Drawing<SurfaceVertex>,
    set_control_state: WriteSignal<ControlState>,
}

impl Scene {
    pub fn new(wgpu: Wgpu, set_control_state: WriteSignal<ControlState>) -> Self {
        let camera = wgpu.create_camera();
        let fabric_drawing = wgpu.create_fabric_drawing();
        let surface_drawing = wgpu.create_surface_drawing();
        Self {
            wgpu,
            camera,
            fabric_drawing,
            surface_drawing,
            set_control_state,
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

    pub fn escape_happens(&mut self) {
        match self.camera.current_pick() {
            Pick::Nothing => {}
            Pick::Joint(_) => self.camera_pick(Pick::Nothing),
            Pick::Interval { joint, .. } => self.camera_pick(Pick::Joint(joint)),
        }
    }

    pub fn redraw(&mut self, fabric: &Fabric) {
        self.fabric_drawing.vertices.clear();
        let intervals = fabric.intervals.iter().flat_map(
            |(interval_id, interval)|
            FabricVertex::for_interval(interval_id, interval, fabric, &self.camera.current_pick())
        );
        self.fabric_drawing.vertices.extend(intervals);
        self.wgpu.update_mvp_matrix(self.camera.mvp_matrix());
        self.wgpu.queue.write_buffer(&self.fabric_drawing.buffer, 0, cast_slice(&self.fabric_drawing.vertices));
        let surface_texture = self.wgpu.surface_texture().expect("surface texture");
        let texture_view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.wgpu.create_encoder();
        self.render(&mut encoder, &texture_view);
        self.wgpu.queue.submit(iter::once(encoder.finish()));
        surface_texture.present();
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.wgpu.resize((width, height));
        self.camera.set_size(width as f32, height as f32);
    }

    pub fn mouse_input(&mut self, state: ElementState, button: MouseButton, fabric: &Fabric) {
        if let Some(pick) = self.camera.mouse_input(state, button, fabric) {
            self.camera_pick(pick);
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
                self.set_control_state.set(ControlState::Viewing);
            }
            Pick::Joint(joint_index) => {
                self.camera.set_target(AroundJoint(joint_index));
                self.set_control_state.set(ControlState::ShowingJoint(joint_index));
            }
            Pick::Interval { joint, id, interval } => {
                self.camera.set_target(AroundInterval(id));
                let role = interval_material(interval.material).role;
                let length = match interval.span {
                    Span::Fixed { length } => length,
                    _ => 0.0
                };
                let alpha_index = if interval.alpha_index == joint { interval.alpha_index } else { interval.omega_index };
                let omega_index = if interval.omega_index == joint { interval.alpha_index } else { interval.omega_index };
                let interval_details = IntervalDetails { alpha_index, omega_index, length, role };
                self.set_control_state.set(ControlState::ShowingInterval(interval_details));
            }
        }
    }
}
