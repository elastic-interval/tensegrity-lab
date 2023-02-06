use cgmath::{EuclideanSpace, MetricSpace, Point3, Transform, Vector3};

use crate::build::brick::{Brick, BrickName};
use crate::build::tenscript::FaceName;
use crate::build::tenscript::FaceName::{*};
use crate::fabric::{Fabric, Link, UniqueId};
use crate::fabric::face::Face;
use crate::fabric::interval::Role;

const ROOT3: f32 = 1.732_050_8;
const ROOT5: f32 = 2.236_068;
const ROOT6: f32 = 2.449_489_8;
const PHI: f32 = (1f32 + ROOT5) / 2f32;

impl Fabric {
    pub fn attach_brick(&mut self, brick_name: BrickName, pretenst_factor: f32, scale_factor: f32, face_id: Option<UniqueId>) -> Vec<(FaceName, UniqueId)> {
        let face = face_id.map(|id| self.face(id));
        let scale = face.map(|Face { scale, .. }| *scale).unwrap_or(1.0) * scale_factor;
        let brick = Brick::new(brick_name);
        let matrix = face.map(|face| face.space(self, true));
        let joints: Vec<usize> = brick.joints
            .iter()
            .map(|point| self.create_joint(match matrix {
                None => *point,
                Some(matrix) => matrix.transform_point(*point),
            })).collect();
        for (alpha, omega, role) in brick.intervals {
            let (alpha_index, omega_index) = (joints[alpha], joints[omega]);
            let distance = self.distance(alpha_index, omega_index);
            self.create_interval(alpha_index, omega_index, match role {
                Role::Push => Link::push(distance * pretenst_factor),
                Role::Pull => Link::pull(distance / pretenst_factor),
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
                    let distance = self.distance(alpha_index, omega_index);
                    self.create_interval(alpha_index, omega_index, Link::pull(distance / pretenst_factor))
                });
                (face_name, self.create_face(face_name, scale, spin, radial_intervals))
            })
            .collect();
        let a_neg_face = faces
            .iter()
            .find_map(|(face_name, face_id)| (*face_name == Aneg).then_some(face_id))
            .expect("no Aneg face");
        if let Some(id) = face_id { self.join_faces(id, *a_neg_face) }
        faces
    }

    pub fn join_faces(&mut self, alpha_id: UniqueId, omega_id: UniqueId) {
        let (alpha, omega) = (self.face(alpha_id), self.face(omega_id));
        let (mut alpha_ends, omega_ends) = (alpha.radial_joints(self), omega.radial_joints(self));
        alpha_ends.reverse();
        let (mut alpha_points, omega_points) = (
            alpha_ends.map(|id| self.location(id)),
            omega_ends.map(|id| self.location(id))
        );
        let links = [(0, 0), (0, 1), (1, 1), (1, 2), (2, 2), (2, 0)];
        let (_, alpha_rotated) = (0..3)
            .map(|rotation| {
                let length: f32 = links
                    .map(|(a, b)| alpha_points[a].distance(omega_points[b]))
                    .iter()
                    .sum();
                alpha_points.rotate_right(1);
                let mut rotated = alpha_ends;
                rotated.rotate_right(rotation);
                (length, rotated)
            })
            .min_by(|(length_a, _), (length_b, _)| length_a.partial_cmp(length_b).unwrap())
            .unwrap();
        let ideal = (alpha.scale + omega.scale) / 2.0;
        for (a, b) in links {
            self.create_interval(alpha_rotated[a], omega_ends[b], Link::pull(ideal));
        }
        self.remove_face(alpha_id);
        self.remove_face(omega_id);
    }
}
