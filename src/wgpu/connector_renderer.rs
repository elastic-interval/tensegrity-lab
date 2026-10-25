/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::camera::Pick;
use crate::connector::parts::{kinds, parts_list, placements, KindSummary, Material, Part, Placement};
use crate::fabric::Fabric;
use crate::units::Unit;
use crate::wgpu::part_meshes::{PartMesh, PartSizes};
use crate::wgpu::parts_panel::PanelRow;
use crate::wgpu::Wgpu;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use instant::Instant;
use std::collections::BTreeMap;
use std::mem::size_of;
use wgpu::util::DeviceExt;

// Materials (docs/connectors.md, "Materials"): the fabricated connector is
// hot-dip galvanized steel, node plates too; terminals are bought
// AISI-316 stainless. Our shader is diffuse-only, so the stainless base
// colour is lifted above the table's PBR value to stand in for its shine —
// the real contrast is roughness, not brightness. Parts of assemblies with
// a construction problem are painted red instead.
const GALVANIZED_COLOR: [f32; 4] = [0.60, 0.62, 0.64, 1.0];
const STAINLESS_COLOR: [f32; 4] = [0.72, 0.73, 0.75, 1.0];
const OVERLAP_COLOR: [f32; 4] = [0.90, 0.12, 0.12, 1.0];
const POM_COLOR: [f32; 4] = [0.93, 0.92, 0.88, 1.0];
// The parts panel's renderings turn this fast, in radians per second.
const THUMBNAIL_TURN_RATE: f32 = 0.5;
// How much of its square a part's bounding sphere fills.
const THUMBNAIL_FILL: f32 = 0.85;

fn material_color(material: Material) -> [f32; 4] {
    match material {
        Material::Galvanized => GALVANIZED_COLOR,
        Material::Stainless => STAINLESS_COLOR,
        Material::Pom => POM_COLOR,
    }
}

/// One copy of a part on the plate pipeline at unit scale: its frame's
/// origin, `y` axis and `x` axis (see `Part`).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PartInstance {
    center_radius: [f32; 4],
    axis_thickness: [f32; 4],
    boss_dir: [f32; 4],
    color: [f32; 4],
}

impl PartInstance {
    fn new(placement: &Placement) -> Self {
        let (o, x, y) = (placement.origin, placement.x, placement.y);
        let color = if placement.culprit {
            OVERLAP_COLOR
        } else {
            material_color(placement.part.material())
        };
        PartInstance {
            center_radius: [o.x, o.y, o.z, 1.0],
            axis_thickness: [y.x, y.y, y.z, 1.0],
            boss_dir: [x.x, x.y, x.z, 0.0],
            color,
        }
    }

    /// `part` turned by `angle` about the vertical, its bounding sphere
    /// centred at the origin and scaled to fill a panel square.
    fn thumbnail(part: Part, mesh: &PartMesh, angle: f32) -> Self {
        let x = Vec3::new(angle.cos(), 0.0, -angle.sin());
        let y = Vec3::Y;
        let z = x.cross(y);
        let scale = THUMBNAIL_FILL / mesh.radius.max(1e-6);
        let c = mesh.center;
        let o = -(x * c.x + y * c.y + z * c.z) * scale;
        PartInstance {
            center_radius: [o.x, o.y, o.z, scale],
            axis_thickness: [y.x, y.y, y.z, scale],
            boss_dir: [x.x, x.y, x.z, 0.0],
            color: material_color(part.material()),
        }
    }
}

/// A part's mesh and the copies of it to draw.
struct PartBuffers {
    mesh: PartMesh,
    instances: Option<wgpu::Buffer>,
    instance_count: u32,
}

/// Draws the connector hardware from the parts list: one mesh per part,
/// built when first needed, instanced at every placement.
pub struct ConnectorRenderer {
    pipeline: wgpu::RenderPipeline,
    // Meshes are rebuilt when the fabric's ring or cap sizes change.
    sizes: Option<PartSizes>,
    parts: BTreeMap<Part, PartBuffers>,
    /// The parts list by kind, as of the last update.
    kinds: Vec<KindSummary>,
    /// The parts panel's renderings: a fixed camera of their own, and one
    /// turning instance per kind.
    thumbnail_bind_group: wgpu::BindGroup,
    thumbnail_instances: Option<wgpu::Buffer>,
    started: Instant,
}

