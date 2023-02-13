use std::convert::Into;

use pest::iterators::Pair;

use crate::build::tenscript::{FaceAlias, FaceMark, Spin};
use crate::build::tenscript::build_phase::BuildNode::{*};
use crate::build::tenscript::build_phase::Launch::{*};
use crate::build::tenscript::Rule;
use crate::fabric::{Fabric, UniqueId};
use crate::fabric::face::FaceRotation;

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
        alias: FaceAlias,
        rotation: usize,
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
            Branch { face_nodes, .. } => {
                for node in face_nodes {
                    node.traverse(f);
                }
            }
        };
    }
}

#[derive(Debug)]
enum Launch {
    Scratch,
    NamedFace { face_alias: FaceAlias },
    IdentifiedFace { face_id: UniqueId },
}

#[derive(Debug, Clone)]
pub struct BaseAliases {
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
            left_bot: FaceAlias::single("Left::Bot"),
            left_top: FaceAlias::single("Left::Top"),
            right_bot: FaceAlias::single("Right::Bot"),
            right_top: FaceAlias::single("Right::Top"),
            omni_left_bot: FaceAlias::single("Omni::Left::Bot"),
            omni_right_bot: FaceAlias::single("Omni::Right::Bot"),
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

}

#[derive(Debug, Clone)]
pub struct BuildPhase {
    pub root: BuildNode,
    pub buds: Vec<Bud>,
    pub marks: Vec<FaceMark>,
    pub base_aliases: BaseAliases,
}

impl BuildPhase {
    pub fn new(root: BuildNode) -> Self {
        Self {
            root,
            buds: Vec::new(),
            marks: Vec::new(),
            base_aliases: BaseAliases::default(),
        }
    }
}

impl BuildPhase {
    pub fn from_pair(pair: Pair<Rule>) -> BuildPhase {
        let root = pair.into_inner().next().map(Self::parse_build_node).unwrap();
        BuildPhase::new(root)
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
                let mut inner = pair.into_inner();
                let alias = FaceAlias::from_pair(inner.next().unwrap());
                let mut face_nodes = Vec::new();
                let mut rotation = 0;
                for node_pair in inner {
                    match node_pair.as_rule() {
                        Rule::face_rotation => {
                            rotation += 1;
                        }
                        Rule::on_face => {
                            face_nodes.push(Self::parse_build_node(node_pair));
                        }
                        _ => unreachable!("{:?}", node_pair)
                    }
                }
                Branch { alias, rotation, face_nodes }
            }
            _ => unreachable!("node {:?}", pair.as_rule()),
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
        let (buds, marks) =
            self.execute_node(fabric, Scratch, &self.root, vec![]);
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
            let faces = fabric.attach_brick(bot_alias, FaceRotation::Zero, scale_factor, Some(face_id));
            buds.push(Bud {
                face_id: Self::find_face_id(top_alias, &faces, fabric),
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

    fn execute_node(&self, fabric: &mut Fabric, launch: Launch, node: &BuildNode, faces: Vec<UniqueId>) -> (Vec<Bud>, Vec<FaceMark>) {
        let mut buds: Vec<Bud> = vec![];
        let mut marks: Vec<FaceMark> = vec![];
        match node {
            Face { alias, node } => {
                let build_node = node.as_ref();
                return self.execute_node(fabric, NamedFace { face_alias: alias.clone() }, build_node, faces);
            }
            Grow { forward, scale_factor, post_growth_node, .. } => {
                let face_id = Self::find_launch_face(launch, &faces, fabric).unwrap();
                let node = post_growth_node.clone().map(|x| *x);
                buds.push(Bud { face_id, forward: forward.clone(), scale_factor: *scale_factor, node })
            }
            Branch { face_nodes, rotation, alias } => {
                let attach_to = Self::find_launch_face(launch, &faces, fabric);
                let brick_faces = fabric.attach_brick(alias, rotation.into(), 1.0, attach_to);
                for (face_alias, node) in Self::branch_pairs(face_nodes) {
                    let (new_buds, new_marks) =
                        self.execute_node(fabric, NamedFace { face_alias }, node, brick_faces.clone());
                    buds.extend(new_buds);
                    marks.extend(new_marks);
                }
            }
            Mark { mark_name } => {
                let face_id = Self::find_launch_face(launch, &faces, fabric).expect("cannot mark from scratch");
                marks.push(FaceMark { face_id, mark_name: mark_name.clone() });
            }
        };
        (buds, marks)
    }

    fn find_launch_face(launch: Launch, faces: &[UniqueId], fabric: &Fabric) -> Option<UniqueId> {
        match launch {
            Scratch =>
                None,
            NamedFace { face_alias } =>
                Some(Self::find_face_id(&face_alias, &faces, fabric)),
            IdentifiedFace { face_id } =>
                Some(face_id),
        }
    }

    fn branch_pairs(nodes: &[BuildNode]) -> Vec<(FaceAlias, &BuildNode)> {
        nodes
            .iter()
            .map(|face_node| {
                let Face { alias, node } = face_node else {
                    unreachable!("Branch can only contain Face nodes");
                };
                (alias.clone(), node.as_ref())
            })
            .collect()
    }

    fn find_face_id(alias: &FaceAlias, face_list: &[UniqueId], fabric: &Fabric) -> UniqueId {
        face_list
            .iter()
            .find_map(|&face_id| fabric.face(face_id).has_alias(alias).then_some(face_id))
            .unwrap_or_else(|| panic!("no such face: {alias}"))
    }
}