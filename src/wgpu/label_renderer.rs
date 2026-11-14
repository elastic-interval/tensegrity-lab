use crate::camera::Pick;
use crate::fabric::Fabric;
use crate::wgpu::highlight_renderer::{sphere_radius, tube_radius};
use crate::wgpu::text_renderer::TextRenderer;
use crate::wgpu::{default_depth_stencil_state, Wgpu};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2, Vec3};
use wgpu::util::DeviceExt;
use wgpu_text::glyph_brush::{HorizontalAlign, Layout, OwnedSection, OwnedText, VerticalAlign};

// In physical pixels.
const FONT_SIZE: f32 = 36.0;
const PADDING: Vec2 = Vec2::new(16.0, 10.0);
const CORNER_RADIUS: f32 = 12.0;
const LEADER_LENGTH: f32 = 70.0;
const LEADER_WIDTH: f32 = 3.0;
// The box's border, in the leader's colour and width.
const BORDER_WIDTH: f32 = LEADER_WIDTH;
const LABEL_GAP: f32 = 10.0;

struct Palette {
    background: [f32; 4],
    text: [f32; 4],
    leader: [f32; 4],
}

const ON_DARK: Palette = Palette {
    background: [0.02, 0.02, 0.025, 0.92],
    text: [0.85, 0.85, 0.85, 1.0],
    leader: [0.6, 0.6, 0.6, 1.0],
};

const ON_LIGHT: Palette = Palette {
    background: [0.92, 0.92, 0.9, 0.92],
    text: [0.0, 0.0, 0.0, 1.0],
    leader: [0.15, 0.15, 0.15, 1.0],
};

/// A rounded rectangle on screen, in pixels.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct BoxVertex {
    position: [f32; 2],
    local: [f32; 2],
    half_size: [f32; 2],
    radius: f32,
    color: [f32; 4],
}

const BOX_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
    0 => Float32x2,
    1 => Float32x2,
    2 => Float32x2,
    3 => Float32,
    4 => Float32x4,
];

/// Labels floating upright above the selected joint and interval, each in a
/// rounded box with a line down to what it names: just what building needs.
pub struct LabelRenderer {
    pipeline: wgpu::RenderPipeline,
    vertices: Option<(wgpu::Buffer, u32)>,
}

