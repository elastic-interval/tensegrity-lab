use cgmath::{EuclideanSpace, Matrix4, Point3, Transform, Vector3};

use crate::build::dsl::brick::{BakedBrick, BakedInterval, BakedJoint, BrickFace};
use crate::build::dsl::brick_dsl::BrickRole;
use crate::build::dsl::brick_dsl::FaceName::Attach;
use crate::build::dsl::{FaceAlias, Spin};
use crate::fabric::face::FaceRotation;
use crate::fabric::interval::Role;
use crate::fabric::joint_path::JointPath;
use crate::fabric::{Fabric, FaceKey, JointKey};

pub enum BaseFace {
    ExistingFace(FaceKey),
    Situated {
        spin: Spin,
        vector_space: Matrix4<f32>,
    },
    Seeded {
        altitude: f32,
    },
}

impl Fabric {
    pub fn attach_brick(
        &mut self,
        baked_brick: &BakedBrick,
        brick_role: BrickRole,
        rotation: FaceRotation,
        scale_factor: f32,
        base_face: BaseFace,
        base_path: &JointPath,
    ) -> (FaceKey, Vec<FaceKey>) {
        let (base_scale, spin, matrix) = match base_face {
            BaseFace::ExistingFace(id) => {
                let face = self.face(id);
                let matrix = face.vector_space(self, rotation);
                let spin = Some(face.spin.mirror());
                (face.scale * scale_factor, spin, matrix)
            }
            BaseFace::Situated { spin, vector_space } => (scale_factor, Some(spin), vector_space),
            BaseFace::Seeded { altitude } => {
                let matrix = Matrix4::from_translation(Vector3::new(0.0, altitude, 0.0))
                    * Matrix4::from_scale(scale_factor);
                (scale_factor, None, matrix)
            }
        };
        let brick = baked_brick.clone();
        let joint_keys: Vec<JointKey> = brick
            .joints
            .into_iter()
            .enumerate()
            .map(|(index, BakedJoint { location, .. })| {
                let path = base_path.with_local_index(index as u8);
                self.create_joint_with_path(matrix.transform_point(location), path)
            })
            .collect();
        for BakedInterval {
            alpha_index,
            omega_index,
            material_name,
            strain,
        } in brick.intervals
        {
            let (alpha_key, omega_key) = (joint_keys[alpha_index], joint_keys[omega_index]);
            let role =
                Role::from_label(&material_name).expect(&format!("Material: {}", material_name));
            self.create_strained_interval(alpha_key, omega_key, role, strain);
        }
        let brick_faces = brick
            .faces
            .into_iter()
            .filter_map(
                |BrickFace {
                     joints: brick_joints,
                     aliases,
                     spin,
                     scale,
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
                        .map(|index| self.joints[joint_keys[index]].location.to_vec())
                        .into_iter()
                        .sum::<Vector3<f32>>()
                        / 3.0;
                    // Face midpoint gets a path based on base_path with local_index 6 + first brick joint
                    // This distinguishes different faces on the same brick
                    let midpoint_local = 6 + brick_joints[0] as u8;
                    let midpoint_path = base_path.with_local_index(midpoint_local);
                    let alpha_key = self.create_joint_with_path(Point3::from_vec(midpoint), midpoint_path);
                    let radial_intervals = brick_joints.map(|omega| {
                        let omega_key = joint_keys[omega];
                        self.create_strained_interval(alpha_key, omega_key, Role::FaceRadial, BakedBrick::TARGET_FACE_STRAIN)
                    });
                    Some(self.create_face(
                        aliases_for_role,
                        base_scale * scale,
                        spin,
                        radial_intervals,
                    ))
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
                .find(|&&face_key| {
                    self.face(face_key)
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
