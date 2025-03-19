use wgpu::util::DeviceExt;
use crate::wgpu::Wgpu;

const RADIUS: f32 = 1.0;
const HEIGHT: f32 = 1.0;
const SEGMENT_COUNT: usize = 6;

impl Wgpu {
    pub fn create_cylinder(&self) -> (wgpu::Buffer, wgpu::Buffer, u32) {
        use std::f32::consts::PI;
        use bytemuck::cast_slice;

        // Vertex format: (position[3], normal[3], uv[2])
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct CylinderVertex {
            position: [f32; 3],
            normal: [f32; 3],
            uv: [f32; 2],
        }

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let half_height = HEIGHT / 2.0;

        // Create the circle vertices for top and bottom caps and the side vertices
        for i in 0..=SEGMENT_COUNT {
            let angle = (i as f32) / (SEGMENT_COUNT as f32) * 2.0 * PI;
            let x = RADIUS * angle.cos();
            let z = RADIUS * angle.sin();

            // Normal pointing outward from the cylinder's side
            let side_normal = [angle.cos(), 0.0, angle.sin()];

            // Top vertex (side)
            vertices.push(CylinderVertex {
                position: [x, half_height, z],
                normal: side_normal,
                uv: [i as f32 / SEGMENT_COUNT as f32, 0.0],
            });

            // Bottom vertex (side)
            vertices.push(CylinderVertex {
                position: [x, -half_height, z],
                normal: side_normal,
                uv: [i as f32 / SEGMENT_COUNT as f32, 1.0],
            });

            // Top cap vertex
            vertices.push(CylinderVertex {
                position: [x, half_height, z],
                normal: [0.0, 1.0, 0.0],
                uv: [0.5 + 0.5 * angle.cos(), 0.5 + 0.5 * angle.sin()],
            });

            // Bottom cap vertex
            vertices.push(CylinderVertex {
                position: [x, -half_height, z],
                normal: [0.0, -1.0, 0.0],
                uv: [0.5 + 0.5 * angle.cos(), 0.5 + 0.5 * angle.sin()],
            });
        }

        // Add center vertices for caps
        vertices.push(CylinderVertex {
            position: [0.0, half_height, 0.0],
            normal: [0.0, 1.0, 0.0],
            uv: [0.5, 0.5],
        });
        let top_center_idx = vertices.len() - 1;

        vertices.push(CylinderVertex {
            position: [0.0, -half_height, 0.0],
            normal: [0.0, -1.0, 0.0],
            uv: [0.5, 0.5],
        });
        let bottom_center_idx = vertices.len() - 1;

        // Create indices for the sides (triangle list)
        for i in 0..SEGMENT_COUNT {
            let base = i * 4; // 4 vertices per segment (2 for sides, 2 for caps)

            // Side triangles (2 per segment)
            indices.push(base);
            indices.push(base + 4);
            indices.push(base + 1);

            indices.push(base + 1);
            indices.push(base + 4);
            indices.push(base + 5);

            // Cap triangles (1 per cap per segment)
            indices.push(base + 2);
            indices.push(top_center_idx);
            indices.push(base + 6);

            indices.push(base + 7);
            indices.push(bottom_center_idx);
            indices.push(base + 3);
        }

        // Create vertex buffer
        let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cylinder Vertex Buffer"),
            contents: cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Create index buffer
        let index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cylinder Index Buffer"),
            contents: cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        (vertex_buffer, index_buffer, indices.len() as u32)
    }

    /// Returns the vertex buffer layout for the cylinder mesh
    pub fn cylinder_vertex_layout<'a>() -> wgpu::VertexBufferLayout<'a> {
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