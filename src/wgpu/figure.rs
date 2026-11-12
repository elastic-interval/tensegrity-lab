//! Figures for the handoff package: the fabric from four sides and each part
//! on its own, drawn by the app's renderers into images instead of a window,
//! on white, as PNG. Each is drawn at twice its size and scaled down, which
//! smooths the edges.

use crate::camera::Pick;
use crate::connector::parts::{parts_list, placements, Part};
use crate::fabric::interval::Role;
use crate::fabric::Fabric;
use crate::units::Unit;
use crate::wgpu::connector_renderer::part_instance_layout;
use crate::wgpu::fabric_renderer::FabricRenderer;
use crate::wgpu::part_meshes::PartSizes;
use crate::wgpu::parts_view::{camera_matrix, create_camera, ShownPart, AZIMUTH, ELEVATION, FILL};
use crate::wgpu::Wgpu;
use crate::RenderStyle;
use glam::{Mat4, Vec3};
use image::imageops::{resize, FilterType};
use image::RgbaImage;
use std::path::Path;

const WHITE: wgpu::Color = wgpu::Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
const SUPERSAMPLE: u32 = 2;
const VIEW_SIZE: (u32, u32) = (800, 600);
const PART_SIZE: u32 = 500;
const PORTRAIT_SIZE: (u32, u32) = (1200, 900);
/// How much of the image the fabric fills, at most, either way.
const VIEW_FILL: f32 = 0.92;

/// A view of the whole fabric: its file, and where the camera looks from and
/// which way is up on the page, in the simulation's frame (Y up).
pub struct View {
    pub file: &'static str,
    toward: Vec3,
    up: Vec3,
}

/// The README's pictures: three-quarters from the front, a little above.
const PORTRAIT: View = View { file: "", toward: Vec3::new(0.5, 0.2, 0.84), up: Vec3::Y };

pub const VIEWS: [View; 4] = [
    View { file: "3d.png", toward: Vec3::new(0.55, 0.35, 0.76), up: Vec3::Y },
    View { file: "voor.png", toward: Vec3::Z, up: Vec3::Y },
    View { file: "zij.png", toward: Vec3::X, up: Vec3::Y },
    View { file: "boven.png", toward: Vec3::Y, up: Vec3::NEG_Z },
];

/// A part's figure: its file, its letter as the P view shows it, the part,
/// and how many there are.
pub struct PartFigure {
    pub file: String,
    pub letter: char,
    pub part: Part,
    pub count: usize,
}

/// Writes the views and one figure per part on the parts list into `folder`,
/// returning the parts' figures in the order of the list.
pub fn write_figures(fabric: &Fabric, folder: &Path) -> Vec<PartFigure> {
    std::fs::create_dir_all(folder).expect("create figure folder");
    let (width, height) = VIEW_SIZE;
    let wgpu = Wgpu::headless(width * SUPERSAMPLE, height * SUPERSAMPLE);
    let mut renderer = FabricRenderer::new(&wgpu);
    renderer.update(&wgpu, fabric, &Pick::Nothing, &RenderStyle::Normal, true);
    let joints: Vec<Vec3> = fabric.joints.values().map(|joint| joint.location).collect();
    for view in &VIEWS {
        wgpu.update_mvp_matrix(view_matrix(fabric, &joints, view, width as f32 / height as f32));
        save(draw_fabric(&wgpu, &renderer), width, height, &folder.join(view.file));
    }

    let Some(connector) = fabric.connector.as_ref() else {
        return Vec::new();
    };
    let side = PART_SIZE * SUPERSAMPLE;
    let wgpu = Wgpu::headless(side, side);
    let sizes = PartSizes::of(&fabric.dimensions, &connector.dimensions);
    let ring_radius = fabric.dimensions.push_radius.f32();
    let pipeline = wgpu.create_plate_pipeline("Figure Part Pipeline", part_instance_layout());
    let (camera, bind_group) = create_camera(&wgpu);
    parts_list(&placements(fabric))
        .into_iter()
        .enumerate()
        .map(|(index, (part, count))| {
            let shown = ShownPart::new(&wgpu, part, count, part.mass(ring_radius, &connector.dimensions), sizes);
            let matrix = camera_matrix(shown.center, shown.radius, AZIMUTH, ELEVATION, 1.0, FILL);
            wgpu.queue.write_buffer(&camera, 0, bytemuck::cast_slice(&matrix.to_cols_array()));
            let mut frame = Frame::new(&wgpu);
            {
                let mut pass = frame.pass();
                pass.set_pipeline(&pipeline);
                shown.draw(&mut pass, &bind_group);
            }
            let image = frame.read(&wgpu);
            let letter = (b'A' + index as u8) as char;
            let file = format!("onderdeel-{letter}-{}.png", slug(&part.name()));
            save(image, PART_SIZE, PART_SIZE, &folder.join(&file));
            PartFigure { file, letter, part, count }
        })
        .collect()
}

