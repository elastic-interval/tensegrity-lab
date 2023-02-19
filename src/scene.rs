use std::f32::consts::PI;
use std::mem;

use bytemuck::{cast_slice, Pod, Zeroable};
use iced_wgpu::wgpu;
use wgpu::{CommandEncoder, TextureView};
use wgpu::util::DeviceExt;
use winit::event::*;
use SceneVariant::{*};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::camera::Camera;
use crate::camera::Target::{*};
use crate::fabric::{Fabric, UniqueId};
use crate::fabric::face::Face;
use crate::fabric::interval::Interval;
use crate::fabric::interval::Role::{Pull, Push};
use crate::graphics::{GraphicsWindow, line_list_primitive_state, triangle_list_primitive_state};

const MAX_INTERVALS: usize = 5000;

struct Drawing<V> {
    pipeline: wgpu::RenderPipeline,
    vertices: Vec<V>,
    buffer: wgpu::Buffer,
}

#[derive(Debug, Clone)]
pub enum SceneAction {
    Variant(SceneVariant),
    WatchMidpoint,
    WatchOrigin,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SceneVariant {
    Suspended,
    Pretensing,
    Tinkering,
    TinkeringOnFace(UniqueId),
    ShowingStrain { threshold: f32, material: usize },
}

pub struct Scene {
    variant: SceneVariant,
    camera: Camera,
    fabric_drawing: Drawing<FabricVertex>,
    surface_drawing: Drawing<SurfaceVertex>,
    uniform_bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
}

impl Scene {
    pub fn new(graphics: &GraphicsWindow) -> Self {
        let shader = graphics.get_shader_module();
        let scale = 6.0;
        let aspect = graphics.config.width as f32 / graphics.config.height as f32;
        let camera = Camera::new((2.0 * scale, 1.0 * scale, 2.0 * scale).into(), aspect);
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

        let fabric_vertices = vec![FabricVertex::default(); MAX_INTERVALS * 2];
        let fabric_pipeline = graphics.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Fabric Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "fabric_vertex",
                buffers: &[FabricVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fabric_fragment",
                targets: &[Some(wgpu::ColorTargetState {
                    format: graphics.config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: line_list_primitive_state(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        let fabric_buffer = graphics.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: cast_slice(&fabric_vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let surface_vertices = SurfaceVertex::for_radius(10.0).to_vec();
        let surface_pipeline = graphics.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Surface Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "surface_vertex",
                buffers: &[SurfaceVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "surface_fragment",
                targets: &[Some(wgpu::ColorTargetState {
                    format: graphics.config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: triangle_list_primitive_state(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        let surface_buffer = graphics.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Surface Buffer"),
            contents: cast_slice(&surface_vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            variant: Suspended,
            camera,
            fabric_drawing: Drawing {
                pipeline: fabric_pipeline,
                vertices: fabric_vertices,
                buffer: fabric_buffer,
            },
            surface_drawing: Drawing {
                pipeline: surface_pipeline,
                vertices: surface_vertices,
                buffer: surface_buffer,
            },
            uniform_buffer,
            uniform_bind_group,
        }
    }

    pub fn render(&self, encoder: &mut CommandEncoder, view: &TextureView) {
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
            depth_stencil_attachment: None,
        });
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);

        render_pass.set_pipeline(&self.fabric_drawing.pipeline);
        render_pass.set_vertex_buffer(0, self.fabric_drawing.buffer.slice(..));
        render_pass.draw(0..self.fabric_drawing.vertices.len() as u32, 0..1);

        let show_surface = match self.variant {
            Tinkering | Suspended | TinkeringOnFace(_) => false,
            Pretensing | ShowingStrain { .. } => true,
        };
        if show_surface {
            render_pass.set_pipeline(&self.surface_drawing.pipeline);
            render_pass.set_vertex_buffer(0, self.surface_drawing.buffer.slice(..));
            render_pass.draw(0..self.surface_drawing.vertices.len() as u32, 0..1);
        }
    }

    pub fn window_event(&mut self, event: &WindowEvent) {
        self.camera.window_event(event);
    }

    pub fn update(&mut self, graphics: &GraphicsWindow, fabric: &Fabric) {
        self.update_from_fabric(fabric);
        self.update_from_camera(graphics);
        graphics.queue.write_buffer(&self.fabric_drawing.buffer, 0, cast_slice(&self.fabric_drawing.vertices));
    }

    fn update_from_fabric(&mut self, fabric: &Fabric) {
        self.fabric_drawing.vertices.clear();
        self.fabric_drawing.vertices.extend(fabric.interval_values()
            .flat_map(|interval| FabricVertex::for_interval(interval, fabric, &self.variant)));
        let show_faces = match self.variant {
            Suspended | Pretensing | ShowingStrain { .. } => false,
            Tinkering | TinkeringOnFace(_) => true,
        };
        if show_faces {
            self.fabric_drawing.vertices.extend(fabric.faces.iter()
                .flat_map(|(face_id, face)| FabricVertex::for_face(*face_id, face, fabric, &self.variant)));
        }
        self.camera.target_approach(fabric);
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

    pub fn target_face_id(&self, fabric: &Fabric) -> Option<UniqueId> {
        match self.camera.target {
            Origin | FabricMidpoint => None,
            SelectedFace(face_id) => fabric.faces.contains_key(&face_id).then_some(face_id),
        }
    }

    pub fn action(&mut self, scene_action: SceneAction) {
        match scene_action {
            SceneAction::Variant(variant) => {
                self.variant = variant;
            }
            SceneAction::WatchMidpoint => {
                self.camera.target = FabricMidpoint;
            }
            SceneAction::WatchOrigin => {
                self.camera.target = Origin
            }
        }
    }

    pub fn select_next_face(&mut self, face_id: Option<UniqueId>, fabric: &Fabric) {
        let face_id = face_id.unwrap_or(match self.camera.target {
            Origin | FabricMidpoint => {
                *fabric.faces.keys().next().unwrap()
            }
            SelectedFace(face_id) => {
                let face_position = fabric.faces.keys()
                    .position(|&id| face_id == id)
                    .expect("Face id not found");
                let &new_face_id = fabric.faces.keys()
                    .cycle()
                    .nth(face_position + 1)
                    .unwrap();
                new_face_id
            }
        });
        self.select_face(Some(face_id));
    }

    pub fn select_face(&mut self, face_id: Option<UniqueId>) {
        self.camera.target = match face_id {
            None => FabricMidpoint,
            Some(face_id) => SelectedFace(face_id)
        };
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Default)]
struct FabricVertex {
    position: [f32; 4],
    color: [f32; 4],
}

impl FabricVertex {
    pub fn for_interval(interval: &Interval, fabric: &Fabric, variation: &SceneVariant) -> [FabricVertex; 2] {
        let (alpha, omega) = interval.locations(&fabric.joints);
        let color = match variation {
            Suspended | Tinkering | Pretensing | TinkeringOnFace(_) => {
                match fabric.materials[interval.material].role {
                    Push => [1.0, 1.0, 1.0, 1.0],
                    Pull => [0.2, 0.2, 1.0, 1.0],
                }
            }
            ShowingStrain { threshold, material } => {
                if fabric.materials[interval.material].role == Pull &&
                    interval.material == *material && interval.strain > *threshold {
                    [0.0, 1.0, 0.0, 1.0]
                } else {
                    [0.3, 0.3, 0.3, 0.5]
                }
            }
        };
        [
            FabricVertex { position: [alpha.x, alpha.y, alpha.z, 1.0], color },
            FabricVertex { position: [omega.x, omega.y, omega.z, 1.0], color }
        ]
    }

    pub fn for_face(face_id: UniqueId, face: &Face, fabric: &Fabric, variation: &SceneVariant) -> [FabricVertex; 2] {
        let (alpha, _, omega) = face.visible_points(fabric);
        let (alpha_color, omega_color) = match variation {
            Pretensing | Suspended | ShowingStrain { .. } => {
                unreachable!()
            }
            TinkeringOnFace(selected_face) if *selected_face == face_id => {
                ([0.0, 1.0, 0.0, 1.0], [0.0, 1.0, 0.0, 1.0])
            }
            Tinkering | TinkeringOnFace(_) =>
                ([1.0, 0.0, 0.0, 1.0], [1.0, 0.0, 0.0, 1.0]),
        };
        [
            FabricVertex { position: [alpha.x, alpha.y, alpha.z, 1.0], color: alpha_color },
            FabricVertex { position: [omega.x, omega.y, omega.z, 1.0], color: omega_color }
        ]
    }

    const ATTRIBUTES: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0=>Float32x4, 1=>Float32x4];
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<FabricVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Default)]
struct SurfaceVertex {
    position: [f32; 4],
}

impl SurfaceVertex {
    pub fn for_radius(radius: f32) -> [SurfaceVertex; 18] {
        let origin = [0f32, 0.0, 0.0, 1.0];
        let point: Vec<[f32; 4]> = (0..6)
            .map(|index| index as f32 * PI / 3.0)
            .map(|angle| [radius * angle.cos(), 0.0, radius * angle.sin(), 1.0])
            .collect();
        let triangles = [
            origin, point[0], point[1],
            origin, point[1], point[2],
            origin, point[2], point[3],
            origin, point[3], point[4],
            origin, point[4], point[5],
            origin, point[5], point[0],
        ];
        triangles.map(|position| SurfaceVertex { position })
    }

    const ATTRIBUTES: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0=>Float32x4, 1=>Float32x4];
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<SurfaceVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

