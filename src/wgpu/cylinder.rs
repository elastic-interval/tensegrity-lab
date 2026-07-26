use crate::wgpu::Wgpu;
use wgpu::util::DeviceExt;

impl Wgpu {
    pub fn create_cylinder(&self, segments: u32) -> (wgpu::Buffer, wgpu::Buffer, u32) {
        use bytemuck::cast_slice;
        use std::f32::consts::PI;
        // Vertex format: (position[3], normal[3], uv[2])
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct CylinderVertex {
            position: [f32; 3],
            normal: [f32; 3],
            uv: [f32; 2],
        }

        const HALF_HEIGHT: f32 = 0.5;

        let mut vertices = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        // Pre-calculate the vertex positions for rings
        let mut ring_vertices = Vec::with_capacity(segments as usize);
        for i in 0..segments {
            let angle = (i as f32) / (segments as f32) * 2.0 * PI;
            let x = angle.cos();
            let z = angle.sin();
            // Normal points outward from the cylinder axis
            let normal = [angle.cos(), 0.0, angle.sin()];
            ring_vertices.push((x, z, normal));
        }

        // Create side vertices - top and bottom rings
        for i in 0..segments {
            let (x, z, normal) = ring_vertices[i as usize];

            // Top vertex of side
            vertices.push(CylinderVertex {
                position: [x, HALF_HEIGHT, z],
                normal, // Normal points outward
                uv: [i as f32 / segments as f32, 0.0],
            });

            // Bottom vertex of side
            vertices.push(CylinderVertex {
                position: [x, -HALF_HEIGHT, z],
                normal, // Normal points outward
                uv: [i as f32 / segments as f32, 1.0],
            });
        }

        // Side indices - IMPORTANT: Ensure correct winding order (counter-clockwise when viewed from outside)
        for i in 0..segments {
            let top_current = i * 2;
            let bottom_current = i * 2 + 1;
            let top_next = ((i + 1) % segments) * 2; // Wrap around to first vertex
            let bottom_next = ((i + 1) % segments) * 2 + 1; // Wrap around to first vertex

            // First triangle (counter-clockwise when viewed from outside)
            indices.push(top_current);
            indices.push(top_next);
            indices.push(bottom_current);

            // Second triangle (counter-clockwise when viewed from outside)
            indices.push(bottom_current);
            indices.push(top_next);
            indices.push(bottom_next);
        }

        // Top cap - flat circle facing up
        let top_center_idx = vertices.len() as u32;
        vertices.push(CylinderVertex {
            position: [0.0, HALF_HEIGHT, 0.0],
            normal: [0.0, 1.0, 0.0], // Normal points up
            uv: [0.5, 0.5],
        });

        // Top cap ring vertices (separate from side vertices for different normals)
        let top_ring_start = vertices.len() as u32;
        for i in 0..segments {
            let (x, z, _) = ring_vertices[i as usize];
            vertices.push(CylinderVertex {
                position: [x, HALF_HEIGHT, z],
                normal: [0.0, 1.0, 0.0], // Normal points up
                uv: [0.5 + 0.5 * x, 0.5 + 0.5 * z],
            });
        }

        // Top cap indices (counter-clockwise when viewed from above)
        for i in 0..segments {
            let current = top_ring_start + i;
            let next = top_ring_start + ((i + 1) % segments);
            indices.push(top_center_idx);
            indices.push(next);
            indices.push(current);
        }

        // Bottom cap - flat circle facing down
        let bottom_center_idx = vertices.len() as u32;
        vertices.push(CylinderVertex {
            position: [0.0, -HALF_HEIGHT, 0.0],
            normal: [0.0, -1.0, 0.0], // Normal points down
            uv: [0.5, 0.5],
        });

        // Bottom cap ring vertices
        let bottom_ring_start = vertices.len() as u32;
        for i in 0..segments {
            let (x, z, _) = ring_vertices[i as usize];
            vertices.push(CylinderVertex {
                position: [x, -HALF_HEIGHT, z],
                normal: [0.0, -1.0, 0.0], // Normal points down
                uv: [0.5 + 0.5 * x, 0.5 + 0.5 * z],
            });
        }

        // Bottom cap indices (counter-clockwise when viewed from below)
        for i in 0..segments {
            let current = bottom_ring_start + i;
            let next = bottom_ring_start + ((i + 1) % segments);
            indices.push(bottom_center_idx);
            indices.push(current);
            indices.push(next);
        }

        // Create vertex buffer
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Cylinder Vertex Buffer"),
                contents: cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        // Create index buffer
        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Cylinder Index Buffer"),
                contents: cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        (vertex_buffer, index_buffer, indices.len() as u32)
    }

