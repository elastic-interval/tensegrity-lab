/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::camera::Pick;
use crate::connector::parts::{placements, Material, Part, Placement};
use crate::fabric::Fabric;
use crate::units::Unit;
use crate::wgpu::part_meshes::PartSizes;
use crate::wgpu::Wgpu;
use bytemuck::{Pod, Zeroable};
use std::collections::BTreeMap;
use std::mem::size_of;
use wgpu::util::DeviceExt;

// Materials (docs/connectors.md, "Materials"): the fabricated connector is
// hot-dip galvanized steel; terminals, shackles and clamps are bought
// AISI-316 stainless. Our shader is diffuse-only, so the stainless base
// colour is lifted above the table's PBR value to stand in for its shine —
// the real contrast is roughness, not brightness. Parts of assemblies with
// a construction problem are painted red instead.
const GALVANIZED_COLOR: [f32; 4] = [0.60, 0.62, 0.64, 1.0];
const STAINLESS_COLOR: [f32; 4] = [0.72, 0.73, 0.75, 1.0];
const OVERLAP_COLOR: [f32; 4] = [0.90, 0.12, 0.12, 1.0];
const POM_COLOR: [f32; 4] = [0.93, 0.92, 0.88, 1.0];

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
            match placement.part.material() {
                Material::Galvanized => GALVANIZED_COLOR,
                Material::Stainless => STAINLESS_COLOR,
                Material::Pom => POM_COLOR,
            }
        };
        PartInstance {
            center_radius: [o.x, o.y, o.z, 1.0],
            axis_thickness: [y.x, y.y, y.z, 1.0],
            boss_dir: [x.x, x.y, x.z, 0.0],
            color,
        }
    }
}

/// A part's mesh and the copies of it to draw.
struct PartBuffers {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
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
        ConnectorRenderer {
            pipeline: wgpu.create_plate_pipeline("Connector Part Pipeline", instance_layout),
            sizes: None,
            parts: BTreeMap::new(),
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

        let mut instances: BTreeMap<Part, Vec<PartInstance>> = BTreeMap::new();
        for placement in placements(fabric) {
            instances.entry(placement.part).or_default().push(PartInstance::new(&placement));
        }
        // A part's mesh is built the first time a placement needs it.
        for &part in instances.keys() {
            self.parts.entry(part).or_insert_with(|| {
                let (vertices, indices, index_count) = wgpu.create_part(part, sizes);
                PartBuffers {
                    vertices,
                    indices,
                    index_count,
                    instances: None,
                    instance_count: 0,
                }
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
            render_pass.set_vertex_buffer(0, buffers.vertices.slice(..));
            render_pass.set_vertex_buffer(1, instances.slice(..));
            render_pass.set_index_buffer(buffers.indices.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..buffers.index_count, 0, 0..buffers.instance_count);
        }
    }
}
