use cgmath::{EuclideanSpace, Matrix4, Point3, Transform, Vector3};

use crate::build::dsl::brick::{BakedBrick, BakedInterval, BakedJoint, BrickFace};
use crate::build::dsl::brick_dsl::BrickRole;
use crate::build::dsl::brick_dsl::FaceName::Attach;
use crate::build::dsl::{FaceAlias, Spin};
use crate::fabric::face::FaceRotation;
use crate::fabric::interval::Role;
use crate::fabric::{Fabric, UniqueId};

pub enum BaseFace {
    ExistingFace(UniqueId),
    Situated {
        spin: Spin,
        vector_space: Matrix4<f32>,
        // seed: Option<usize>,
    },
    Seeded,
}

impl Fabric {
    pub fn attach_brick(
        &mut self,
        baked_brick: &BakedBrick,
        brick_role: BrickRole,
        rotation: FaceRotation,
        scale_factor: f32,
        base_face: BaseFace,
    ) -> (UniqueId, Vec<UniqueId>) {
        let (scale, spin, matrix) = match base_face {
            BaseFace::ExistingFace(id) => {
                let face = self.face(id);
                let matrix = face.vector_space(self, rotation);
                let spin = Some(face.spin.opposite());
                (face.scale * scale_factor, spin, matrix)
            }
            BaseFace::Situated { spin, vector_space } => (scale_factor, Some(spin), vector_space),
            BaseFace::Seeded => (scale_factor, None, Matrix4::from_scale(scale_factor)),
        };
        let brick = baked_brick.clone();
        let joints: Vec<usize> = brick
            .joints
            .into_iter()
            .map(|BakedJoint { location, .. }| self.create_joint(matrix.transform_point(location)))
            .collect();
        for BakedInterval {
            alpha_index,
            omega_index,
            material_name,
            strain,
        } in brick.intervals
        {
            let (alpha_index, omega_index) = (joints[alpha_index], joints[omega_index]);
            let ideal = self.ideal(alpha_index, omega_index, strain);
            let role =
                Role::from_label(&material_name).expect(&format!("Material: {}", material_name));
            self.create_interval(alpha_index, omega_index, ideal, role);
        }
        let brick_faces = brick
            .faces
            .into_iter()
            .filter_map(
                |BrickFace {
                     joints: brick_joints,
                     aliases,
                     spin,
                 }| {
                    let aliases_for_role: Vec<_> = aliases
                        .into_iter()
                        .filter(
                            |FaceAlias {
                                 brick_role: alias_role,
                                 ..
                             }| { *alias_role == brick_role },
                        )
                        .collect();

                    // Skip faces with no aliases for this brick_role
                    if aliases_for_role.is_empty() {
                        return None;
                    }

                    let midpoint = brick_joints
                        .map(|index| self.joints[joints[index]].location.to_vec())
                        .into_iter()
                        .sum::<Vector3<f32>>()
                        / 3.0;
                    let alpha_index = self.create_joint(Point3::from_vec(midpoint));
                    let radial_intervals = brick_joints.map(|omega| {
                        let omega_index = joints[omega];
                        let ideal = self.ideal(alpha_index, omega_index, BakedBrick::TARGET_FACE_STRAIN);
                        self.create_interval(alpha_index, omega_index, ideal, Role::FaceRadial)
                    });
                    Some(self.create_face(aliases_for_role, scale, spin, radial_intervals))
                },
            )
            .collect::<Vec<_>>();
        let base_face = if spin.is_none() {
            // For seed bricks, use the first face with any alias (already filtered by brick_role)
            brick_faces.first().copied().expect("brick has no faces")
        } else {
            // For attached bricks, find the Attach face with matching spin
            brick_faces
                .iter()
                .find(|&&face_id| {
                    self.face(face_id)
                        .aliases
                        .iter()
                        .any(|FaceAlias { face_name, .. }| *face_name == Attach(spin.unwrap()))
                })
                .copied()
                .expect("missing attach face after creating brick")
        };
        (base_face, brick_faces)
    }
}
