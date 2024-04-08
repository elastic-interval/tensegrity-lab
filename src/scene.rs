use std::f32::consts::PI;
use std::mem;

use bytemuck::{cast_slice, Pod, Zeroable};
use leptos::{ReadSignal, SignalGet, SignalUpdate, WriteSignal};
use wgpu::util::DeviceExt;
use winit::keyboard::KeyCode;
use winit_input_helper::WinitInputHelper;

use crate::camera::{Camera, Pick};
use crate::camera::Target::*;
use crate::control_state::{ControlState, IntervalDetails};
use crate::fabric::{Fabric, MATERIALS, UniqueId};
use crate::fabric::interval::{Interval, Role, Span};
use crate::graphics::Graphics;

const MAX_INTERVALS: usize = 5000;

#[derive(Debug, Clone)]
pub struct StrainRendering {
    _threshold: f32,
    _material: usize,
}

pub struct Scene {
    _strain_rendering: Option<StrainRendering>,
    camera: Camera,
    fabric_drawing: Drawing<FabricVertex>,
    surface_drawing: Drawing<SurfaceVertex>,
    uniform_bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    graphics: Graphics,
    control_state: ReadSignal<ControlState>,
    set_control_state: WriteSignal<ControlState>,
}

impl Scene {
    pub fn new(graphics: Graphics, (control_state, set_control_state): (ReadSignal<ControlState>, WriteSignal<ControlState>)) -> Self {
        let shader = graphics
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            });
        let scale = 6.0;
        let camera = Camera::new(
            (2.0 * scale, 1.0 * scale, 2.0 * scale).into(),
            graphics.config.width as f32,
            graphics.config.height as f32,
        );
        let uniform_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("MVP"),
                    contents: cast_slice(&[0.0f32; 16]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
        let uniform_bind_group_layout =
            graphics
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Uniform Bind Group Layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });
        let uniform_bind_group = graphics
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &uniform_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }],
                label: Some("Uniform Bind Group"),
            });
        let pipeline_layout =
            graphics
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[&uniform_bind_group_layout],
                    push_constant_ranges: &[],
                });
        let fabric_vertices = vec![FabricVertex::default(); MAX_INTERVALS * 2];
        let fabric_pipeline =
            graphics
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::LineList,
                        strip_index_format: None,
                        ..Default::default()
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                });
        let fabric_buffer = graphics
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: cast_slice(&fabric_vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        let surface_vertices = SurfaceVertex::for_radius(10.0).to_vec();
        let surface_pipeline =
            graphics
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        ..Default::default()
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                });
        let surface_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Surface Buffer"),
                    contents: cast_slice(&surface_vertices),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });
        Self {
            _strain_rendering: None,
            camera,
            graphics,
            fabric_drawing: Drawing {
                vertices: fabric_vertices,
                pipeline: fabric_pipeline,
                buffer: fabric_buffer,
            },
            surface_drawing: Drawing {
                vertices: surface_vertices,
                pipeline: surface_pipeline,
                buffer: surface_buffer,
            },
            uniform_buffer,
            uniform_bind_group,
            control_state,
            set_control_state,
        }
    }

    pub fn render(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
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
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);

        render_pass.set_pipeline(&self.fabric_drawing.pipeline);
        render_pass.set_vertex_buffer(0, self.fabric_drawing.buffer.slice(..));
        render_pass.draw(0..self.fabric_drawing.vertices.len() as u32, 0..1);

        render_pass.set_pipeline(&self.surface_drawing.pipeline);
        render_pass.set_vertex_buffer(0, self.surface_drawing.buffer.slice(..));
        render_pass.draw(0..self.surface_drawing.vertices.len() as u32, 0..1);
    }

    pub fn handle_input(&mut self, input: &WinitInputHelper, fabric: &Fabric) {
        if input.key_pressed(KeyCode::Escape) {
            match self.camera.pick {
                Pick::Nothing => {
                    match self.control_state.get() {
                        ControlState::Choosing =>
                            self.set_control_state.update(move |state| *state = ControlState::Viewing),
                        ControlState::Viewing =>
                            self.set_control_state.update(move |state| *state = ControlState::Choosing),
                        _ => {}
                    };
                }
                Pick::Joint(_) => self.do_pick(Pick::Nothing),
                Pick::Interval { joint, .. } => self.do_pick(Pick::Joint(joint)),
            }
        } else if let Some(pick) = self.camera.handle_input(input, fabric) {
            if !fabric.progress.is_busy() {
                self.do_pick(pick)
            }
        }
    }

    pub fn selection_active(&self) -> bool {
        !matches!(self.camera.pick, Pick::Nothing)
    }

    pub fn update(&mut self, fabric: &Fabric) {
        self.update_from_fabric(fabric);
        self.update_from_camera(&self.graphics);
        self.graphics.queue.write_buffer(
            &self.fabric_drawing.buffer,
            0,
            cast_slice(&self.fabric_drawing.vertices),
        );
    }

    fn update_from_fabric(&mut self, fabric: &Fabric) {
        self.fabric_drawing.vertices.clear();
        self.fabric_drawing
            .vertices
            .extend(fabric
                .intervals
                .iter()
                .flat_map(|(interval_id, interval)|
                    FabricVertex::for_interval(interval_id, interval, fabric, &self.camera.pick))
            );
        self.camera.target_approach(fabric);
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.graphics.queue
    }

    pub fn create_encoder(&self) -> wgpu::CommandEncoder {
        self.graphics
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Encoder"),
            })
    }

    pub fn surface_texture(&self) -> Result<wgpu::SurfaceTexture, wgpu::SurfaceError> {
        self.graphics.surface.get_current_texture()
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.graphics.config.width = width;
        self.graphics.config.height = height;
        self.graphics
            .surface
            .configure(&self.graphics.device, &self.graphics.config);
        self.camera.set_size(width as f32, height as f32);
    }

    fn update_from_camera(&self, graphics: &Graphics) {
        let mvp_mat = self.camera.mvp_matrix();
        let mvp_ref: &[f32; 16] = mvp_mat.as_ref();
        graphics
            .queue
            .write_buffer(&self.uniform_buffer, 0, cast_slice(mvp_ref));
    }

    pub fn do_pick(&mut self, pick: Pick) {
        match pick {
            Pick::Nothing => {
                self.camera.target = FabricMidpoint;
                self.set_control_state.update(|state| *state = ControlState::Viewing);
            }
            Pick::Joint(joint_index) => {
                self.camera.target = AroundJoint(joint_index);
                self.set_control_state.update(|state| *state = ControlState::ShowingJoint(joint_index));
            }
            Pick::Interval { joint, id, interval } => {
                self.camera.target = AroundInterval(id);
                let role = MATERIALS[interval.material].role;
                let length = match interval.span {
                    Span::Fixed { length } => length,
                    _ => 0.0
                };
                let alpha_index = if interval.alpha_index == joint { interval.alpha_index } else { interval.omega_index };
                let omega_index = if interval.omega_index == joint { interval.alpha_index } else { interval.omega_index };
                let interval_details = IntervalDetails { alpha_index, omega_index, length, role };
                self.set_control_state.update(|state| *state = ControlState::ShowingInterval(interval_details));
            }
        }
        self.camera.pick = pick;
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Default)]
struct FabricVertex {
    position: [f32; 4],
    color: [f32; 4],
}