impl ConnectorRenderer {
    pub fn new(wgpu: &Wgpu) -> Self {
        let instance_layout = wgpu::VertexBufferLayout {
            array_stride: size_of::<PartInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        };
        // Looking down a little at the origin, the unit sphere filling the
        // view: each part sits there, scaled to fit, in its own viewport.
        let view = Mat4::look_at_rh(Vec3::new(0.0, 1.2, 2.6), Vec3::ZERO, Vec3::Y);
        let projection = Mat4::orthographic_rh(-1.0, 1.0, -1.0, 1.0, 0.1, 10.0);
        let thumbnail_camera = wgpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Parts Panel Camera"),
            contents: bytemuck::cast_slice(&(projection * view).to_cols_array()),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let thumbnail_bind_group = wgpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Parts Panel Bind Group"),
            layout: &wgpu.uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: thumbnail_camera.as_entire_binding(),
            }],
        });
        ConnectorRenderer {
            pipeline: wgpu.create_plate_pipeline("Connector Part Pipeline", instance_layout),
            sizes: None,
            parts: BTreeMap::new(),
            kinds: Vec::new(),
            thumbnail_bind_group,
            thumbnail_instances: None,
            started: Instant::now(),
        }
    }

    pub fn update(&mut self, wgpu: &Wgpu, fabric: &Fabric, _pick: &Pick) {
        let Some(connector) = fabric.connector.as_ref() else {
            self.parts.values_mut().for_each(|buffers| buffers.instance_count = 0);
            return;
        };
        let sizes = PartSizes {
            ring_radius: fabric.dimensions.push_radius.f32(),
            pivot_radius: connector.dimensions.pivot_radius.f32(),
            ring_thickness: connector.dimensions.ring_thickness.f32(),
            washer_thickness: connector.dimensions.washer_thickness.f32(),
            cap_thickness: connector.dimensions.cap_thickness.f32(),
        };
        if self.sizes != Some(sizes) {
            self.sizes = Some(sizes);
            self.parts.clear();
        }

        let placed = placements(fabric);
        self.kinds = kinds(&parts_list(&placed));
        let mut instances: BTreeMap<Part, Vec<PartInstance>> = BTreeMap::new();
        for placement in &placed {
            instances.entry(placement.part).or_default().push(PartInstance::new(placement));
        }
        // A part's mesh is built the first time a placement needs it.
        for &part in instances.keys() {
            self.parts.entry(part).or_insert_with(|| PartBuffers {
                mesh: wgpu.create_part(part, sizes),
                instances: None,
                instance_count: 0,
            });
        }
        for (part, buffers) in &mut self.parts {
            let copies = instances.remove(part).unwrap_or_default();
            buffers.instance_count = copies.len() as u32;
            buffers.instances = (!copies.is_empty()).then(|| {
                wgpu.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&part.name()),
                        contents: bytemuck::cast_slice(&copies),
                        usage: wgpu::BufferUsages::VERTEX,
                    })
            });
        }
        let angle = self.started.elapsed().as_secs_f32() * THUMBNAIL_TURN_RATE;
        let thumbnails: Vec<PartInstance> = self
            .kinds
            .iter()
            .map(|kind| PartInstance::thumbnail(kind.sample, &self.parts[&kind.sample].mesh, angle))
            .collect();
        self.thumbnail_instances = (!thumbnails.is_empty()).then(|| {
            wgpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Parts Panel Instances"),
                contents: bytemuck::cast_slice(&thumbnails),
                usage: wgpu::BufferUsages::VERTEX,
            })
        });
    }

    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        bind_group: &'a wgpu::BindGroup,
    ) {
        for buffers in self.parts.values() {
            let Some(instances) = &buffers.instances else {
                continue;
            };
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, bind_group, &[]);
            render_pass.set_vertex_buffer(0, buffers.mesh.vertices.slice(..));
            render_pass.set_vertex_buffer(1, instances.slice(..));
            render_pass.set_index_buffer(buffers.mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..buffers.mesh.index_count, 0, 0..buffers.instance_count);
        }
    }

    /// The parts list by kind, as of the last update.
    pub fn kinds(&self) -> &[KindSummary] {
        &self.kinds
    }

    /// Each kind's rendering in its panel row's square.
    pub fn render_thumbnails<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, rows: &[PanelRow]) {
        let Some(instances) = &self.thumbnail_instances else {
            return;
        };
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.thumbnail_bind_group, &[]);
        render_pass.set_vertex_buffer(1, instances.slice(..));
        for (index, (kind, row)) in self.kinds.iter().zip(rows).enumerate() {
            let mesh = &self.parts[&kind.sample].mesh;
            let [left, top, side] = row.thumbnail;
            render_pass.set_viewport(left, top, side, side, 0.0, 1.0);
            render_pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            render_pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            let instance = index as u32;
            render_pass.draw_indexed(0..mesh.index_count, 0, instance..instance + 1);
        }
    }
}
