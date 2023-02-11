use std::convert::Into;
use std::string::ToString;

use cgmath::{EuclideanSpace, InnerSpace, Matrix4, Quaternion, Rotation, Vector3};
use pest::iterators::Pair;

use crate::build::tenscript::{FaceAlias, FaceMark, Spin};
use crate::build::tenscript::build_phase::BuildNode::{*};
use crate::build::tenscript::build_phase::Launch::{*};
use crate::build::tenscript::Rule;
use crate::fabric::{Fabric, UniqueId};

#[derive(Debug, Default, Clone)]
pub struct Bud {
    face_id: UniqueId,
    forward: String,
    scale_factor: f32,
    node: Option<BuildNode>,
}

#[derive(Debug, Clone)]
pub enum BuildNode {
    Face {
        alias: FaceAlias,
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

impl BuildNode {
    pub fn traverse(&self, f: &mut impl FnMut(&Self)) {
        f(self);
        match self {
            Mark { .. } => {}
            Face { node, .. } => {
                node.traverse(f);
            }
            Grow { post_growth_node, .. } => {
                let Some(node) = post_growth_node else {
                    return;
                };
                node.traverse(f);
            }
            Branch { face_nodes } => {
                for node in face_nodes {
                    node.traverse(f);
                }
            }
        };
    }
}

#[derive(Debug)]
enum Launch {
    Scratch { face_alias: FaceAlias },
    NamedFace { face_alias: FaceAlias },
    IdentifiedFace { face_id: UniqueId },
}

#[derive(Debug, Clone)]
struct BaseAliases {
    left_bot: FaceAlias,
    left_top: FaceAlias,
    right_bot: FaceAlias,
    right_top: FaceAlias,
    omni_left_bot: FaceAlias,
    omni_right_bot: FaceAlias,
}

impl Default for BaseAliases {
    fn default() -> Self {
        Self {
            left_bot: FaceAlias("Left::Bot".to_string()),
            left_top: FaceAlias("Left::Top".to_string()),
            right_bot: FaceAlias("Right::Bot".to_string()),
            right_top: FaceAlias("Right::Top".to_string()),
            omni_left_bot: FaceAlias("Omni::Left::Top".to_string()),
            omni_right_bot: FaceAlias("Omni::Right::Top".to_string()),
        }
    }
}

impl BaseAliases {
    pub fn spin_based(&self, spin: Spin) -> (&FaceAlias, &FaceAlias) {
        match spin {
            Spin::Left => (&self.left_bot, &self.left_top),
            Spin::Right => (&self.right_bot, &self.right_top),
        }
    }

    pub fn spin_double_based(&self, spin: Spin, needs_double: bool) -> &FaceAlias {
        match spin {
            Spin::Left if needs_double => &self.omni_left_bot,
            Spin::Right if needs_double => &self.omni_right_bot,
            Spin::Left => &self.left_bot,
            Spin::Right => &self.right_bot,
        }
    }

    pub fn not_single_top(&self, alias: &FaceAlias) -> bool {
        !(&self.right_top == alias || &self.left_top == alias)
    }
}

#[derive(Debug, Clone, Default)]
pub struct BuildPhase {
    pub face_alias: FaceAlias,
    pub root: Option<BuildNode>,
    pub buds: Vec<Bud>,
    pub marks: Vec<FaceMark>,
    pub base_aliases: BaseAliases,
}

impl BuildPhase {
    pub fn new(face_alias: FaceAlias, root: Option<BuildNode>) -> Self {
        Self {
            face_alias,
            root,
            buds: vec![],
            marks: vec![],
            base_aliases: BaseAliases::default(),
        }
    }
}

impl BuildPhase {
    pub fn from_pair(pair: Pair<Rule>) -> BuildPhase {
        let mut face_alias = None;
        let mut build_node = None;
        for sub_pair in pair.into_inner() {
            match sub_pair.as_rule() {
                Rule::face_alias => {
                    face_alias = Some(FaceAlias::from_pair(sub_pair));
                }
                Rule::build_node => {
                    build_node = Some(Self::parse_build_node(sub_pair));
                }
                _ => unreachable!("build phase"),
            }
        }
        BuildPhase::new(face_alias.expect("build must have face alias"), build_node)
    }

    fn parse_build_node(pair: Pair<Rule>) -> BuildNode {
        match pair.as_rule() {
            Rule::build_node =>
                Self::parse_build_node(pair.into_inner().next().unwrap()),
            Rule::on_face => {
                let [face_name_pair, node_pair] = pair.into_inner().next_chunk().unwrap();
                let alias = FaceAlias::from_pair(face_name_pair);
                let node = Self::parse_build_node(node_pair);
                Face {
                    alias,
                    node: Box::new(node),
                }
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
                    .map(|post_growth| Box::new(Self::parse_build_node(post_growth)));
                Grow {
                    forward,
                    scale_factor,
                    post_growth_node,
                }
            }
            Rule::mark => {
                let mark_name = pair.into_inner().next().unwrap().as_str()[1..].into();
                Mark { mark_name }
            }
            Rule::branch => {
                Branch {
                    face_nodes: pair.into_inner()
                        .map(Self::parse_build_node)
                        .collect()
                }
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

    pub fn init(&mut self, fabric: &mut Fabric) {
        let face_alias = self.face_alias.clone();
        let (buds, marks) =
            self.execute_node(fabric, Scratch { face_alias }, self.root.as_ref(), vec![]);
        self.buds = buds;
        self.marks = marks;
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
            let (bot_alias, top_alias) = self.base_aliases.spin_based(spin);
            let faces = fabric.attach_brick(bot_alias, scale_factor, Some(face_id));
            buds.push(Bud {
                face_id: Self::find_face_id(top_alias, &faces, fabric),
                forward: forward[1..].into(),
                scale_factor,
                node,
            });
        } else if let Some(node) = node {
            let (node_buds, node_marks) =
                self.execute_node(fabric, IdentifiedFace { face_id }, Some(&node), vec![]);
            buds.extend(node_buds);
            marks.extend(node_marks);
        };
        (buds, marks)
    }

    fn execute_node(&self, fabric: &mut Fabric, launch: Launch, node_option: Option<&BuildNode>, faces: Vec<UniqueId>) -> (Vec<Bud>, Vec<FaceMark>) {
        let mut buds: Vec<Bud> = vec![];
        let mut marks: Vec<FaceMark> = vec![];
        if let Scratch { face_alias } = launch {
            let faces = fabric.attach_brick(&face_alias, 1.0, None);
            return self.execute_node(fabric, NamedFace { face_alias }, node_option, faces);
        }
        match node_option.unwrap() {
            Face { alias, node } => {
                let build_node = node.as_ref();
                return self.execute_node(fabric, NamedFace { face_alias: alias.clone() }, Some(build_node), faces);
            }
            Grow { forward, scale_factor, post_growth_node, .. } => {
                let face_id = match launch {
                    Scratch { .. } => unreachable!("cannot grow from scratch"),
                    NamedFace { face_alias } => Self::find_face_id(&face_alias, &faces, fabric),
                    IdentifiedFace { face_id } => face_id,
                };
                let node = post_growth_node.clone().map(|x|*x);
                buds.push(Bud { face_id, forward: forward.clone(), scale_factor: *scale_factor, node })
            }
            Branch { face_nodes } => {
                let pairs = Self::branch_pairs(&face_nodes);
                let needs_double = pairs
                    .iter()
                    .any(|(face_alias, _)| self.base_aliases.not_single_top(face_alias));
                let face_name = |spin: Spin| self.base_aliases.spin_double_based(spin, needs_double);
                let (face_alias, face_id) = match launch {
                    Scratch { .. } => unreachable!("cannot branch from scratch"),
                    NamedFace { face_alias } => {
                        let face_id = Self::find_face_id(&face_alias, &faces, fabric);
                        let spin = fabric.face(face_id).spin.opposite();
                        (face_name(spin), Some(face_id))
                    }
                    IdentifiedFace { face_id } => {
                        let spin = fabric.face(face_id).spin.opposite();
                        (face_name(spin), Some(face_id))
                    }
                };
                let twist_faces = fabric.attach_brick(&face_alias, 1.0, face_id);
                for (face_name, node) in pairs {
                    let (new_buds, new_marks) =
                        self.execute_node(fabric, NamedFace { face_alias: face_name }, Some(&node), twist_faces.clone());
                    buds.extend(new_buds);
                    marks.extend(new_marks);
                }
            }
            Mark { mark_name } => {
                let face_id = match launch {
                    Scratch { .. } => unreachable!("cannot mark from scratch"),
                    NamedFace { face_alias } => Self::find_face_id(&face_alias, &faces, fabric),
                    IdentifiedFace { face_id } => face_id,
                };
                marks.push(FaceMark { face_id, mark_name: mark_name.clone() });
            }
        }
        (buds, marks)
    }

    fn branch_pairs(nodes: &[BuildNode]) -> Vec<(FaceAlias, &BuildNode)> {
        nodes
            .iter()
            .map(|face_node| {
                let Face { alias: face_name, node } = face_node else {
                    unreachable!("Branch can only contain Face nodes");
                };
                (face_name.clone(), node.as_ref())
            })
            .collect()
    }

    fn orient_fabric(fabric: &mut Fabric, faces: &[(FaceAlias, UniqueId)], down_faces: &[FaceAlias]) {
        let mut new_down: Vector3<f32> = faces
            .iter()
            .filter(|(face_name, _)| down_faces.contains(face_name))
            .map(|(_, face_id)| fabric.face(*face_id).normal(fabric))
            .sum();
        new_down = new_down.normalize();
        let midpoint = fabric.midpoint().to_vec();
        let rotation =
            Matrix4::from_translation(midpoint) *
                Matrix4::from(Quaternion::between_vectors(new_down, -Vector3::unit_y())) *
                Matrix4::from_translation(-midpoint);
        fabric.apply_matrix4(rotation);
    }

    fn find_face_id(alias: &FaceAlias, face_list: &[UniqueId], fabric: &Fabric) -> UniqueId {
        face_list
            .iter()
            .find_map(|&face_id| fabric.face(face_id).has_alias(&alias).then_some(face_id))
            .expect("no such face")
    }
}