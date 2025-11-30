use std::collections::HashMap;

use cgmath::{
    EuclideanSpace, InnerSpace, Matrix3, Matrix4, Point3, Quaternion, Rotation, Transform, Vector3,
};

use crate::build::dsl::brick_dsl::FaceName::Downwards;
use crate::build::dsl::brick_dsl::{BrickName, BrickParams, BrickRole, JointName};
use crate::build::dsl::Spin::{Left, Right};
use crate::build::dsl::{FaceAlias, ScaleMode, Spin};
use crate::fabric::interval::Role;
use crate::fabric::Fabric;

#[derive(Copy, Clone, Debug)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    /// Create a push interval definition along this axis
    pub fn push(self, ideal: f32, alpha: JointName, omega: JointName) -> PushDef {
        PushDef {
            axis: self,
            ideal,
            alpha,
            omega,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PushDef {
    pub axis: Axis,
    pub ideal: f32,
    pub alpha: JointName,
    pub omega: JointName,
}

#[derive(Clone, Debug)]
pub struct PullDef {
    pub alpha: JointName,
    pub omega: JointName,
    pub ideal: f32,
    pub material: String,
}

#[derive(Clone, Debug)]
pub struct FaceDef {
    pub spin: Spin,
    pub joints: [JointName; 3],
    pub aliases: Vec<FaceAlias>,
    pub scale_overrides: Vec<(ScaleMode, f32)>,
}

impl FaceDef {
    /// Get the scale factor for a given scaling scheme
    pub fn scale_for(&self, scaling: ScaleMode) -> f32 {
        match scaling {
            ScaleMode::None => 1.0,
            scheme => self
                .scale_overrides
                .iter()
                .find(|(s, _)| *s == scheme)
                .map(|(_, scale)| *scale)
                .unwrap_or(1.0),
        }
    }
}

#[derive(Clone, Debug)]
pub struct BrickPrototype {
    pub brick_name: BrickName,
    pub brick_roles: Vec<BrickRole>,
    pub scale_modes: Vec<ScaleMode>,
    pub joints: Vec<JointName>,
    pub pushes: Vec<PushDef>,
    pub pulls: Vec<PullDef>,
    pub faces: Vec<FaceDef>,
}

impl BrickPrototype {
    /// Get the maximum seed role (the one with the most downward faces)
    pub fn max_seed(&self) -> BrickRole {
        self.brick_roles
            .iter()
            .filter_map(|role| match role {
                BrickRole::Seed(n) => Some((*role, *n)),
                _ => None,
            })
            .max_by_key(|(_, n)| *n)
            .map(|(role, _)| role)
            .expect("BrickPrototype has no Seed roles")
    }

    /// Derive baked faces from the prototype
    pub fn derive_baked_faces(&self, scale_mode: ScaleMode) -> Vec<BrickFace> {
        let mut joint_map = HashMap::new();

        for (idx, joint_name) in self.joints.iter().enumerate() {
            joint_map.insert(*joint_name, idx);
        }

        let offset = self.joints.len();
        for (idx, push) in self.pushes.iter().enumerate() {
            let alpha_idx = offset + idx * 2;
            let omega_idx = offset + idx * 2 + 1;
            joint_map.insert(push.alpha, alpha_idx);
            joint_map.insert(push.omega, omega_idx);
        }

        self.faces
            .iter()
            .map(|face_def| {
                let joints = [
                    *joint_map.get(&face_def.joints[0]).expect("Joint not found"),
                    *joint_map.get(&face_def.joints[1]).expect("Joint not found"),
                    *joint_map.get(&face_def.joints[2]).expect("Joint not found"),
                ];
                BrickFace {
                    spin: face_def.spin,
                    joints,
                    aliases: face_def.aliases.clone(),
                    scale: face_def.scale_for(scale_mode),
                }
            })
            .collect()
    }
}

impl BrickPrototype {
    /// Convert prototype to fabric for baking, applying face scaling
    pub fn to_fabric(&self, face_scaling: ScaleMode) -> Fabric {
        let mut fabric = Fabric::new("prototype".to_string());
        let mut joints_by_name: HashMap<JointName, usize> = HashMap::new();
        for name in &self.joints {
            let joint_index = fabric.create_joint(Point3::origin());
            if joints_by_name.insert(*name, joint_index).is_some() {
                panic!("joint with that name already exists")
            }
        }
        for push in &self.pushes {
            let vector = match push.axis {
                Axis::X => Vector3::unit_x(),
                Axis::Y => Vector3::unit_y(),
                Axis::Z => Vector3::unit_z(),
            };
            let ideal = push.ideal;
            let ends = [
                (push.alpha, -vector * ideal / 2.0),
                (push.omega, vector * ideal / 2.0),
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
        for pull in &self.pulls {
            let [alpha_index, omega_index] = [pull.alpha, pull.omega].map(|name| {
                *joints_by_name
                    .get(&name)
                    .expect(&format!("Joint {:?} not found", name))
            });
            let role = Role::from_label(&pull.material)
                .expect(&format!("Unknown role label: {}", pull.material));
            fabric.create_interval(alpha_index, omega_index, pull.ideal, role);
        }
        for face_def in &self.faces {
            let joint_indices = face_def.joints.map(|name| {
                *joints_by_name
                    .get(&name)
                    .expect(&format!("Joint {:?} not found", name))
            });
            let face_scale = face_def.scale_for(face_scaling);
            // Start face center at origin - radial tensions will pull it into position
            let alpha_index = fabric.create_joint(Point3::origin());
            let radial_intervals = joint_indices.map(|omega_index| {
                fabric.create_interval(alpha_index, omega_index, face_scale, Role::FaceRadial)
            });
            fabric.create_face(
                face_def.aliases.clone(),
                face_scale,
                face_def.spin,
                radial_intervals,
            );
        }
        fabric.check_orphan_joints();
        fabric
    }
}

#[derive(Debug, Clone, Default)]
pub struct BrickFace {
    pub joints: [usize; 3],
    pub aliases: Vec<FaceAlias>,
    pub spin: Spin,
    pub scale: f32,
}

impl BrickFace {
    pub fn vector_space(&self, baked: &BakedBrick) -> Matrix4<f32> {
        let location = self.radial_locations(baked);
        let midpoint = Self::midpoint(location);
        let radial = self.radial_vectors(location);
        let inward = match self.spin {
            Left => radial[1].cross(radial[2]),
            Right => radial[2].cross(radial[1]),
        };
        let (x_axis, y_axis, scale) = (
            radial[0].normalize(),
            inward.normalize(),
            radial[0].magnitude(),
        );
        let z_axis = x_axis.cross(y_axis).normalize();
        Matrix4::from_translation(midpoint)
            * Matrix4::from(Matrix3::from_cols(x_axis, y_axis, z_axis))
            * Matrix4::from_scale(scale)
    }

    pub fn normal(&self, baked: &BakedBrick) -> Vector3<f32> {
        let location = self.radial_locations(baked);
        let radial = self.radial_vectors(location);
        let direction = match self.spin {
            Left => radial[2].cross(radial[1]),
            Right => radial[1].cross(radial[2]),
        };
        direction.normalize()
    }

    fn radial_locations(&self, baked: &BakedBrick) -> [Vector3<f32>; 3] {
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

#[derive(Debug, Clone)]
pub struct BakedBrick {
    pub params: BrickParams,
    pub scale: f32,
    pub joints: Vec<BakedJoint>,
    pub intervals: Vec<BakedInterval>,
    pub faces: Vec<BrickFace>,
}

impl BakedBrick {
    pub fn apply_matrix(&mut self, matrix: Matrix4<f32>) {
        for joint in &mut self.joints {
            joint.location = matrix.transform_point(joint.location)
        }
    }

    pub fn down_rotation(&self, brick_role: BrickRole) -> Matrix4<f32> {
        let downward_count = match brick_role {
            BrickRole::Seed(downward_count) => downward_count,
            _ => {
                panic!("Brick role {:?} is not a seed", brick_role);
            }
        };
        let downward_faces: Vec<_> = self
            .faces
            .iter()
            .filter_map(|face| {
                face.aliases
                    .iter()
                    .find(|alias| alias.face_name == Downwards(downward_count))
                    .map(|_| face.normal(self))
            })
            .collect();
        if downward_faces.len() != downward_count {
            panic!(
                "{:?} but found {} downward faces",
                brick_role,
                downward_faces.len()
            );
        }
        let down = downward_faces.into_iter().sum::<Vector3<f32>>().normalize();
        Matrix4::from(Quaternion::between_vectors(down, -Vector3::unit_y()))
    }

    pub const TARGET_FACE_STRAIN: f32 = 0.1;
    pub const TOLERANCE: f32 = 0.001;
}
