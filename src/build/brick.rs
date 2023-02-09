use cgmath::{Point3, point3};
use clap::ValueEnum;
use pest::iterators::Pair;

use crate::build::tenscript::{FaceName, parse_atom, parse_name, ParseError, Spin};
use crate::build::tenscript::Rule;
use crate::build::tenscript::Spin::{Left, Right};
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
            Rule::axis_x => Axis::X,
            Rule::axis_y => Axis::Y,
            Rule::axis_z => Axis::Z,
            _ => unreachable!(),
        }
    }
}

impl Spin {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        match pair.as_rule() {
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
    pub name: String,
}

#[derive(Clone, Default, Debug)]
pub struct Prototype {
    pub pushes: Vec<PushDef>,
    pub pulls: Vec<PullDef>,
    pub faces: Vec<FaceDef>,
}

#[derive(Clone, Debug)]
pub struct BrickDefinition {
    pub name: String,
    pub proto: Prototype,
    pub baked: Option<Baked>,
}

impl Prototype {
    pub fn from_pair(pair: Pair<Rule>) -> Result<Self, ParseError> {
        let mut prototype = Self::default();
        for pair in pair.into_inner() {
            match pair.as_rule() {
                Rule::pushes_proto => {
                    let mut inner = pair.into_inner();
                    let [axis, ideal] = inner.next_chunk().unwrap();
                    let axis = Axis::from_pair(axis);
                    let ideal = ideal.as_str().parse().unwrap();
                    for push_pair in inner {
                        let (alpha_name, omega_name) = Self::extract_alpha_and_omega(push_pair);
                        prototype.pushes.push(PushDef {
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
                        prototype.pulls.push(PullDef {
                            alpha_name,
                            omega_name,
                            ideal,
                        });
                    }
                }
                Rule::faces_proto => {
                    for face_pair in pair.into_inner() {
                        let [spin, a, b, c, name] = face_pair.into_inner().next_chunk().unwrap();
                        let spin = Spin::from_pair(spin);
                        let joint_names = [a, b, c].map(parse_atom);
                        let name = parse_atom(name);
                        prototype.faces.push(FaceDef {
                            spin,
                            joint_names,
                            name,
                        });
                    }
                }
                _ => unreachable!(),
            }
        }
        // TODO: validate all the names used
        Ok(prototype)
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
        let [name, proto] = inner.next_chunk().unwrap();
        let name = parse_name(name);
        let proto = Prototype::from_pair(proto)?;
        let baked = inner.next().map(Baked::from_pair);
        Ok(Self {
            name,
            proto,
            baked,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Baked {
    pub joints: Vec<Point3<f32>>,
    pub intervals: Vec<(usize, usize, Role, f32)>,
    pub faces: Vec<([usize; 3], FaceName, Spin)>,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum, Debug, Default)]
pub enum BrickName {
    #[default]
    LeftTwist,
    RightTwist,
    LeftOmniTwist,
    RightOmniTwist,
    LeftMitosis,
    RightMitosis,
}

impl Baked {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        let mut baked = Self::default();
        for pair in pair.into_inner() {
            match pair.as_rule() {
                Rule::joint_baked => {
                    let [x, y, z] = pair
                        .into_inner()
                        .next_chunk()
                        .unwrap()
                        .map(|pair| pair.as_str().parse().unwrap());
                    baked.joints.push(point3(x, y, z));
                }
                Rule::interval_baked => {
                    let [role, alpha_index, omega_index, strain] = pair.into_inner().next_chunk().unwrap();
                    let role = match role.as_rule() {
                        Rule::push => Push,
                        Rule::pull => Pull,
                        _ => unreachable!()
                    };
                    let [alpha_index, omega_index] = [alpha_index, omega_index].map(|pair| pair.as_str().parse().unwrap());
                    let strain = strain.as_str().parse().unwrap();
                    baked.intervals.push((alpha_index, omega_index, role, strain));
                }
                Rule::face_baked => {
                    // TODO: use name instead of FaceName(0)
                    let [spin, a, b, c, _name] = pair.into_inner().next_chunk().unwrap();
                    let spin = Spin::from_pair(spin);
                    let joint_indices = [a, b, c].map(|pair| pair.as_str().parse().unwrap());
                    baked.faces.push((joint_indices, FaceName(0), spin));
                }
                _ => unreachable!()
            }
        }
        baked
    }


    pub fn new(name: BrickName) -> Baked {
        match name {
            BrickName::LeftTwist => Baked {
                joints: vec![
                    point3(-0.5000, 0.0000, 0.8660),
                    point3(-0.5000, 0.0000, -0.8660),
                    point3(1.0000, 0.0000, 0.0000),
                    point3(-0.0067, 1.9663, -1.0000),
                    point3(0.8694, 1.9663, 0.4941),
                    point3(-0.8626, 1.9663, 0.5058),
                    point3(0.0000, -0.0001, -0.0000),
                    point3(0.0000, 1.9664, -0.0000),
                ],
                intervals: vec![
                    (0, 3, Push, -0.0531),
                    (1, 3, Pull, 0.1176),
                    (2, 4, Pull, 0.1176),
                    (1, 4, Push, -0.0531),
                    (2, 5, Push, -0.0531),
                    (0, 5, Pull, 0.1176),
                ],
                faces: vec![
                    ([3, 4, 5], FaceName(1), Left),
                    ([2, 1, 0], FaceName(0), Left),
                ],
            },
            BrickName::RightTwist => Baked {
                joints: vec![
                    point3(-0.5000, -0.0000, 0.8660),
                    point3(-0.5000, 0.0000, -0.8660),
                    point3(1.0000, -0.0000, -0.0000),
                    point3(0.8694, 1.9663, -0.4942),
                    point3(-0.0067, 1.9663, 1.0000),
                    point3(-0.8627, 1.9663, -0.5058),
                    point3(-0.0000, -0.0001, -0.0000),
                    point3(-0.0000, 1.9664, -0.0000),
                ],
                intervals: vec![
                    (0, 4, Pull, 0.1176),
                    (2, 5, Push, -0.0531),
                    (2, 3, Pull, 0.1176),
                    (1, 5, Pull, 0.1176),
                    (1, 4, Push, -0.0531),
                    (0, 3, Push, -0.0531),
                ],
                faces: vec![
                    ([2, 1, 0], FaceName(0), Right),
                    ([3, 4, 5], FaceName(1), Right),
                ],
            },
            BrickName::LeftOmniTwist => Baked {
                joints: vec![
                    point3(-0.0000, 0.0001, 0.0000),
                    point3(1.0000, 0.0000, 0.0000),
                    point3(-0.5000, 0.0000, -0.8660),
                    point3(-0.5000, 0.0000, 0.8660),
                    point3(-0.0048, 1.6290, -1.1518),
                    point3(-1.0048, 1.6330, -1.1464),
                    point3(0.5000, 2.4436, -0.8660),
                    point3(0.4904, 0.8106, -1.4433),
                    point3(-0.9951, 1.6290, 0.5800),
                    point3(-0.4904, 1.6330, 1.4433),
                    point3(-1.4952, 0.8106, 0.2970),
                    point3(-1.0000, 2.4436, 0.0000),
                    point3(0.9999, 1.6290, 0.5717),
                    point3(1.4952, 1.6330, -0.2970),
                    point3(0.5000, 2.4436, 0.8660),
                    point3(1.0048, 0.8106, 1.1464),
                    point3(0.0000, 2.4434, 0.0000),
                    point3(0.0048, 0.8146, 1.1518),
                    point3(0.9951, 0.8146, -0.5800),
                    point3(-0.9999, 0.8146, -0.5717),
                ],
                intervals: vec![
                    (3, 13, Push, -0.0473),
                    (7, 14, Push, -0.0473),
                    (1, 5, Push, -0.0473),
                    (11, 15, Push, -0.0473),
                    (6, 10, Push, -0.0473),
                    (2, 9, Push, -0.0473),
                ],
                faces: vec![
                    ([13, 15, 14], FaceName(6), Right),
                    ([5, 7, 6], FaceName(2), Right),
                    ([13, 7, 1], FaceName(5), Left),
                    ([9, 10, 11], FaceName(4), Right),
                    ([1, 2, 3], FaceName(0), Right),
                    ([9, 15, 3], FaceName(3), Left),
                    ([10, 2, 5], FaceName(7), Left),
                    ([14, 11, 6], FaceName(1), Left),
                ],
            },
            BrickName::RightOmniTwist => Baked {
                joints: vec![
                    point3(0.0000, 0.0002, 0.0000),
                    point3(1.0000, 0.0000, -0.0000),
                    point3(-0.5000, 0.0000, -0.8660),
                    point3(-0.5000, 0.0000, 0.8660),
                    point3(-0.0048, 1.6290, 1.1518),
                    point3(-1.0048, 1.6330, 1.1464),
                    point3(0.4904, 0.8106, 1.4433),
                    point3(0.5000, 2.4436, 0.8660),
                    point3(0.9999, 1.6290, -0.5717),
                    point3(1.4952, 1.6330, 0.2970),
                    point3(0.5000, 2.4436, -0.8660),
                    point3(1.0048, 0.8106, -1.1464),
                    point3(-0.9951, 1.6290, -0.5800),
                    point3(-0.4904, 1.6330, -1.4433),
                    point3(-1.4952, 0.8106, -0.2970),
                    point3(-1.0000, 2.4436, -0.0000),
                    point3(0.0000, 2.4434, -0.0000),
                    point3(0.0048, 0.8146, -1.1518),
                    point3(-0.9999, 0.8146, 0.5717),
                    point3(0.9951, 0.8146, 0.5800),
                ],
                intervals: vec![
                    (1, 5, Push, -0.0473),
                    (3, 13, Push, -0.0473),
                    (2, 9, Push, -0.0473),
                    (11, 15, Push, -0.0473),
                    (7, 14, Push, -0.0473),
                    (6, 10, Push, -0.0473),
                ],
                faces: vec![
                    ([1, 2, 3], FaceName(0), Left),
                    ([5, 7, 6], FaceName(2), Left),
                    ([6, 9, 1], FaceName(7), Right),
                    ([3, 14, 5], FaceName(5), Right),
                    ([13, 15, 14], FaceName(6), Left),
                    ([9, 10, 11], FaceName(4), Left),
                    ([2, 11, 13], FaceName(3), Right),
                    ([7, 15, 10], FaceName(1), Right),
                ],
            },
            BrickName::LeftMitosis => Baked {
                joints: vec![
                    point3(1.8948, 1.4897, -0.0230),
                    point3(-0.5230, 0.0000, -0.8371),
                    point3(0.5402, 3.2664, 0.8782),
                    point3(-1.9359, 1.7408, 0.0445),
                    point3(-0.8727, 5.0072, 1.7597),
                    point3(-3.2905, 3.5175, 0.9456),
                    point3(1.1674, 2.3557, -1.2582),
                    point3(1.1240, 0.9864, 1.3761),
                    point3(-0.9188, 5.0072, 0.0856),
                    point3(-0.9622, 3.6379, 2.7199),
                    point3(-0.4336, 1.3693, -1.7973),
                    point3(-0.4770, 0.0000, 0.8371),
                    point3(-2.5197, 4.0207, -0.4535),
                    point3(-2.5631, 2.6515, 2.1809),
                    point3(1.0000, 0.0000, -0.0000),
                    point3(-2.4168, 4.3427, 2.2009),
                    point3(1.0211, 0.6644, -1.2783),
                    point3(-2.3958, 5.0072, 0.9227),
                    point3(0.0136, 0.0001, -0.0117),
                    point3(1.3613, 1.4855, -0.8503),
                    point3(-1.4305, 4.3389, 2.2197),
                    point3(-2.7359, 4.1897, 0.4877),
                    point3(1.3401, 0.8175, 0.4350),
                    point3(0.0347, 0.6682, -1.2970),
                    point3(-2.7571, 3.5216, 1.7730),
                    point3(-1.4093, 5.0070, 0.9344),
                ],
                intervals: vec![
                    (3, 12, Pull, 0.0772),
                    (10, 11, Push, -0.0408),
                    (8, 9, Push, -0.0408),
                    (2, 9, Pull, 0.0772),
                    (14, 15, Push, -0.0393),
                    (4, 5, Push, -0.0456),
                    (3, 13, Pull, 0.0772),
                    (0, 1, Push, -0.0456),
                    (3, 11, Pull, 0.0772),
                    (6, 7, Push, -0.0408),
                    (16, 17, Push, -0.0393),
                    (12, 13, Push, -0.0408),
                    (2, 8, Pull, 0.0772),
                    (2, 3, Push, -0.0229),
                    (2, 7, Pull, 0.0772),
                    (2, 6, Pull, 0.0772),
                    (3, 10, Pull, 0.0772),
                ],
                faces: vec![
                    ([14, 1, 11], FaceName(0), Left),
                    ([17, 5, 12], FaceName(6), Left),
                    ([16, 0, 6], FaceName(2), Left),
                    ([14, 7, 0], FaceName(7), Right),
                    ([17, 8, 4], FaceName(1), Right),
                    ([15, 13, 5], FaceName(3), Right),
                    ([16, 10, 1], FaceName(5), Right),
                    ([15, 4, 9], FaceName(4), Left),
                ],
            },
            BrickName::RightMitosis => Baked {
                joints: vec![
                    point3(-0.5230, 0.0000, 0.8371),
                    point3(1.8948, 1.4897, 0.0230),
                    point3(-1.9359, 1.7408, -0.0445),
                    point3(0.5402, 3.2664, -0.8782),
                    point3(-3.2905, 3.5175, -0.9456),
                    point3(-0.8727, 5.0072, -1.7597),
                    point3(-0.4336, 1.3693, 1.7973),
                    point3(-0.4770, 0.0000, -0.8371),
                    point3(-2.5197, 4.0207, 0.4535),
                    point3(-2.5631, 2.6515, -2.1809),
                    point3(1.1674, 2.3557, 1.2582),
                    point3(1.1240, 0.9864, -1.3761),
                    point3(-0.9188, 5.0072, -0.0856),
                    point3(-0.9622, 3.6379, -2.7199),
                    point3(1.0000, -0.0000, 0.0000),
                    point3(-2.4168, 4.3427, -2.2009),
                    point3(1.0211, 0.6644, 1.2783),
                    point3(-2.3958, 5.0072, -0.9227),
                    point3(1.3401, 0.8175, -0.4350),
                    point3(0.0347, 0.6682, 1.2970),
                    point3(-2.7571, 3.5216, -1.7730),
                    point3(-1.4093, 5.0070, -0.9344),
                    point3(0.0136, 0.0001, 0.0117),
                    point3(1.3613, 1.4855, 0.8503),
                    point3(-1.4305, 4.3390, -2.2197),
                    point3(-2.7359, 4.1897, -0.4877),
                ],
                intervals: vec![
                    (4, 5, Push, -0.0456),
                    (2, 8, Pull, 0.0772),
                    (8, 9, Push, -0.0408),
                    (12, 13, Push, -0.0408),
                    (6, 7, Push, -0.0408),
                    (2, 6, Pull, 0.0772),
                    (2, 7, Pull, 0.0772),
                    (2, 9, Pull, 0.0772),
                    (3, 13, Pull, 0.0772),
                    (3, 10, Pull, 0.0772),
                    (0, 1, Push, -0.0456),
                    (3, 12, Pull, 0.0772),
                    (16, 17, Push, -0.0393),
                    (10, 11, Push, -0.0408),
                    (14, 15, Push, -0.0393),
                    (3, 11, Pull, 0.0772),
                    (2, 3, Push, -0.0229),
                ],
                faces: vec![
                    ([16, 10, 1], FaceName(2), Right),
                    ([17, 8, 4], FaceName(6), Right),
                    ([15, 13, 5], FaceName(4), Right),
                    ([15, 4, 9], FaceName(3), Left),
                    ([16, 0, 6], FaceName(5), Left),
                    ([14, 7, 0], FaceName(0), Right),
                    ([17, 5, 12], FaceName(1), Left),
                    ([14, 1, 11], FaceName(7), Left),
                ],
            },
        }
    }
}
