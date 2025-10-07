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

        const HALF_HEIGHT: f32 = 0.45; // Shortened by 10% (was 0.5)
        const TIP_EXTENSION: f32 = 0.05; // Cone tips extend 5% beyond cylinder body
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

        // Top cone with flat shading (each face has its own normal)
        // Each triangle needs its own set of vertices with the same face normal
        let apex_pos = [0.0, HALF_HEIGHT + TIP_EXTENSION, 0.0];
        
        for i in 0..SEGMENTS {
            let current_idx = i as usize;
            let next_idx = ((i + 1) % SEGMENTS) as usize;
            
            let (x1, z1, _) = ring_vertices[current_idx];
            let (x2, z2, _) = ring_vertices[next_idx];
            
            let v1 = [x1, HALF_HEIGHT, z1];
            let v2 = [x2, HALF_HEIGHT, z2];
            
            // Calculate face normal: (v2 - apex) × (v1 - apex)
            let edge1 = [v2[0] - apex_pos[0], v2[1] - apex_pos[1], v2[2] - apex_pos[2]];
            let edge2 = [v1[0] - apex_pos[0], v1[1] - apex_pos[1], v1[2] - apex_pos[2]];
            
            // Cross product
            let face_normal = [
                edge1[1] * edge2[2] - edge1[2] * edge2[1],
                edge1[2] * edge2[0] - edge1[0] * edge2[2],
                edge1[0] * edge2[1] - edge1[1] * edge2[0],
            ];
            
            // Normalize
            let len = (face_normal[0] * face_normal[0] + 
                      face_normal[1] * face_normal[1] + 
                      face_normal[2] * face_normal[2]).sqrt();
            let face_normal = [
                face_normal[0] / len,
                face_normal[1] / len,
                face_normal[2] / len,
            ];
            
            // Create 3 vertices with the same face normal
            let apex_idx = vertices.len() as u32;
            vertices.push(CylinderVertex {
                position: apex_pos,
                normal: face_normal,
                uv: [0.5, 0.0],
            });
            
            vertices.push(CylinderVertex {
                position: v2,
                normal: face_normal,
                uv: [0.5 + 0.5 * x2, 0.5 + 0.5 * z2],
            });
            
            vertices.push(CylinderVertex {
                position: v1,
                normal: face_normal,
                uv: [0.5 + 0.5 * x1, 0.5 + 0.5 * z1],
            });
            
            // Indices for this triangle
            indices.push(apex_idx);
            indices.push(apex_idx + 1);
            indices.push(apex_idx + 2);
        }

        // Bottom cone with flat shading
        let bottom_apex_pos = [0.0, -HALF_HEIGHT - TIP_EXTENSION, 0.0];
        
        for i in 0..SEGMENTS {
            let current_idx = i as usize;
            let next_idx = ((i + 1) % SEGMENTS) as usize;
            
            let (x1, z1, _) = ring_vertices[current_idx];
            let (x2, z2, _) = ring_vertices[next_idx];
            
            let v1 = [x1, -HALF_HEIGHT, z1];
            let v2 = [x2, -HALF_HEIGHT, z2];
            
            // Calculate face normal: (v1 - apex) × (v2 - apex)
            let edge1 = [v1[0] - bottom_apex_pos[0], v1[1] - bottom_apex_pos[1], v1[2] - bottom_apex_pos[2]];
            let edge2 = [v2[0] - bottom_apex_pos[0], v2[1] - bottom_apex_pos[1], v2[2] - bottom_apex_pos[2]];
            
            // Cross product
            let face_normal = [
                edge1[1] * edge2[2] - edge1[2] * edge2[1],
                edge1[2] * edge2[0] - edge1[0] * edge2[2],
                edge1[0] * edge2[1] - edge1[1] * edge2[0],
            ];
            
            // Normalize
            let len = (face_normal[0] * face_normal[0] + 
                      face_normal[1] * face_normal[1] + 
                      face_normal[2] * face_normal[2]).sqrt();
            let face_normal = [
                face_normal[0] / len,
                face_normal[1] / len,
                face_normal[2] / len,
            ];
            
            // Create 3 vertices with the same face normal
            let apex_idx = vertices.len() as u32;
            vertices.push(CylinderVertex {
                position: bottom_apex_pos,
                normal: face_normal,
                uv: [0.5, 1.0],
            });
            
            vertices.push(CylinderVertex {
                position: v1,
                normal: face_normal,
                uv: [0.5 + 0.5 * x1, 0.5 + 0.5 * z1],
            });
            
            vertices.push(CylinderVertex {
                position: v2,
                normal: face_normal,
                uv: [0.5 + 0.5 * x2, 0.5 + 0.5 * z2],
            });
            
            // Indices for this triangle
            indices.push(apex_idx);
            indices.push(apex_idx + 1);
            indices.push(apex_idx + 2);
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
