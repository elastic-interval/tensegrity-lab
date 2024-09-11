use std::collections::HashMap;

use cgmath::{
    EuclideanSpace, InnerSpace, Matrix3, Matrix4, point3, Point3, Quaternion, Rotation, Transform,
    Vector3,
};
use cgmath::num_traits::abs;
use pest::iterators::Pair;

use crate::build::tenscript::{FaceAlias, parse_atom, Spin, TenscriptError};
use crate::build::tenscript::Rule;
use crate::build::tenscript::Spin::{Left, Right};
use crate::fabric::Fabric;
use crate::fabric::interval::{FACE_RADIAL_GROUP, Interval};
use crate::fabric::joint_incident::JointIncident;
use crate::fabric::material::{interval_material, Material, material_by_label};

#[derive(Copy, Clone, Debug)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::axis => Self::from_pair(pair.into_inner().next().unwrap()),
            Rule::axis_x => Axis::X,
            Rule::axis_y => Axis::Y,
            Rule::axis_z => Axis::Z,
            _ => unreachable!("{:?}", pair.as_rule()),
        }
    }
}

impl Spin {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::chirality => Self::from_pair(pair.into_inner().next().unwrap()),
            Rule::left => Left,
            Rule::right => Right,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PushDef {
    pub axis: Axis,
    pub ideal: f32,
    pub alpha_name: String,
    pub omega_name: String,
    pub group: usize,
}

impl PushDef {
    fn from_pair(pair: Pair<Rule>, axis: Axis, ideal: f32, group: usize) -> Self {
        let mut walk = pair.into_inner();
        let alpha_name = parse_atom(walk.next().unwrap());
        let omega_name = parse_atom(walk.next().unwrap());
        Self { alpha_name, omega_name, ideal, axis, group }
    }
}

#[derive(Clone, Debug)]
pub struct PullDef {
    pub alpha_name: String,
    pub omega_name: String,
    pub ideal: f32,
    pub material: String,
    pub group: usize,
}

impl PullDef {
    fn from_pair(pair: Pair<Rule>, ideal: f32, group: usize) -> Self {
        let mut walk = pair.into_inner();
        let alpha_name = parse_atom(walk.next().unwrap());
        let omega_name = parse_atom(walk.next().unwrap());
        let material = walk.next().unwrap().as_str().parse().unwrap();
        Self { alpha_name, omega_name, ideal, material, group }
    }
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

impl From<Prototype> for Fabric {
    fn from(proto: Prototype) -> Self {
        let mut fabric = Fabric::default();
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
            group,
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
            fabric.create_interval(alpha_index, omega_index, ideal, Material::PushMaterial, group);
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
            fabric.create_interval(
                alpha_index,
                omega_index,
                ideal,
                material_by_label(material),
                0,
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
                fabric.create_interval(alpha_index, omega_index, 1.0, Material::PullMaterial, FACE_RADIAL_GROUP)
            });
            fabric.create_face(aliases, 1.0, spin, radial_intervals);
        }
        fabric.check_orphan_joints();
        fabric
    }
}

impl Prototype {
    pub fn from_pair(pair: Pair<Rule>) -> Result<Self, TenscriptError> {
        let mut inner = pair.into_inner();
        let alias = FaceAlias::from_pair(inner.next().unwrap());
        let mut joints = Vec::new();
        let mut pushes = Vec::new();
        let mut pulls = Vec::new();
        let mut faces = Vec::new();
        let mut group_index: usize = 0;
        for pair in inner {
            match pair.as_rule() {
                Rule::joints_proto => {
                    let inner = pair.into_inner();
                    for joint_pair in inner {
                        joints.push(parse_atom(joint_pair));
                    }
                }
                Rule::pushes_proto => {
                    let mut inner = pair.into_inner();
                    let [axis, ideal] = [inner.next().unwrap(), inner.next().unwrap()];
                    let axis = Axis::from_pair(axis);
                    let ideal = ideal.as_str().parse().unwrap();
                    for push_pair in inner {
                        pushes.push(PushDef::from_pair(push_pair, axis, ideal, group_index));
                    }
                    group_index += 1;
                }
                Rule::pulls_proto => {
                    let mut inner = pair.into_inner();
                    let ideal = inner.next().unwrap().as_str().parse().unwrap();
                    for pull_pair in inner {
                        pulls.push(PullDef::from_pair(pull_pair, ideal, group_index));
                    }
                    group_index += 1;
                }
                Rule::faces_proto => {
                    for face_pair in pair.into_inner() {
                        let mut inner = face_pair.into_inner();
                        let [spin, a, b, c] = [
                            inner.next().unwrap(),
                            inner.next().unwrap(),
                            inner.next().unwrap(),
                            inner.next().unwrap(),
                        ];
                        let joint_names = [a, b, c].map(parse_atom);
                        let mut aliases = FaceAlias::from_pairs(inner);
                        aliases = aliases
                            .into_iter()
                            .map(|a| alias.clone() + &a)
                            .collect();
                        let spin = Spin::from_pair(spin);
                        faces.push(FaceDef {
                            spin,
                            joint_names,
                            aliases,
                        });
                    }
                }
                Rule::face_aliases => {
                    let mut inner = pair.into_inner();
                    let with_atoms = FaceAlias::from_pair(inner.next().unwrap());
                    let aliases = FaceAlias::from_pairs(inner);
                    if aliases.len() != faces.len() {
                        return Err(TenscriptError::FaceAliasError("face-aliases must have the same size as faces".to_string()));
                    }
                    for (index, face_alias) in aliases.into_iter().enumerate() {
                        faces[index].aliases.push(face_alias + &with_atoms + &alias);
                    }
                }
                _ => unreachable!("{:?}", pair.as_rule()),
            }
        }
        Ok(Prototype { alias, joints, pushes, pulls, faces })
    }
}

