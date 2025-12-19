use crate::wgpu::Wgpu;
use wgpu::util::DeviceExt;

impl Wgpu {
    pub fn create_cylinder(&self) -> (wgpu::Buffer, wgpu::Buffer, u32) {
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
        const SEGMENTS: u32 = 12;

        let mut vertices = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        // Pre-calculate the vertex positions for rings
        let mut ring_vertices = Vec::with_capacity(SEGMENTS as usize);
        for i in 0..SEGMENTS {
            let angle = (i as f32) / (SEGMENTS as f32) * 2.0 * PI;
            let x = angle.cos();
            let z = angle.sin();
            // Normal points outward from the cylinder axis
            let normal = [angle.cos(), 0.0, angle.sin()];
            ring_vertices.push((x, z, normal));
        }

        // Create side vertices - top and bottom rings
        for i in 0..SEGMENTS {
            let (x, z, normal) = ring_vertices[i as usize];

            // Top vertex of side
            vertices.push(CylinderVertex {
                position: [x, HALF_HEIGHT, z],
                normal, // Normal points outward
                uv: [i as f32 / SEGMENTS as f32, 0.0],
            });

            // Bottom vertex of side
            vertices.push(CylinderVertex {
                position: [x, -HALF_HEIGHT, z],
                normal, // Normal points outward
                uv: [i as f32 / SEGMENTS as f32, 1.0],
            });
        }

        // Side indices - IMPORTANT: Ensure correct winding order (counter-clockwise when viewed from outside)
        for i in 0..SEGMENTS {
            let top_current = i * 2;
            let bottom_current = i * 2 + 1;
            let top_next = ((i + 1) % SEGMENTS) * 2; // Wrap around to first vertex
            let bottom_next = ((i + 1) % SEGMENTS) * 2 + 1; // Wrap around to first vertex

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
        for i in 0..SEGMENTS {
            let (x, z, _) = ring_vertices[i as usize];
            vertices.push(CylinderVertex {
                position: [x, HALF_HEIGHT, z],
                normal: [0.0, 1.0, 0.0], // Normal points up
                uv: [0.5 + 0.5 * x, 0.5 + 0.5 * z],
            });
        }

        // Top cap indices (counter-clockwise when viewed from above)
        for i in 0..SEGMENTS {
            let current = top_ring_start + i;
            let next = top_ring_start + ((i + 1) % SEGMENTS);
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
        for i in 0..SEGMENTS {
            let (x, z, _) = ring_vertices[i as usize];
            vertices.push(CylinderVertex {
                position: [x, -HALF_HEIGHT, z],
                normal: [0.0, -1.0, 0.0], // Normal points down
                uv: [0.5 + 0.5 * x, 0.5 + 0.5 * z],
            });
        }

        // Bottom cap indices (counter-clockwise when viewed from below)
        for i in 0..SEGMENTS {
            let current = bottom_ring_start + i;
            let next = bottom_ring_start + ((i + 1) % SEGMENTS);
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