impl LabelRenderer {
    pub fn new(wgpu: &Wgpu) -> Self {
        let layout = wgpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Label Pipeline Layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });
        let pipeline = wgpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: None,
            label: Some("Label Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                module: &wgpu.shader,
                entry_point: Some("label_vertex"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<BoxVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &BOX_VERTEX_ATTRIBUTES,
                }],
            },
            fragment: Some(wgpu::FragmentState {
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: wgpu.gamma_constants(),
                    ..Default::default()
                },
                module: &wgpu.shader,
                entry_point: Some("label_fragment"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu.surface_configuration.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                ..default_depth_stencil_state()
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
        });
        Self { pipeline, vertices: None }
    }

    /// Lay out this frame's labels: their boxes here, their text in the
    /// text renderer, which draws after.
    pub fn update(
        &mut self,
        wgpu: &Wgpu,
        text_renderer: &mut TextRenderer,
        fabric: &Fabric,
        pick: &Pick,
        mvp: Mat4,
        screen_up: Vec3,
        scale: f32,
        light_background: bool,
    ) {
        let palette = if light_background { &ON_LIGHT } else { &ON_DARK };
        let (width, height) = wgpu.size();
        let mut placed: Vec<(Vec2, Vec2)> = Vec::new();
        let mut vertices = Vec::new();
        let mut sections = Vec::new();
        for (anchor, radius, text) in labels(fabric, pick, scale) {
            // The line ends on top of the highlight, not at its centre.
            let Some(point) = to_screen(mvp, anchor + screen_up * radius, width, height) else {
                continue;
            };
            let mut bottom = point.y - LEADER_LENGTH;
            let section = |bottom: f32| {
                OwnedSection::default()
                    .with_layout(
                        Layout::default()
                            .h_align(HorizontalAlign::Center)
                            .v_align(VerticalAlign::Bottom),
                    )
                    .with_screen_position((point.x, bottom - BORDER_WIDTH - PADDING.y))
                    .add_text(OwnedText::new(text.clone()).with_scale(FONT_SIZE).with_color(palette.text))
            };
            let Some(bounds) = text_renderer.glyph_bounds(&section(bottom)) else {
                continue;
            };
            let half = Vec2::new(bounds.width(), bounds.height()) / 2.0 + PADDING;
            let outer = half + Vec2::splat(BORDER_WIDTH);
            // Above any label already placed that it would overlap.
            for (center, other_half) in &placed {
                let top = bottom - 2.0 * outer.y;
                let overlaps_x = (point.x - center.x).abs() < outer.x + other_half.x;
                let overlaps_y = top < center.y + other_half.y && bottom > center.y - other_half.y;
                if overlaps_x && overlaps_y {
                    bottom = center.y - other_half.y - LABEL_GAP;
                }
            }
            let center = Vec2::new(point.x, bottom - outer.y);
            let leader_half = Vec2::new(LEADER_WIDTH / 2.0, (point.y - bottom) / 2.0);
            let leader_center = Vec2::new(point.x, (point.y + bottom) / 2.0);
            push_box(&mut vertices, leader_center, leader_half, 0.0, palette.leader, width, height);
            let border = Vec2::splat(BORDER_WIDTH);
            push_box(&mut vertices, center, half + border, CORNER_RADIUS + BORDER_WIDTH, palette.leader, width, height);
            push_box(&mut vertices, center, half, CORNER_RADIUS, palette.background, width, height);
            sections.push(section(bottom));
            placed.push((center, outer));
        }
        text_renderer.set_labels(sections);
        self.vertices = (!vertices.is_empty()).then(|| {
            let buffer = wgpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Label Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            (buffer, vertices.len() as u32)
        });
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        let Some((vertices, count)) = &self.vertices else {
            return;
        };
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, vertices.slice(..));
        render_pass.draw(0..*count, 0..1);
    }
}

/// What to label: the selected joint, or the selected interval and the
/// joint the next click continues from, each with its highlight's centre
/// and radius.
fn labels(fabric: &Fabric, pick: &Pick, scale: f32) -> Vec<(Vec3, f32, String)> {
    let bubble = sphere_radius(fabric);
    match pick {
        Pick::Nothing => vec![],
        Pick::Joint(details) => vec![(fabric.location(details.key), bubble, details.joint_text())],
        Pick::Interval(details) => {
            let Some(interval) = fabric.intervals.get(details.key) else {
                return vec![];
            };
            vec![
                (fabric.location(details.near_joint), bubble, details.near_joint_label.clone()),
                (
                    interval.midpoint(&fabric.joints),
                    tube_radius(fabric, interval),
                    details.label_text(scale),
                ),
            ]
        }
    }
}

fn to_screen(mvp: Mat4, point: Vec3, width: f32, height: f32) -> Option<Vec2> {
    let clip = mvp * point.extend(1.0);
    if clip.w <= 0.0 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    Some(Vec2::new((ndc.x + 1.0) / 2.0 * width, (1.0 - ndc.y) / 2.0 * height))
}

fn push_box(
    vertices: &mut Vec<BoxVertex>,
    center: Vec2,
    half: Vec2,
    radius: f32,
    color: [f32; 4],
    width: f32,
    height: f32,
) {
    let corner = |sx: f32, sy: f32| {
        let local = Vec2::new(sx * half.x, sy * half.y);
        let pixel = center + local;
        BoxVertex {
            position: [pixel.x / width * 2.0 - 1.0, 1.0 - pixel.y / height * 2.0],
            local: local.into(),
            half_size: half.into(),
            radius,
            color,
        }
    };
    let [a, b, c, d] = [corner(-1.0, -1.0), corner(1.0, -1.0), corner(1.0, 1.0), corner(-1.0, 1.0)];
    vertices.extend_from_slice(&[a, b, c, a, c, d]);
}
