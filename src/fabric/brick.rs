use cgmath::{EuclideanSpace, Point3, Transform, Vector3};

use crate::build::tenscript::brick::{Baked, BakedInterval, BrickFace};
use crate::build::tenscript::brick_library::BrickLibrary;
use crate::build::tenscript::{FaceAlias, Spin};
use crate::fabric::face::{Face, FaceRotation};
use crate::fabric::{Fabric, Link, UniqueId};

impl Fabric {
    pub fn create_brick(
        &mut self,
        face_alias: &FaceAlias,
        rotation: FaceRotation,
        scale_factor: f32,
        face_id: Option<UniqueId>,
        brick_library: &BrickLibrary,
    ) -> (UniqueId, Vec<UniqueId>) {
        let face = face_id.map(|id| self.face(id));
        let scale = face.map(|Face { scale, .. }| *scale).unwrap_or(1.0) * scale_factor;
        let spin_alias = face_alias
            .spin()
            .or(face.map(|face| face.spin.opposite()))
            .map(Spin::into_alias);
        let search_alias = match spin_alias {
            None => face_alias.with_seed(),
            Some(spin_alias) => spin_alias + face_alias,
        };
        let brick = brick_library.new_brick(&search_alias);
        let matrix = face.map(|face| face.vector_space(self, rotation));
        let joints: Vec<usize> = brick
            .joints
            .into_iter()
            .map(|point| {
                self.create_joint(match matrix {
                    None => point,
                    Some(matrix) => matrix.transform_point(point),
                })
            })
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
            self.create_interval(
                alpha_index,
                omega_index,
                Link {
                    ideal,
                    material_name,
                },
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
                        self.create_interval(alpha_index, omega_index, Link::pull(ideal))
                    });
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
        let search_base = search_alias.with_base();
        let base_face = brick_faces
            .iter()
            .find(|&&face_id| search_base.matches(self.face(face_id).alias()))
            .expect("missing face after creating brick");
        (*base_face, brick_faces)
    }
}
