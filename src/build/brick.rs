use std::collections::HashMap;
use std::iter;
use std::sync::LazyLock;

use cgmath::{EuclideanSpace, InnerSpace, Matrix3, Matrix4, Point3, point3, Quaternion, Rotation, SquareMatrix, Transform, Vector3};
use pest::iterators::Pair;

use crate::build::tenscript::{FaceAlias, Library, parse_atom, ParseError, Spin};
use crate::build::tenscript::Rule;
use crate::build::tenscript::Spin::{Left, Right};
use crate::fabric::{Fabric, Link};
use crate::fabric::interval::Role;
use crate::fabric::interval::Role::{Pull, Push};

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
}

#[derive(Clone, Debug)]
pub struct PullDef {
    pub ideal: f32,
    pub alpha_name: String,
    pub omega_name: String,
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
        for PushDef { alpha_name, omega_name, axis, ideal } in proto.pushes {
            let vector = match axis {
                Axis::X => Vector3::unit_x(),
                Axis::Y => Vector3::unit_y(),
                Axis::Z => Vector3::unit_z(),
            };
            let ends = [(alpha_name, -vector * ideal / 2.0), (omega_name, vector * ideal / 2.0)];
            let [alpha_index, omega_index] = ends.map(|(name, loc)| {
                let joint_index = fabric.create_joint(Point3::from_vec(loc));
                if joints_by_name.insert(name, joint_index).is_some() {
                    panic!("joint with that name already exists")
                }
                joint_index
            });
            fabric.create_interval(alpha_index, omega_index, Link::push(ideal));
        }
        for PullDef { alpha_name, omega_name, ideal } in proto.pulls {
            let [alpha_index, omega_index] = [alpha_name, omega_name]
                .map(|name| *joints_by_name.get(&name).expect("no joint with that name"));
            fabric.create_interval(alpha_index, omega_index, Link::pull(ideal));
        }
        for FaceDef { aliases, joint_names, spin } in proto.faces {
            let joint_indices = joint_names.map(|name| *joints_by_name.get(&name).expect("no joint with that name"));
            let joints = joint_indices.map(|index| fabric.joints[index].location.to_vec());
            let midpoint = joints.into_iter().sum::<Vector3<_>>() / 3.0;
            let alpha_index = fabric.create_joint(Point3::from_vec(midpoint));
            let radial_intervals = joint_indices.map(|omega_index| {
                fabric.create_interval(alpha_index, omega_index, Link::pull(1.0))
            });
            fabric.create_face(aliases, 1.0, spin, radial_intervals);
        }
        fabric
    }
}

impl Prototype {
    pub fn from_pair(pair: Pair<Rule>) -> Result<Self, ParseError> {
        let mut inner = pair.into_inner();
        let prototype_alias = FaceAlias::from_pair(inner.next().unwrap());
        let mut pushes = Vec::new();
        let mut pulls = Vec::new();
        let mut faces = Vec::new();
        for pair in inner {
            match pair.as_rule() {
                Rule::pushes_proto => {
                    let mut inner = pair.into_inner();
                    let [axis, ideal] = inner.next_chunk().unwrap();
                    let axis = Axis::from_pair(axis);
                    let ideal = ideal.as_str().parse().unwrap();
                    for push_pair in inner {
                        let (alpha_name, omega_name) = Self::extract_alpha_and_omega(push_pair);
                        pushes.push(PushDef {
                            alpha_name,
                            omega_name,
                            ideal,
                            axis,
                        })
                    }
                }
                Rule::pulls_proto => {
                    let mut inner = pair.into_inner();
                    let ideal = inner.next().unwrap().as_str().parse().unwrap();
                    for pull_pair in inner {
                        let (alpha_name, omega_name) = Self::extract_alpha_and_omega(pull_pair);
                        pulls.push(PullDef {
                            alpha_name,
                            omega_name,
                            ideal,
                        });
                    }
                }
                Rule::faces_proto => {
                    for face_pair in pair.into_inner() {
                        let mut inner = face_pair.into_inner();
                        let [spin, a, b, c] = inner.next_chunk().unwrap();
                        let joint_names = [a, b, c].map(parse_atom);
                        let mut aliases = FaceAlias::from_pairs(inner);
                        aliases = aliases.into_iter().map(|a| prototype_alias.clone() + &a).collect();
                        let spin = Spin::from_pair(spin);
                        faces.push(FaceDef { spin, joint_names, aliases });
                    }
                }
                _ => unreachable!("{:?}", pair.as_rule()),
            }
        }
        Ok(Prototype { alias: prototype_alias, pushes, pulls, faces })
    }

    fn extract_alpha_and_omega(pair: Pair<Rule>) -> (String, String) {
        let [alpha_name, omega_name] = pair
            .into_inner()
            .next_chunk()
            .unwrap()
            .map(parse_atom);
        (alpha_name, omega_name)
    }
}

