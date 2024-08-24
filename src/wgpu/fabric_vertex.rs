use bytemuck::{Pod, Zeroable};
use crate::camera::Pick;
use crate::fabric::interval::{Interval, Role};
use crate::fabric::{Fabric, UniqueId};
use crate::fabric::material::interval_material;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Default)]
pub struct FabricVertex {
    position: [f32; 4],
    color: [f32; 4],
}

const GRAY: [f32; 4] = [0.5, 0.5, 0.5, 0.5];
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
        let (center_color, end_color) = match interval_material(interval.material).role {
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

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<FabricVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}