impl BrickDefinition {
    pub fn from_pair(pair: Pair<Rule>) -> Result<Self, TenscriptError> {
        let mut inner = pair.into_inner();
        let proto = Prototype::from_pair(inner.next().unwrap())?;
        let baked = inner.next().map(Baked::from_pair);
        Ok(Self { proto, baked })
    }
}

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
        self.joints.map(|index| baked.joints[index].location.to_vec())
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
    pub(crate) location: Point3<f32>,
}

#[derive(Debug, Clone)]
pub struct BakedInterval {
    pub alpha_index: usize,
    pub omega_index: usize,
    pub material_name: String,
    pub strain: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Baked {
    pub joints: Vec<BakedJoint>,
    pub intervals: Vec<BakedInterval>,
    pub faces: Vec<BrickFace>,
}

impl Baked {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        let mut joints = Vec::new();
        let mut intervals = Vec::new();
        let mut faces = Vec::new();
        for pair in pair.into_inner() {
            match pair.as_rule() {
                Rule::joint_baked => {
                    let mut inner = pair.into_inner();
                    let [x, y, z] = [
                        inner.next().unwrap().as_str().parse().unwrap(),
                        inner.next().unwrap().as_str().parse().unwrap(),
                        inner.next().unwrap().as_str().parse().unwrap(),
                    ];
                    joints.push(BakedJoint { location: point3(x, y, z) });
                }
                Rule::interval_baked => {
                    let mut inner = pair.into_inner();
                    let [alpha_index, omega_index, strain, material] = [
                        inner.next().unwrap(),
                        inner.next().unwrap(),
                        inner.next().unwrap(),
                        inner.next().unwrap(),
                    ];
                    let [alpha_index, omega_index] =
                        [alpha_index, omega_index].map(|pair| pair.as_str().parse().unwrap());
                    let strain = strain.as_str().parse().unwrap();
                    let material = material.as_str().to_string();
                    intervals.push(BakedInterval {
                        alpha_index,
                        omega_index,
                        strain,
                        material_name: material,
                    });
                }
                Rule::face_baked => {
                    let mut inner = pair.into_inner();
                    let [spin, a, b, c] = [
                        inner.next().unwrap(),
                        inner.next().unwrap(),
                        inner.next().unwrap(),
                        inner.next().unwrap(),
                    ];
                    let aliases = FaceAlias::from_pairs(inner);
                    let spin = Spin::from_pair(spin);
                    let joints = [a, b, c].map(|pair| pair.as_str().parse().unwrap());
                    faces.push(BrickFace { joints, spin, aliases });
                }
                _ => unreachable!(),
            }
        }
        Baked {
            joints,
            intervals,
            faces,
        }
    }

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

    pub fn into_tenscript(self) -> String {
        format!(
            "\t\t(baked\n\t\t\t{joints}\n\t\t\t{intervals}\n\t\t\t{faces})",
            joints = self
                .joints
                .into_iter()
                .map(|BakedJoint { location, .. }| location)
                .map(|Point3 { x, y, z }| format!("(joint {x:.4} {y:.4} {z:.4})"))
                .collect::<Vec<_>>()
                .join("\n\t\t\t"),
            intervals = self
                .intervals
                .into_iter()
                .map(
                    |BakedInterval {
                         alpha_index,
                         omega_index,
                         material_name,
                         strain,
                     }| {
                        format!("(interval {alpha_index} {omega_index} {strain:.4} {material_name})")
                    }
                )
                .collect::<Vec<_>>()
                .join("\n\t\t\t"),
            faces = self
                .faces
                .into_iter()
                .map(
                    |BrickFace {
                         joints: [a, b, c],
                         aliases,
                         spin,
                     }| format!(
                        "({spin} {a} {b} {c} {aliases})",
                        spin = match spin {
                            Left => "left",
                            Right => "right",
                        },
                        aliases = aliases
                            .into_iter()
                            .map(|alias| format!("(alias {})", alias.into_vec().join(" ")))
                            .collect::<Vec<_>>()
                            .join(" "),
                    )
                )
                .collect::<Vec<_>>()
                .join("\n\t\t\t")
        )
    }
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
            println!("Face interval strain too far from {} {strains:?}", Baked::TARGET_FACE_STRAIN);
        }
        let average_strain = strain_sum / fabric.faces.len() as f32;
        if abs(average_strain - Baked::TARGET_FACE_STRAIN) > Baked::TOLERANCE {
            return Err(format!("Face interval strain too far from (avg) {} {average_strain:?}", Baked::TARGET_FACE_STRAIN));
        }
        let face_joints: Vec<usize> = fabric
            .faces
            .values()
            .map(|face| face.middle_joint(&fabric))
            .collect();
        Ok(Self {
            joints: joint_incidents
                .iter()
                .filter_map(|JointIncident { index, location, .. }| {
                    if face_joints.contains(index) {
                        None
                    } else {
                        Some(BakedJoint { location: *location })
                    }
                })
                .collect(),
            intervals: fabric
                .interval_values()
                .filter_map(
                    |&Interval {
                        alpha_index,
                        omega_index,
                        material,
                        group,
                        strain,
                        ..
                    }| {
                        if group == FACE_RADIAL_GROUP {
                            return None;
                        }
                        let material_name = interval_material(material).label.to_string();
                        Some(BakedInterval { alpha_index, omega_index, strain, material_name })
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
