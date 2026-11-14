use crate::camera::{Pick, PREVIEW_SECS};
use crate::fabric::interval::Interval;
use crate::fabric::Fabric;
use crate::units::Unit;
use crate::wgpu::connector_renderer::{part_instance_layout, PartInstance};
use crate::wgpu::cylinder_renderer::{cylinder_instance_layout, physical_radius, CylinderInstance};
use crate::wgpu::Wgpu;
use crate::{IntervalDetails, JointDetails};
use wgpu::util::DeviceExt;

const SELECTED_COLOR: [f32; 4] = [0.4, 0.6, 1.0, 0.35];
const PREVIEW_COLOR: [f32; 4] = [1.0, 0.7, 0.2, 0.35];
// On a light background, deeper versions of the same.
const SELECTED_COLOR_ON_LIGHT: [f32; 4] = [0.0, 0.04, 0.25, 0.5];
const PREVIEW_COLOR_ON_LIGHT: [f32; 4] = [0.35, 0.12, 0.0, 0.5];
// A click's preview comes up this quickly, then fades over `PREVIEW_SECS`.
const PREVIEW_RISE_SECS: f32 = 0.15;
// In strut radii: a tube stands this far off its member's surface, so it
// hugs a cable as closely as a strut.
const TUBE_GAP: f32 = 0.8;
const SPHERE_RADIUS: f32 = 3.0;

struct Mesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
}

/// What is selected, shown by see-through shapes around it rather than by
/// recolouring the fabric: a tube around the interval, and a sphere at the
/// joint the next click continues from. What a click where the pointer rests
/// would select appears in another colour, then fades.
pub struct HighlightRenderer {
    tube: Mesh,
    sphere: Mesh,
    tube_pipeline: wgpu::RenderPipeline,
    sphere_pipeline: wgpu::RenderPipeline,
    tubes: Option<(wgpu::Buffer, u32)>,
    spheres: Option<(wgpu::Buffer, u32)>,
}

impl HighlightRenderer {
    pub fn new(wgpu: &Wgpu) -> Self {
        let (vertices, indices, count) = wgpu.create_cylinder(24);
        let tube = Mesh { vertices, indices, count };
        let (vertices, indices, count) = wgpu.create_sphere(24);
        let sphere = Mesh { vertices, indices, count };
        Self {
            tube,
            sphere,
            tube_pipeline: wgpu.create_transparent_pipeline(
                "Highlight Tube Pipeline",
                cylinder_instance_layout(),
                "fabric_vertex",
            ),
            sphere_pipeline: wgpu.create_transparent_pipeline(
                "Highlight Sphere Pipeline",
                part_instance_layout(),
                "plate_vertex",
            ),
            tubes: None,
            spheres: None,
        }
    }

    pub fn update(
        &mut self,
        wgpu: &Wgpu,
        fabric: &Fabric,
        pick: &Pick,
        preview: Option<(&Pick, f32)>,
        light_background: bool,
    ) {
        let (selected_color, preview_color) = if light_background {
            (SELECTED_COLOR_ON_LIGHT, PREVIEW_COLOR_ON_LIGHT)
        } else {
            (SELECTED_COLOR, PREVIEW_COLOR)
        };
        let mut tubes = Vec::new();
        let mut spheres = Vec::new();
        add_shapes(fabric, pick, selected_color, true, &mut tubes, &mut spheres);
        if let Some((preview, age)) = preview {
            let rise = (age / PREVIEW_RISE_SECS).min(1.0);
            let fade = (1.0 - age / PREVIEW_SECS).max(0.0);
            let [r, g, b, a] = preview_color;
            // A preview interval continues from the selected joint, whose sphere is already shown.
            add_shapes(fabric, preview, [r, g, b, a * rise * fade], false, &mut tubes, &mut spheres);
        }
        self.tubes = instance_buffer(wgpu, &tubes);
        self.spheres = instance_buffer(wgpu, &spheres);
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, bind_group: &'a wgpu::BindGroup) {
        let drawn = [
            (&self.tube_pipeline, &self.tube, &self.tubes),
            (&self.sphere_pipeline, &self.sphere, &self.spheres),
        ];
        for (pipeline, mesh, instances) in drawn {
            let Some((instances, count)) = instances else { continue };
            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, bind_group, &[]);
            render_pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            render_pass.set_vertex_buffer(1, instances.slice(..));
            render_pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..mesh.count, 0, 0..*count);
        }
    }
}

/// The bubble around a highlighted joint.
pub fn sphere_radius(fabric: &Fabric) -> f32 {
    fabric.dimensions.push_radius.f32() * SPHERE_RADIUS
}

/// The tube around a highlighted interval, standing off its surface.
pub fn tube_radius(fabric: &Fabric, interval: &Interval) -> f32 {
    physical_radius(interval.role, &fabric.dimensions) + fabric.dimensions.push_radius.f32() * TUBE_GAP
}

fn add_shapes(
    fabric: &Fabric,
    pick: &Pick,
    color: [f32; 4],
    near_sphere: bool,
    tubes: &mut Vec<CylinderInstance>,
    spheres: &mut Vec<PartInstance>,
) {
    let (joint, interval) = match pick {
        Pick::Nothing => (None, None),
        Pick::Joint(JointDetails { key, .. }) => (Some(*key), None),
        Pick::Interval(IntervalDetails { key, near_joint, .. }) => {
            (near_sphere.then_some(*near_joint), fabric.intervals.get(*key))
        }
    };
    if let Some(interval) = interval {
        let start = fabric.joints[interval.alpha_key].location;
        let end = fabric.joints[interval.omega_key].location;
        tubes.push(CylinderInstance {
            start: start.into(),
            radius_factor: tube_radius(fabric, interval),
            end: end.into(),
            material_type: 0,
            color,
        });
    }
    if let Some(key) = joint {
        spheres.push(PartInstance::sphere(fabric.location(key), sphere_radius(fabric), color));
    }
}

fn instance_buffer<T: bytemuck::Pod>(wgpu: &Wgpu, instances: &[T]) -> Option<(wgpu::Buffer, u32)> {
    if instances.is_empty() {
        return None;
    }
    let buffer = wgpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Highlight Instance Buffer"),
        contents: bytemuck::cast_slice(instances),
        usage: wgpu::BufferUsages::VERTEX,
    });
    Some((buffer, instances.len() as u32))
}