const GRAY: [f32; 4] = [0.1, 0.1, 0.1, 0.5];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 0.5];
const SELECTED: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const RED_END: [f32; 4] = [1.0, 0.0, 0.0, 0.2];
const BLUE: [f32; 4] = [0.5, 0.5, 1.0, 1.0];
const BLUE_END: [f32; 4] = [0.5, 0.5, 1.0, 0.2];

impl FabricVertex {
    pub fn for_interval(interval_id: &UniqueId, interval: &Interval, fabric: &Fabric, pick: &Pick) -> [FabricVertex; 4] {
        let (alpha, omega) = interval.locations(&fabric.joints);
        let midpoint = interval.midpoint(&fabric.joints);
        let (center_color, end_color) = match fabric.materials[interval.material].role {
            Role::Push => (RED, RED_END),
            Role::Pull => (BLUE, BLUE_END),
            Role::Spring => (WHITE, WHITE),
        };
        let (center_color, end_color) = match pick {
            Pick::Nothing => {
                (center_color, end_color)
            }
            Pick::Joint(joint_index) => {
                if interval.touches(*joint_index) {
                    (center_color, end_color)
                } else {
                    (GRAY, GRAY)
                }
            }
            Pick::Interval { joint, id, .. } => {
                if *id == *interval_id {
                    (SELECTED, SELECTED)
                } else if interval.touches(*joint) {
                    (center_color, end_color)
                } else {
                    (GRAY, GRAY)
                }
            }
        };
        [
            FabricVertex {
                position: [alpha.x, alpha.y, alpha.z, 1.0],
                color: end_color,
            },
            FabricVertex {
                position: [midpoint.x, midpoint.y, midpoint.z, 1.0],
                color: center_color,
            },
            FabricVertex {
                position: [midpoint.x, midpoint.y, midpoint.z, 1.0],
                color: center_color,
            },
            FabricVertex {
                position: [omega.x, omega.y, omega.z, 1.0],
                color: end_color,
            },
        ]
    }

    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0=>Float32x4, 1=>Float32x4];
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
            origin, point[0], point[1], origin, point[1], point[2], origin, point[2], point[3],
            origin, point[3], point[4], origin, point[4], point[5], origin, point[5], point[0],
        ];
        triangles.map(|position| SurfaceVertex { position })
    }

    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0=>Float32x4, 1=>Float32x4];
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<SurfaceVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

struct Drawing<V> {
    vertices: Vec<V>,
    pipeline: wgpu::RenderPipeline,
    buffer: wgpu::Buffer,
}
