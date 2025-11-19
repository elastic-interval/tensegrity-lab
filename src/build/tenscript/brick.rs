use std::collections::HashMap;

use cgmath::num_traits::abs;
use cgmath::{
    EuclideanSpace, InnerSpace, Matrix3, Matrix4, Point3, Quaternion, Rotation, Transform,
    Vector3,
};

use crate::build::tenscript::Spin::{Left, Right};
use crate::build::tenscript::{FaceAlias, Spin};
use crate::fabric::interval::{Interval, Role};
use crate::fabric::joint_incident::JointIncident;
use crate::fabric::Fabric;

#[derive(Copy, Clone, Debug)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    /// Create a push interval definition along this axis
    pub fn push(self, ideal: f32, alpha: impl Into<String>, omega: impl Into<String>) -> PushDef {
        PushDef {
            axis: self,
            ideal,
            alpha_name: alpha.into(),
            omega_name: omega.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PushDef {
    pub axis: Axis,
    pub ideal: f32,
    pub alpha_name: String,
    pub omega_name: String,
}

#[derive(Clone, Debug)]
pub struct PullDef {
    pub alpha_name: String,
    pub omega_name: String,
    pub ideal: f32,
    pub material: String,
}

#[derive(Clone, Debug)]
pub struct FaceDef {
    pub spin: Spin,
    pub joint_names: [String; 3],
    pub aliases: Vec<FaceAlias>,
}

#[derive(Clone, Default, Debug)]
pub struct Prototype {
    pub alias: FaceAlias,
    pub joints: Vec<String>,
    pub pushes: Vec<PushDef>,
    pub pulls: Vec<PullDef>,
    pub faces: Vec<FaceDef>,
}

#[derive(Clone, Debug)]
pub struct BrickDefinition {
    pub proto: Prototype,
    pub baked: Option<Baked>,
}

impl BrickDefinition {
    /// Get the baked faces, deriving them from the prototype if not stored
    pub fn baked_faces(&self) -> Vec<BrickFace> {
        if let Some(baked) = &self.baked {
            if !baked.faces.is_empty() {
                return baked.faces.clone();
            }
        }
        
        // Derive faces from prototype
        crate::build::brick_dsl::derive_baked_faces(&self.proto)
    }
}

impl From<Prototype> for Fabric {
    fn from(proto: Prototype) -> Self {
        let mut fabric = Fabric::new("prototype".to_string());
        let mut joints_by_name = HashMap::new();
        for name in proto.joints {
            let joint_index = fabric.create_joint(Point3::origin());
            if joints_by_name.insert(name, joint_index).is_some() {
                panic!("joint with that name already exists")
            }
        }
        for PushDef {
            alpha_name,
            omega_name,
            axis,
            ideal,
        } in proto.pushes
        {
            let vector = match axis {
                Axis::X => Vector3::unit_x(),
                Axis::Y => Vector3::unit_y(),
                Axis::Z => Vector3::unit_z(),
            };
            let ends = [
                (alpha_name, -vector * ideal / 2.0),
                (omega_name, vector * ideal / 2.0),
            ];
            let [alpha_index, omega_index] = ends.map(|(name, loc)| {
                let joint_index = fabric.create_joint(Point3::from_vec(loc));
                if joints_by_name.insert(name, joint_index).is_some() {
                    panic!("joint with that name already exists")
                }
                joint_index
            });
            fabric.create_interval(alpha_index, omega_index, ideal, Role::Pushing);
        }
        for PullDef {
            alpha_name,
            omega_name,
            ideal,
            material,
            ..
        } in proto.pulls
        {
            let [alpha_index, omega_index] =
                [alpha_name, omega_name].map(|name| *joints_by_name.get(&name).expect(&name));
            let role = Role::from_label(&material)
                .expect(&format!("Unknown role label: {}", material));
            fabric.create_interval(
                alpha_index,
                omega_index,
                ideal,
                role,
            );
        }
        for FaceDef {
            aliases,
            joint_names,
            spin,
        } in proto.faces
        {
            let joint_indices = joint_names
                .map(|name| *joints_by_name.get(&name).expect("no joint with that name"));
            let joints = joint_indices.map(|index| fabric.joints[index].location.to_vec());
            let midpoint = joints.into_iter().sum::<Vector3<_>>() / 3.0;
            let alpha_index = fabric.create_joint(Point3::from_vec(midpoint));
            let radial_intervals = joint_indices.map(|omega_index| {
                fabric.create_interval(alpha_index, omega_index, 1.0, Role::FaceRadial)
            });
            fabric.create_face(aliases, 1.0, spin, radial_intervals);
        }
        fabric.check_orphan_joints();
        fabric
    }
}

// Tenscript parsing removed - bricks are now defined using Rust DSL in brick_builders.rs

#[derive(Debug, Clone, Default)]
pub struct BrickFace {
    pub joints: [usize; 3],
    pub aliases: Vec<FaceAlias>,
    pub spin: Spin,
}

impl BrickFace {
    pub fn vector_space(&self, baked: &Baked) -> Matrix4<f32> {
        let location = self.radial_locations(baked);
        let midpoint = Self::midpoint(location);
        let radial = self.radial_vectors(location);
        let inward = match self.spin {
            Left => radial[1].cross(radial[2]),
            Right => radial[2].cross(radial[1]),
        }
        .normalize();
        let (x_axis, y_axis, scale) = (radial[0].normalize(), inward, radial[0].magnitude());
        let z_axis = x_axis.cross(y_axis).normalize();
        Matrix4::from_translation(midpoint)
            * Matrix4::from(Matrix3::from_cols(x_axis, y_axis, z_axis))
            * Matrix4::from_scale(scale)
    }

    pub fn normal(&self, baked: &Baked) -> Vector3<f32> {
        let location = self.radial_locations(baked);
        let radial = self.radial_vectors(location);
        match self.spin {
            Left => radial[2].cross(radial[1]),
            Right => radial[1].cross(radial[2]),
        }
        .normalize()
    }

    fn radial_locations(&self, baked: &Baked) -> [Vector3<f32>; 3] {
        self.joints
            .map(|index| baked.joints[index].location.to_vec())
    }

    fn midpoint(radial: [Vector3<f32>; 3]) -> Vector3<f32> {
        (radial[0] + radial[1] + radial[2]) / 3.0
    }

    fn radial_vectors(&self, location: [Vector3<f32>; 3]) -> [Vector3<f32>; 3] {
        let midpoint = Self::midpoint(location);
        location.map(|location| location - midpoint)
    }
}

#[derive(Debug, Clone)]
pub struct BakedJoint {
    pub location: Point3<f32>,
}

#[derive(Debug, Clone)]
pub struct BakedInterval {
    pub alpha_index: usize,
    pub omega_index: usize,
    pub strain: f32,
    pub material_name: String,
}

#[derive(Debug, Clone, Default)]
pub struct Baked {
    pub joints: Vec<BakedJoint>,
    pub intervals: Vec<BakedInterval>,
    pub faces: Vec<BrickFace>,
}

impl Baked {
    pub(crate) fn apply_matrix(&mut self, matrix: Matrix4<f32>) {
        for joint in &mut self.joints {
            joint.location = matrix.transform_point(joint.location)
        }
    }

    pub(crate) fn down_rotation(&self, seed: Option<usize>) -> Matrix4<f32> {
        let down = self
            .faces
            .iter()
            .filter_map(|face| {
                face.aliases
                    .iter()
                    .find(|alias| alias.is_seed(seed) && alias.is_base())
                    .map(|_| face.normal(self))
            })
            .sum::<Vector3<f32>>()
            .normalize();
        Matrix4::from(Quaternion::between_vectors(down, -Vector3::unit_y()))
    }

    pub const TARGET_FACE_STRAIN: f32 = 0.1;
    pub const TOLERANCE: f32 = 0.001;
}

impl TryFrom<Fabric> for Baked {
    type Error = String;

    fn try_from(fabric: Fabric) -> Result<Self, String> {
        let joint_incidents = fabric.joint_incidents();
        let mut strains = Vec::new();
        let mut strain_sum = 0.0;
        for face in fabric.faces.values() {
            let strain = face.strain(&fabric);
            strain_sum += strain;
            if abs(strain - Baked::TARGET_FACE_STRAIN) > Baked::TOLERANCE {
                strains.push(strain);
            }
        }
        if !strains.is_empty() {
            println!(
                "Face interval strain too far from {} {strains:?}",
                Baked::TARGET_FACE_STRAIN
            );
        }
        let average_strain = strain_sum / fabric.faces.len() as f32;
        if abs(average_strain - Baked::TARGET_FACE_STRAIN) > Baked::TOLERANCE {
            return Err(format!(
                "Face interval strain too far from (avg) {} {average_strain:?}",
                Baked::TARGET_FACE_STRAIN
            ));
        }
        let face_joints: Vec<usize> = fabric
            .faces
            .values()
            .map(|face| face.middle_joint(&fabric))
            .collect();
        Ok(Self {
            joints: joint_incidents
                .iter()
                .filter_map(
                    |JointIncident {
                         index, location, ..
                     }| {
                        if face_joints.contains(index) {
                            None
                        } else {
                            Some(BakedJoint {
                                location: *location,
                            })
                        }
                    },
                )
                .collect(),
            intervals: fabric
                .interval_values()
                .filter_map(
                    |&Interval {
                         alpha_index,
                         omega_index,
                         role,
                         strain,
                         ..
                     }| {
                        if role == Role::FaceRadial {
                            return None;
                        }
                        let material_name = role.label().to_string();
                        Some(BakedInterval {
                            alpha_index,
                            omega_index,
                            strain,
                            material_name,
                        })
                    },
                )
                .collect(),
            faces: fabric
                .faces
                .values()
                .map(|face| BrickFace {
                    joints: face.radial_joints(&fabric),
                    aliases: face.aliases.clone(),
                    spin: face.spin,
                })
                .collect(),
        })
    }
}
