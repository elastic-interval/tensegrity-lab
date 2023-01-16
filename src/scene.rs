use std::mem;

use bytemuck::{cast_slice, Pod, Zeroable};
use iced_wgpu::wgpu;
#[allow(unused_imports)]
use log::info;
use wgpu::{CommandEncoder, TextureView};
use wgpu::util::DeviceExt;
use winit::event::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::camera::Camera;
use crate::fabric::Fabric;
use crate::fabric::interval::Interval;
use crate::fabric::interval::Role::{Measure, Pull, Push};
use crate::graphics::{get_depth_stencil_state, get_primitive_state, GraphicsWindow};
use crate::gui::Controls;

const MAX_INTERVALS: usize = 5000;

pub struct Scene {
    camera: Camera,
    vertices: [Vertex; MAX_INTERVALS * 2],
    vertex_count: usize,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
}

impl Scene {
    pub fn new(graphics: &GraphicsWindow) -> Self {
        let shader = graphics.get_shader_module();
        let scale = 3.0;
        let aspect = graphics.config.width as f32 / graphics.config.height as f32;
        let camera = Camera::new((3.0 * scale, 1.5 * scale, 3.0 * scale).into(), aspect);
        let uniform_buffer = graphics.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("MVP"),
            contents: cast_slice(&[0.0f32; 16]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let uniform_bind_group_layout = graphics.create_uniform_bind_group_layout();
        let uniform_bind_group = graphics.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("Uniform Bind Group"),
        });

        let pipeline_layout = graphics.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = graphics.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: graphics.config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: get_primitive_state(),
            depth_stencil: Some(get_depth_stencil_state()),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let vertices = [Vertex::default(); MAX_INTERVALS * 2];
        let vertex_buffer = graphics.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            camera,
            vertices,
            vertex_count: 0,
            pipeline,
            vertex_buffer,
            uniform_buffer,
            uniform_bind_group,
        }
    }

    pub fn render(&self, encoder: &mut CommandEncoder, view: &TextureView, depth_view: &TextureView) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
                    store: true,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: false,
                }),
                stencil_ops: None,
            }),
        });
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.draw(0..self.vertex_count as u32, 0..1);
    }

    pub fn window_event(&mut self, event: &WindowEvent) {
        self.camera.window_event(event);
    }

    pub fn update(&mut self, graphics: &GraphicsWindow, controls: &Controls, fabric: &Fabric) {
        self.update_from_fabric(fabric, controls);
        self.update_from_camera(graphics);
        graphics.queue.write_buffer(&self.vertex_buffer, 0, cast_slice(&self.vertices));
    }

    fn update_from_fabric(&mut self, fabric: &Fabric, controls: &Controls) {
        let measure_limits = fabric.measure_limits();
        let measure_lower_limit = match measure_limits {
            Some(limits) => limits.interpolate(controls.measure_threshold),
            None => f32::NEG_INFINITY,
        };
        let updated_vertices = fabric.interval_values()
            .filter(|Interval { strain, role, .. }| *role != Measure || *strain > measure_lower_limit)
            .flat_map(|interval| Vertex::for_interval(interval, fabric));
        self.vertex_count = 0;
        for (vertex, slot) in updated_vertices.zip(self.vertices.iter_mut()) {
            *slot = vertex;
            self.vertex_count += 1;
        }
        self.camera.target_approach(fabric.midpoint())
    }


    pub fn resize(&mut self, graphics: &GraphicsWindow) {
        let new_size = graphics.size;
        let aspect = new_size.width as f32 / new_size.height as f32;
        self.camera.set_aspect(aspect);
        self.update_from_camera(graphics);
    }

    fn update_from_camera(&self, graphics: &GraphicsWindow) {
        let mvp_mat = self.camera.mvp_matrix();
        let mvp_ref: &[f32; 16] = mvp_mat.as_ref();
        graphics.queue.write_buffer(&self.uniform_buffer, 0, cast_slice(mvp_ref));
    }

    pub fn adjust_camera_up(&mut self, up: f32) {
        self.camera.go_up(up);
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Default)]
struct Vertex {
    position: [f32; 4],
    color: [f32; 4],
}

impl Vertex {
    pub fn for_interval(interval: &Interval, fabric: &Fabric) -> [Vertex; 2] {
        let (alpha, omega) = interval.locations(&fabric.joints);
        let color = match interval.role {
            Push => [1.0, 1.0, 1.0, 1.0],
            Pull => [0.2, 0.2, 1.0, 1.0],
            Measure => if interval.strain < 0.0 {
                [1.0, 0.8, 0.0, 1.0]
            } else {
                [0.0, 1.0, 0.0, 1.0]
            },
        };
        [
            Vertex { position: [alpha.x, alpha.y, alpha.z, 1.0], color },
            Vertex { position: [omega.x, omega.y, omega.z, 1.0], color }
        ]
    }

    const ATTRIBUTES: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0=>Float32x4, 1=>Float32x4];
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

