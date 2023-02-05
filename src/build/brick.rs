use cgmath::{EuclideanSpace, InnerSpace, Matrix4, MetricSpace, Point3, Quaternion, Rotation, Vector3};
use crate::build::tenscript::Spin;
use crate::fabric::{Fabric, UniqueId};
use crate::fabric::interval::{Interval, Role};
use crate::fabric::joint::Joint;

#[derive(Debug, Clone)]
pub struct Brick {
    joints: Vec<Point3<f32>>,
    intervals: Vec<(usize, usize, Role)>,
    faces: Vec<([usize; 3], Spin)>,
}

impl From<(Fabric, UniqueId)> for Brick {
    fn from((fabric_template, face_id): (Fabric, UniqueId)) -> Self {
        let mut fabric = fabric_template;
        let face = fabric.face(face_id);
        let midpoint = face.midpoint(&fabric);
        let radial_x = face.radial_joint_locations(&fabric)[0].to_vec();
        let length = midpoint.distance(radial_x);
        fabric.apply_matrix4(
            Matrix4::from_scale(1.0 / length) *
                Matrix4::from(Quaternion::between_vectors(face.normal(&fabric), -Vector3::unit_y())) *
                Matrix4::from(Quaternion::between_vectors(radial_x - midpoint, Vector3::unit_x())) *
                Matrix4::from_translation(-midpoint)
        );
        Self {
            joints: fabric.joints
                .iter()
                .map(|Joint { location, .. }| *location)
                .collect(),
            intervals: fabric.interval_values()
                .map(|Interval { alpha_index, omega_index, material, .. }|
                    (*alpha_index, *omega_index, fabric.materials[*material].role)
                )
                .collect(),
            faces: fabric.faces
                .values()
                .map(|face| (face.radial_joints(&fabric), face.spin))
                .collect(),
        }
    }
}

#[test]
fn left_twist() {
    let mut fabric = Fabric::default();
    let [(_, face_id), ..] = fabric.double_twist(Spin::Left, 1.3, 1.0, None);
    fabric.set_altitude(10.0);
    let brick = Brick::from((fabric, face_id));
    dbg!(brick);
}
