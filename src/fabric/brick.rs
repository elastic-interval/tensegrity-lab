use cgmath::{EuclideanSpace, Point3, Transform, Vector3};

use crate::build::brick::Baked;
use crate::build::tenscript::FaceName;
use crate::fabric::{Fabric, Link, UniqueId};
use crate::fabric::face::Face;
use crate::fabric::interval::Role;

const ROOT3: f32 = 1.732_050_8;
const ROOT5: f32 = 2.236_068;
const ROOT6: f32 = 2.449_489_8;
const PHI: f32 = (1f32 + ROOT5) / 2f32;

impl Fabric {
    pub fn attach_brick(&mut self, brick_name: &str, scale_factor: f32, face_id: Option<UniqueId>) -> Vec<(FaceName, UniqueId)> {
        let face = face_id.map(|id| self.face(id));
        let scale = face.map(|Face { scale, .. }| *scale).unwrap_or(1.0) * scale_factor;
        let brick = Baked::new(brick_name);
        let matrix = face.map(|face| face.vector_space(self, true));
        let joints: Vec<usize> = brick.joints
            .into_iter()
            .map(|point| self.create_joint(match matrix {
                None => point,
                Some(matrix) => matrix.transform_point(point),
            }))
            .collect();
        for (alpha, omega, role, strain) in brick.intervals {
            let (alpha_index, omega_index) = (joints[alpha], joints[omega]);
            let ideal = self.ideal(alpha_index, omega_index, strain);
            self.create_interval(alpha_index, omega_index, match role {
                Role::Push => Link::push(ideal),
                Role::Pull => Link::pull(ideal),
            });
        }
        let faces: Vec<_> = brick.faces
            .into_iter()
            .map(|(brick_joints, face_name, spin)| {
                let midpoint = brick_joints
                    .map(|index| self.joints[joints[index]].location.to_vec())
                    .into_iter()
                    .sum::<Vector3<f32>>() / 3.0;
                let alpha_index = self.create_joint(Point3::from_vec(midpoint));
                let radial_intervals = brick_joints.map(|omega| {
                    let omega_index = joints[omega];
                    let ideal = self.ideal(alpha_index, omega_index, Baked::TARGET_FACE_STRAIN);
                    self.create_interval(alpha_index, omega_index, Link::pull(ideal))
                });
                (face_name, self.create_face(face_name, scale, spin, radial_intervals))
            })
            .collect();
        let a_neg_face = faces
            .iter()
            .find_map(|(FaceName(index), face_id)| (*index == 0).then_some(face_id))
            .expect("no Aneg face");
        if let Some(id) = face_id { self.join_faces(id, *a_neg_face) }
        faces
    }
}