/// A README picture of `fabric`, framed on its struts, so guy lines run
/// out of the picture to their anchors.
pub fn write_portrait(fabric: &Fabric, path: &Path) {
    let (width, height) = PORTRAIT_SIZE;
    let wgpu = Wgpu::headless(width * SUPERSAMPLE, height * SUPERSAMPLE);
    let mut renderer = FabricRenderer::new(&wgpu);
    renderer.update(&wgpu, fabric, &Pick::Nothing, &RenderStyle::Normal, true);
    let struts: Vec<Vec3> = fabric
        .intervals
        .values()
        .filter(|interval| interval.has_role(Role::Pushing))
        .flat_map(|interval| [interval.alpha_key, interval.omega_key])
        .map(|joint| fabric.joints[joint].location)
        .collect();
    wgpu.update_mvp_matrix(view_matrix(fabric, &struts, &PORTRAIT, width as f32 / height as f32));
    save(draw_fabric(&wgpu, &renderer), width, height, path);
}

fn draw_fabric(wgpu: &Wgpu, renderer: &FabricRenderer) -> RgbaImage {
    let mut frame = Frame::new(wgpu);
    {
        let mut pass = frame.pass();
        wgpu.set_bind_group(&mut pass);
        renderer.render(&mut pass, &wgpu.uniform_bind_group, true);
    }
    frame.read(wgpu)
}

/// Lower case, letters and digits, the rest one dash.
fn slug(name: &str) -> String {
    let mut slug = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_matches('-').to_string()
}

/// Looking at `fabric` along `view`, every one of `joints` within the image.
fn view_matrix(fabric: &Fabric, joints: &[Vec3], view: &View, aspect: f32) -> Mat4 {
    let (low, high) = joints.iter().fold((Vec3::MAX, Vec3::MIN), |(low, high), &p| (low.min(p), high.max(p)));
    let center = (low + high) / 2.0;
    let toward = view.toward.normalize();
    let right = view.up.cross(toward).normalize();
    let up = toward.cross(right);
    let (half_width, half_height) = joints.iter().fold((0.0f32, 0.0f32), |(w, h), &p| {
        let offset = p - center;
        (w.max(offset.dot(right).abs()), h.max(offset.dot(up).abs()))
    });
    let half_height = (half_height.max(half_width / aspect) + fabric.dimensions.push_radius.f32()) / VIEW_FILL;
    let half_width = half_height * aspect;
    let distance = fabric.bounding_radius() * 4.0 + 1.0;
    Mat4::orthographic_rh(-half_width, half_width, -half_height, half_height, 0.0, distance * 2.0)
        * Mat4::look_at_rh(center + toward * distance, center, up)
}

/// A frame being drawn on white, to read back as an image.
struct Frame {
    texture: wgpu::Texture,
    target: wgpu::TextureView,
    depth: wgpu::TextureView,
    encoder: wgpu::CommandEncoder,
}

impl Frame {
    fn new(wgpu: &Wgpu) -> Self {
        let (width, height) = wgpu.size();
        let texture = wgpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Figure"),
            size: wgpu::Extent3d { width: width as u32, height: height as u32, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let target = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Frame { texture, target, depth: wgpu.create_depth_view(), encoder: wgpu.create_encoder() }
    }

    fn pass(&mut self) -> wgpu::RenderPass<'_> {
        self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Figure Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.target,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(WHITE), store: wgpu::StoreOp::Store },
                depth_slice: None,
            })],
            multiview_mask: None,
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth,
                depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }

    fn read(self, wgpu: &Wgpu) -> RgbaImage {
        let Frame { texture, mut encoder, .. } = self;
        let size = texture.size();
        let (width, height) = (size.width, size.height);
        let row = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = wgpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Figure Readback"),
            size: (row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(height) },
            },
            size,
        );
        wgpu.queue.submit(std::iter::once(encoder.finish()));
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |result| result.expect("map figure"));
        wgpu.device.poll(wgpu::PollType::wait_indefinitely()).expect("poll");
        let mapped = slice.get_mapped_range();
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for line in mapped.chunks(row as usize) {
            pixels.extend_from_slice(&line[..(width * 4) as usize]);
        }
        RgbaImage::from_raw(width, height, pixels).expect("figure pixels")
    }
}

fn save(image: RgbaImage, width: u32, height: u32, path: &Path) {
    resize(&image, width, height, FilterType::Lanczos3).save(path).expect("save figure");
}

#[cfg(test)]
mod tests {
    use crate::build::dsl::fabric_library::{self, FabricName};
    use crate::build::dsl::fabric_plan_executor::FabricPlanExecutor;
    use std::path::Path;

    /// Redraws the README's pictures, `headless-hug.png` and `open-claw.png`:
    ///
    /// ```
    /// cargo test --release --lib export_readme_images -- --ignored
    /// ```
    #[test]
    #[ignore]
    fn export_readme_images() {
        for (name, file) in [(FabricName::HeadlessHug, "headless-hug.png"), (FabricName::OpenClaw, "open-claw.png")] {
            let mut executor = FabricPlanExecutor::new(fabric_library::get_fabric_plan(name));
            while !executor.is_complete() {
                let _ = executor.iterate();
            }
            executor.fabric.update_all_attachment_connections();
            let struts = executor.fabric.intervals.values().filter(|i| i.has_role(crate::fabric::interval::Role::Pushing)).count();
            println!("{file}: {struts} struts");
            super::write_portrait(&executor.fabric, Path::new(file));
        }
    }
}
