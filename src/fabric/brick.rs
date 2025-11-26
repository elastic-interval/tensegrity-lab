use cgmath::{EuclideanSpace, Matrix4, Point3, Transform, Vector3};

use crate::build::dsl::brick::{Baked, BakedInterval, BakedJoint, Brick, BrickFace};
use crate::build::dsl::brick_dsl::BrickName;
use crate::build::dsl::brick_library::BrickLibrary;
use crate::build::dsl::Spin;
use crate::fabric::face::FaceRotation;
use crate::fabric::interval::Role;
use crate::fabric::{Fabric, UniqueId};

pub enum BaseFace {
    ExistingFace(UniqueId),
    Situated {
        spin: Spin,
        vector_space: Matrix4<f32>,
        seed: Option<usize>,
    },
    Seeded(usize),
    Baseless,
}

impl Fabric {
    pub fn attach_brick(
        &mut self,
        brick_definition: &Brick,
        rotation: FaceRotation,
        scale_factor: f32,
        base_face: BaseFace,
    ) -> (UniqueId, Vec<UniqueId>) {
        let (scale, spin, matrix, seed) = match base_face {
            BaseFace::ExistingFace(id) => {
                let face = self.face(id);
                (
                    face.scale * scale_factor,
                    Some(face.spin.opposite()),
                    face.vector_space(self, rotation),
                    None,
                )
            }
            BaseFace::Situated {
                spin,
                vector_space,
                seed,
            } => (scale_factor, Some(spin), vector_space, seed),
            BaseFace::Seeded(orient_alias) => (
                scale_factor,
                None,
                Matrix4::from_scale(scale_factor),
                Some(orient_alias),
            ),
            BaseFace::Baseless => (scale_factor, None, Matrix4::from_scale(scale_factor), None),
        };
        let baked = brick_definition.baked.unwrap();
        let joints: Vec<usize> = baked
            .joints
            .into_iter()
            .map(|BakedJoint { location, .. }| self.create_joint(matrix.transform_point(location)))
            .collect();
        for BakedInterval {
            alpha_index,
            omega_index,
            material_name,
            strain,
        } in baked.intervals
        {
            let (alpha_index, omega_index) = (joints[alpha_index], joints[omega_index]);
            let ideal = self.ideal(alpha_index, omega_index, strain);
            let role = Role::from_label(&material_name)
                .expect(&format!("Unknown role label: {}", material_name));
            self.create_interval(
                alpha_index,
                omega_index,
                ideal,
                role,
            );
        }
        let brick_faces = brick
            .faces
            .into_iter()
            .map(
                |BrickFace {
                     joints: brick_joints,
                     aliases,
                     spin,
                 }| {
                    let midpoint = brick_joints
                        .map(|index| self.joints[joints[index]].location.to_vec())
                        .into_iter()
                        .sum::<Vector3<f32>>()
                        / 3.0;
                    let alpha_index = self.create_joint(Point3::from_vec(midpoint));
                    let radial_intervals = brick_joints.map(|omega| {
                        let omega_index = joints[omega];
                        let ideal = self.ideal(alpha_index, omega_index, Baked::TARGET_FACE_STRAIN);
                        self.create_interval(alpha_index, omega_index, ideal, Role::FaceRadial)
                    });
                    let spin_alias = face_alias.spin().or(spin).map(Spin::into_alias);
                    let search_alias = match spin_alias {
                        None => face_alias.with_seed(seed),
                        Some(spin_alias) => spin_alias + face_alias,
                    };
                    let single_alias: Vec<_> = aliases
                        .into_iter()
                        .filter(|alias| search_alias.matches(alias))
                        .collect();
                    assert_eq!(
                        single_alias.len(),
                        1,
                        "filter must leave exactly one face alias"
                    );
                    self.create_face(single_alias, scale, spin, radial_intervals)
                },
            )
            .collect::<Vec<_>>();
        let base_face = brick_faces
            .iter()
            .find(|&&face_id| self.face(face_id).alias().is_base())
            .expect("missing face after creating brick");
        (*base_face, brick_faces)
    }
}
