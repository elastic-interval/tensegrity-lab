use cgmath::{EuclideanSpace, InnerSpace, Matrix4, Quaternion, Rotation, Vector3};
use crate::build::tenscript::Spin;
use crate::fabric::{Fabric, UniqueId};
use crate::fabric::face::Face;
use crate::fabric::interval::{Interval, Role};
use crate::fabric::joint::Joint;

#[derive(Debug, Clone)]
pub struct Brick {
    joints: Vec<(f32, f32, f32)>,
    intervals: Vec<(usize, usize, Role)>,
    faces: Vec<(usize, usize, usize, Spin)>,
}

impl From<(Fabric, UniqueId)> for Brick {
    fn from((fabric_template, face_id): (Fabric, UniqueId)) -> Self {
        let mut fabric = fabric_template;
        let face = fabric.face(face_id);
        let base = face.radial_intervals
            .map(|id| fabric.interval(id))
            .map(|Interval{omega_index, ..}|*omega_index)
            .map(|index| fabric.joints[index].location);
        let midpoint = base
            .iter()
            .map(|p| p.to_vec())
            .sum::<Vector3<f32>>() / base.len() as f32;
        let down = (base[1] - base[0])
            .cross(base[2] - base[0])
            .normalize();
        fabric.apply_matrix4(
            Matrix4::from_translation(midpoint) *
                Matrix4::from(Quaternion::between_vectors(down, -Vector3::unit_y())) *
                Matrix4::from_translation(-midpoint)
        );
        Self {
            joints: fabric.joints
                .iter()
                .map(|Joint { location, .. }|
                    (
                        location.x,
                        location.y,
                        location.z
                    )
                )
                .collect(),
            intervals: fabric.interval_values()
                .map(|Interval { alpha_index, omega_index, material, .. }|
                    (
                        *alpha_index,
                        *omega_index,
                        fabric.materials[*material].role
                    )
                )
                .collect(),
            faces: fabric.faces
                .values()
                .map(|Face { radial_intervals, spin, .. }|
                    (
                        fabric.interval(radial_intervals[0]).omega_index,
                        fabric.interval(radial_intervals[1]).omega_index,
                        fabric.interval(radial_intervals[2]).omega_index,
                        *spin,
                    )
                )
                .collect(),
        }
    }
}

#[test]
fn left_twist() {
    let mut fabric = Fabric::default();
    let [(_, face_id), ..] = fabric.single_twist(Spin::Left, 1.3, 1.0, None);
    let brick = Brick::from((fabric, face_id));
    dbg!(brick);
}