    /// Unit connector plate: circle of radius 1 in the x/z plane with a boss
    /// extending to x = `boss_reach`, `boss_half_width` wide in z, extruded
    /// over y ∈ [−0.5, +0.5]. Instanced by `plate_vertex`, which scales x/z
    /// by the ring radius and y by the plate thickness, and aims +x (the
    /// boss) along the instance's radial direction.
    pub fn create_connector_plate(
        &self,
        boss_reach: f32,
        boss_half_width: f32,
    ) -> (wgpu::Buffer, wgpu::Buffer, u32) {
        use bytemuck::cast_slice;
        use std::f32::consts::PI;
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct PlateVertex {
            position: [f32; 3],
            normal: [f32; 3],
            uv: [f32; 2],
        }

        const HALF_HEIGHT: f32 = 0.5;
        const ARC_SEGMENTS: u32 = 48;

        // Outline in (x, z), counterclockwise in parameter angle: the long
        // arc between the two boss junctions, then the two boss corners.
        let theta = boss_half_width.asin();
        let mut outline: Vec<[f32; 2]> = Vec::new();
        for i in 0..=ARC_SEGMENTS {
            let a = theta + (2.0 * PI - 2.0 * theta) * (i as f32) / (ARC_SEGMENTS as f32);
            outline.push([a.cos(), a.sin()]);
        }
        outline.push([boss_reach, -boss_half_width]);
        outline.push([boss_reach, boss_half_width]);
        let n = outline.len();

        let mut vertices: Vec<PlateVertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        // Arc side wall: smooth radial normals shared between neighbouring
        // quads, so the curved rim shades round instead of faceted. Arc
        // points sit on the unit circle, so the radial normal is the
        // position itself.
        let arc_base = vertices.len() as u32;
        for [x, z] in outline.iter().take(ARC_SEGMENTS as usize + 1) {
            let normal = [*x, 0.0, *z];
            vertices.push(PlateVertex {
                position: [*x, HALF_HEIGHT, *z],
                normal,
                uv: [0.0, 0.0],
            });
            vertices.push(PlateVertex {
                position: [*x, -HALF_HEIGHT, *z],
                normal,
                uv: [0.0, 1.0],
            });
        }
        for i in 0..ARC_SEGMENTS {
            let p_top = arc_base + i * 2;
            let p_bot = p_top + 1;
            let q_top = p_top + 2;
            let q_bot = p_top + 3;
            // Same relative winding as the cylinder side quads.
            indices.extend([p_top, q_top, p_bot, p_bot, q_top, q_bot]);
        }

        // Boss side walls: three straight edges with flat outward normals,
        // so the boss keeps crisp corners. For this winding the outward
        // normal of edge d = (dx, dz) is (dz, −dx).
        let a = ARC_SEGMENTS as usize; // index of the last arc point
        for (i, j) in [(a, a + 1), (a + 1, a + 2), (a + 2, 0)] {
            let [px, pz] = outline[i];
            let [qx, qz] = outline[j];
            let (dx, dz) = (qx - px, qz - pz);
            let len = (dx * dx + dz * dz).sqrt();
            let normal = [dz / len, 0.0, -dx / len];

            let base = vertices.len() as u32;
            for (x, z) in [(px, pz), (qx, qz)] {
                vertices.push(PlateVertex {
                    position: [x, HALF_HEIGHT, z],
                    normal,
                    uv: [0.0, 0.0],
                });
                vertices.push(PlateVertex {
                    position: [x, -HALF_HEIGHT, z],
                    normal,
                    uv: [0.0, 1.0],
                });
            }
            let (p_top, p_bot, q_top, q_bot) = (base, base + 1, base + 2, base + 3);
            indices.extend([p_top, q_top, p_bot, p_bot, q_top, q_bot]);
        }

        // Top and bottom faces: fan from the origin (the outline is
        // star-shaped from there, since both the disc and the boss
        // rectangle contain it).
        for (y, normal, flip) in [
            (HALF_HEIGHT, [0.0, 1.0, 0.0], false),
            (-HALF_HEIGHT, [0.0, -1.0, 0.0], true),
        ] {
            let center = vertices.len() as u32;
            vertices.push(PlateVertex {
                position: [0.0, y, 0.0],
                normal,
                uv: [0.5, 0.5],
            });
            let ring_start = vertices.len() as u32;
            for [x, z] in &outline {
                vertices.push(PlateVertex {
                    position: [*x, y, *z],
                    normal,
                    uv: [0.5 + 0.5 * x, 0.5 + 0.5 * z],
                });
            }
            for i in 0..n as u32 {
                let current = ring_start + i;
                let next = ring_start + (i + 1) % n as u32;
                if flip {
                    indices.extend([center, current, next]);
                } else {
                    indices.extend([center, next, current]);
                }
            }
        }

        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Connector Plate Vertex Buffer"),
                contents: cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Connector Plate Index Buffer"),
                contents: cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        (vertex_buffer, index_buffer, indices.len() as u32)
    }

    pub fn cylinder_vertex_layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem::size_of;

        wgpu::VertexBufferLayout {
            array_stride: size_of::<[f32; 8]>() as wgpu::BufferAddress, // position[3] + normal[3] + uv[2]
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // normal
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // uv
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}
