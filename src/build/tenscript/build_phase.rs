use cgmath::{EuclideanSpace, InnerSpace, Matrix4, Quaternion, Rotation, Vector3};
use pest::iterators::Pair;

use crate::build::tenscript::{FaceMark, FaceName, Spin};
use crate::build::tenscript::build_phase::BuildNode::{*};
use crate::build::tenscript::build_phase::Launch::{*};
use crate::build::tenscript::FaceName::Apos;
use crate::build::tenscript::parser::{ParseError, Rule};
use crate::fabric::{Fabric, UniqueId};

#[derive(Debug, Default, Clone)]
pub struct Bud {
    face_id: UniqueId,
    forward: String,
    scale_factor: f32,
    node: Option<BuildNode>,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum SeedType {
    #[default]
    Left,
    Right,
    LeftRight,
    RightLeft,
}

#[derive(Debug, Clone)]
pub enum BuildNode {
    Face {
        face_name: FaceName,
        node: Box<BuildNode>,
    },
    Grow {
        forward: String,
        scale_factor: f32,
        post_growth_node: Option<Box<BuildNode>>,
    },
    Mark {
        mark_name: String,
    },
    Branch {
        face_nodes: Vec<BuildNode>,
    },
}

impl Seed {
    pub fn spin(&self) -> Spin {
        match self.seed_type {
            SeedType::Left | SeedType::LeftRight => Spin::Left,
            SeedType::Right | SeedType::RightLeft => Spin::Right,
        }
    }