impl BrickDefinition {
    pub fn from_pair(pair: Pair<Rule>) -> Result<Self, ParseError> {
        let mut inner = pair.into_inner();
        let proto = Prototype::from_pair(inner.next().unwrap())?;
        let baked = inner.next().map(Baked::from_pair);
        Ok(Self {
            proto,
            baked,
        })
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
        }.normalize();
        let (x_axis, y_axis, scale) =
            (radial[0].normalize(), inward, radial[0].magnitude());
        let z_axis = x_axis.cross(y_axis).normalize();
        Matrix4::from_translation(midpoint) *
            Matrix4::from(Matrix3::from_cols(x_axis, y_axis, z_axis)) *
            Matrix4::from_scale(scale)
    }

    pub fn normal(&self, baked: &Baked) -> Vector3<f32> {
        let location = self.radial_locations(baked);
        let radial = self.radial_vectors(location);
        match self.spin {
            Left => radial[2].cross(radial[1]),
            Right => radial[1].cross(radial[2]),
        }.normalize()
    }

    fn radial_locations(&self, baked: &Baked) -> [Vector3<f32>; 3] {
        self.joints.map(|index| baked.joints[index].to_vec())
    }

    fn midpoint(radial: [Vector3<f32>; 3]) -> Vector3<f32> {
        (radial[0] + radial[1] + radial[2]) / 3.0
    }

    fn radial_vectors(&self, location: [Vector3<f32>; 3]) -> [Vector3<f32>; 3] {
        let midpoint = Self::midpoint(location);
        location.map(|location| location - midpoint)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Baked {
    pub alias: FaceAlias,
    pub joints: Vec<Point3<f32>>,
    pub intervals: Vec<(usize, usize, Role, f32)>,
    pub faces: Vec<BrickFace>,
}

impl Baked {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        let mut inner = pair.into_inner();
        let baked_alias = FaceAlias::from_pair(inner.next().unwrap());
        let mut joints = Vec::new();
        let mut intervals = Vec::new();
        let mut faces = Vec::new();
        for pair in inner {
            match pair.as_rule() {
                Rule::joint_baked => {
                    let [x, y, z] = pair
                        .into_inner()
                        .next_chunk()
                        .unwrap()
                        .map(|pair| pair.as_str().parse().unwrap());
                    joints.push(point3(x, y, z));
                }
                Rule::interval_baked => {
                    let [role, alpha_index, omega_index, strain] = pair.into_inner().next_chunk().unwrap();
                    let role = match role.into_inner().next().unwrap().as_rule() {
                        Rule::push => Push,
                        Rule::pull => Pull,
                        _ => unreachable!()
                    };
                    let [alpha_index, omega_index] = [alpha_index, omega_index].map(|pair| pair.as_str().parse().unwrap());
                    let strain = strain.as_str().parse().unwrap();
                    intervals.push((alpha_index, omega_index, role, strain));
                }
                Rule::face_baked => {
                    let mut inner = pair.into_inner();
                    let [spin, a, b, c] = inner.next_chunk().unwrap();
                    let mut aliases = FaceAlias::from_pairs(inner);
                    aliases = aliases.into_iter().map(|a| baked_alias.clone() + &a).collect();
                    let spin = Spin::from_pair(spin);
                    let joints = [a, b, c].map(|pair| pair.as_str().parse().unwrap());
                    faces.push(BrickFace { joints, spin, aliases });
                }
                _ => unreachable!()
            }
        }
        Baked { alias: baked_alias, joints, intervals, faces }
    }

    fn apply_matrix(&mut self, matrix: Matrix4<f32>) {
        for joint in &mut self.joints {
            *joint = matrix.transform_point(*joint)
        }
    }

    fn down_rotation(&self) -> Matrix4<f32> {
        let down = self.faces
            .iter()
            .filter_map(|face|
                face.aliases
                    .iter()
                    .find(|alias| alias.is_seed() && alias.is_base())
                    .map(|_| face.normal(self)))
            .sum::<Vector3<f32>>()
            .normalize();
        Matrix4::from(Quaternion::between_vectors(down, -Vector3::unit_y()))
    }

    pub fn new_brick(search_alias: &FaceAlias) -> Baked {
        static BAKED_BRICKS: LazyLock<Vec<(FaceAlias, Baked)>> = LazyLock::new(|| {
            Library::standard()
                .bricks
                .into_iter()
                .filter_map(|brick| brick.baked)
                .flat_map(|baked| {
                    let cloned_bricks = iter::repeat(baked.clone());
                    baked
                        .faces
                        .into_iter()
                        .zip(cloned_bricks)
                        .flat_map(|(face, baked)| {
                            let face_space = face.vector_space(&baked).invert().unwrap();
                            let aliases: Vec<_> = face.aliases
                                .into_iter()
                                .map(|alias| {
                                    let space = if alias.is_seed() {
                                        baked.down_rotation()
                                    } else {
                                        face_space
                                    };
                                    (alias, space)
                                })
                                .collect();
                            aliases
                                .into_iter()
                                .map(move |(alias, space)| {
                                    let alias = alias + &baked.alias;
                                    let mut baked = baked.clone();
                                    baked.apply_matrix(space);
                                    (alias, baked)
                                })
                        })
                })
                .collect()
        });
        let search_with_base = search_alias.with_base();
        let (_, baked) = &BAKED_BRICKS
            .iter()
            .filter(|(baked_alias, _)| search_with_base.matches(baked_alias))
            .min_by_key(|(brick_alias, _)| brick_alias.0.len())
            .expect(&format!("no such brick: '{search_with_base}'"));
        let mut thawed = baked.clone();
        for face in &mut thawed.faces {
            face.aliases.retain(|candidate| search_alias.matches(candidate));
            assert_eq!(face.aliases.len(), 1, "exactly one face should be retained {:?}", face.aliases);
        }
        thawed.clone()
    }
}