    pub fn needs_double(&self) -> bool {
        match self.seed_type {
            SeedType::Left | SeedType::Right => false,
            SeedType::LeftRight | SeedType::RightLeft => true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Seed {
    pub seed_type: SeedType,
    pub down_faces: Vec<FaceName>,
}

#[derive(Debug)]
enum Launch {
    Seeded { seed: Seed },
    NamedFace { face_name: FaceName },
    IdentifiedFace { face_id: UniqueId },
}

#[derive(Debug, Default, Clone)]
pub struct BuildPhase {
    pub seed: Seed,
    pub root: Option<BuildNode>,
    pub pretenst_factor: Option<f32>,
    pub buds: Vec<Bud>,
    pub marks: Vec<FaceMark>,
}

impl BuildPhase {
    pub(crate) fn from_pair(pair: Pair<Rule>) -> Result<BuildPhase, ParseError> {
        let mut phase = BuildPhase::default();
        for sub_pair in pair.into_inner() {
            match sub_pair.as_rule() {
                Rule::seed => {
                    let mut inner = sub_pair.into_inner();
                    phase.seed.seed_type = match inner.next().unwrap().as_str() {
                        ":left-right" => SeedType::LeftRight,
                        ":right-left" => SeedType::RightLeft,
                        ":left" => SeedType::Left,
                        ":right" => SeedType::Right,
                        _ => unreachable!()
                    };
                    for sub_pair in inner {
                        match sub_pair.as_rule() {
                            Rule::orient_down => {
                                phase.seed.down_faces = sub_pair
                                    .into_inner()
                                    .map(|face_name| face_name.as_str().try_into().unwrap())
                                    .collect();
                            }
                            _ => unreachable!("build phase seed: {sub_pair:?}")
                        }
                    }
                }
                Rule::build_node => {
                    phase.root = Some(Self::parse_build_node(sub_pair).unwrap());
                }
                _ => unreachable!("build phase"),
            }
        }
        Ok(phase)
    }

    fn parse_build_node(node_pair: Pair<Rule>) -> Result<BuildNode, ParseError> {
        let pair = node_pair.into_inner().next().unwrap();
        match pair.as_rule() {
            Rule::face => {
                let [face_name_pair, node_pair] = pair.into_inner().next_chunk().unwrap();
                let face_name = face_name_pair.as_str().try_into().unwrap();
                let node = Self::parse_build_node(node_pair).unwrap();
                Ok(Face {
                    face_name,
                    node: Box::new(node),
                })
            }
            Rule::grow => {
                let mut inner = pair.into_inner();
                let forward_string = inner.next().unwrap().as_str();
                let forward = match forward_string.parse() {
                    Ok(count) => { "X".repeat(count) }
                    Err(_) => { forward_string[1..forward_string.len() - 1].into() }
                };
                let scale_factor = Self::parse_scale(inner.next());
                let post_growth_node = inner.next()
                    .map(|post_growth| Box::new(Self::parse_build_node(post_growth).unwrap()));
                Ok(Grow {
                    forward,
                    scale_factor,
                    post_growth_node,
                })
            }
            Rule::mark => {
                let mark_name = pair.into_inner().next().unwrap().as_str()[1..].into();
                Ok(Mark { mark_name })
            }
            Rule::branch => {
                Ok(Branch {
                    face_nodes: pair.into_inner()
                        .map(|face_node| Self::parse_build_node(face_node).unwrap())
                        .collect()
                })
            }
            _ => unreachable!("node"),
        }
    }

    fn parse_scale(scale_pair: Option<Pair<Rule>>) -> f32 {
        match scale_pair {
            None => 1.0,
            Some(scale_pair) => {
                let scale_string = scale_pair.into_inner().next().unwrap().as_str();
                scale_string.parse().unwrap()
            }
        }
    }

    pub fn is_growing(&self) -> bool {
        !self.buds.is_empty()
    }

    pub fn growth_step(&mut self, fabric: &mut Fabric) {
        let buds = self.buds.clone();
        self.buds.clear();
        for bud in buds {
            let (new_buds, new_marks) = self.execute_bud(fabric, bud);
            self.buds.extend(new_buds);
            self.marks.extend(new_marks);
        }
    }

    fn execute_bud(&self, fabric: &mut Fabric, Bud { face_id, forward, scale_factor, node }: Bud) -> (Vec<Bud>, Vec<FaceMark>) {
        let (mut buds, mut marks) = (vec![], vec![]);
        let face = fabric.face(face_id);
        let spin = if forward.starts_with('X') { face.spin.opposite() } else { face.spin };
        if !forward.is_empty() {
            let faces = fabric.single_twist(spin, self.pretenst_factor(), scale_factor, Some(face_id));
            buds.push(Bud {
                face_id: Self::find_face_id(Apos, faces.to_vec()),
                forward: forward[1..].into(),
                scale_factor,
                node,
            });
        } else if let Some(node) = node {
            let (node_buds, node_marks) =
                self.execute_node(fabric, IdentifiedFace { face_id }, &node, vec![]);
            buds.extend(node_buds);
            marks.extend(node_marks);
        };
        (buds, marks)
    }

    pub fn init(&mut self, fabric: &mut Fabric) {
        let BuildPhase { seed, root, .. } = &self;
        match root {
            None => {
                self.twist(fabric, seed.needs_double(), seed.spin(), None);
            }
            Some(node) => {
                let (buds, marks) =
                    self.execute_node(fabric, Seeded { seed: seed.clone() }, node, vec![]);
                self.buds = buds;
                self.marks = marks;
            }
        }
    }

    fn execute_node(&self, fabric: &mut Fabric, launch: Launch, node: &BuildNode, faces: Vec<(FaceName, UniqueId)>) -> (Vec<Bud>, Vec<FaceMark>) {
        let mut buds: Vec<Bud> = vec![];
        let mut marks: Vec<FaceMark> = vec![];
        match node {
            Face { face_name, node } => {
                return self.execute_node(fabric, NamedFace { face_name: *face_name }, node, faces);
            }
            Grow { forward, scale_factor, post_growth_node, .. } => {
                let face_id = match launch {
                    Seeded { seed } => {
                        let faces = fabric.single_twist(seed.spin(), self.pretenst_factor(), *scale_factor, None);
                        return self.execute_node(fabric, NamedFace { face_name: Apos }, node, faces.to_vec());
                    }
                    NamedFace { face_name } => Self::find_face_id(face_name, faces),
                    IdentifiedFace { face_id } => face_id,
                };
                let node = post_growth_node.clone().map(|node_box| *node_box);
                buds.push(Bud { face_id, forward: forward.clone(), scale_factor: *scale_factor, node })
            }
            Branch { face_nodes } => {
                let pairs = Self::branch_pairs(face_nodes);
                let any_special_face = pairs.iter().any(|(face_name, _)| *face_name != Apos);
                let (spin, face_id, needs_double) = match launch {
                    Seeded { seed } => (seed.spin(), None, seed.needs_double()),
                    NamedFace { face_name } => {
                        let face_id = Self::find_face_id(face_name, faces);
                        let spin = fabric.face(face_id).spin.opposite();
                        (spin, Some(face_id), any_special_face)
                    }
                    IdentifiedFace { face_id } => {
                        let spin = fabric.face(face_id).spin.opposite();
                        (spin, Some(face_id), any_special_face)
                    }
                };
                let twist_faces = self.twist(fabric, needs_double, spin, face_id);
                for (face_name, node) in pairs {
                    let (new_buds, new_marks) =
                        self.execute_node(fabric, NamedFace { face_name }, node, twist_faces.clone());
                    buds.extend(new_buds);
                    marks.extend(new_marks);
                }
            }
            Mark { mark_name } => {
                let face_id = match launch {
                    NamedFace { face_name } => Self::find_face_id(face_name, faces),
                    IdentifiedFace { face_id } => face_id,
                    Seeded { .. } => panic!("Need launch face"),
                };
                marks.push(FaceMark { face_id, mark_name: mark_name.clone() });
            }
        }
        (buds, marks)
    }

    fn twist(&self, fabric: &mut Fabric, needs_double: bool, spin: Spin, face_id: Option<UniqueId>) -> Vec<(FaceName, UniqueId)> {
        let faces =
            if needs_double {
                fabric.double_twist(spin, self.pretenst_factor(), 1.0, face_id).to_vec()
            } else {
                fabric.single_twist(spin, self.pretenst_factor(), 1.0, face_id).to_vec()
            };
        let Seed { down_faces, .. } = &self.seed;
        if face_id.is_none() && !down_faces.is_empty() {
            Self::orient_fabric(fabric, &faces, down_faces);
        }
        faces
    }

    fn branch_pairs(nodes: &[BuildNode]) -> Vec<(FaceName, &BuildNode)> {
        nodes
            .iter()
            .map(|face_node| {
                let Face { face_name, node } = face_node else {
                    panic!("Branch may only contain Face nodes");
                };
                (*face_name, node.as_ref())
            })
            .collect()
    }

    fn orient_fabric(fabric: &mut Fabric, faces: &[(FaceName, UniqueId)], down_faces: &[FaceName]) {
        let mut new_down: Vector3<f32> = faces
            .iter()
            .filter(|(face_name, _)| down_faces.contains(face_name))
            .map(|(_, face_id)| fabric.face(*face_id).normal(&fabric.joints, fabric))
            .sum();
        new_down = new_down.normalize();
        let midpoint = fabric.midpoint().to_vec();
        let rotation =
            Matrix4::from_translation(midpoint) *
                Matrix4::from(Quaternion::between_vectors(new_down, -Vector3::unit_y())) *
                Matrix4::from_translation(-midpoint);
        fabric.apply_matrix4(rotation);
    }

    fn find_face_id(face_name: FaceName, face_list: Vec<(FaceName, UniqueId)>) -> UniqueId {
        face_list
            .iter()
            .find(|(name, _)| *name == face_name)
            .map(|(_, face_id)| *face_id)
            .unwrap()
    }

    fn pretenst_factor(&self) -> f32 {
        self.pretenst_factor.unwrap_or(1.3)
    }
}