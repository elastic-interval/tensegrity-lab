use cgmath::{EuclideanSpace, InnerSpace, Matrix4, Quaternion, Rotation, Vector3};
use pest::iterators::Pair;

use crate::build::tenscript::{FaceAlias, FaceMark, parse_atom, Spin};
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
        face_name: FaceAlias,
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
    Seeded { face_alias: FaceAlias },
    NamedFace { face_alias: FaceAlias },
    IdentifiedFace { face_id: UniqueId },
}

#[derive(Debug, Default, Clone)]
pub struct BuildPhase {
    pub root: Option<BuildNode>,
    pub buds: Vec<Bud>,
    pub marks: Vec<FaceMark>,
}

impl BuildPhase {
    pub fn from_pair(pair: Pair<Rule>) -> BuildPhase {
        let mut phase = BuildPhase::default();
        for sub_pair in pair.into_inner() {
            match sub_pair.as_rule() {
                Rule::build_node => {
                    phase.root = Some(Self::parse_build_node(sub_pair));
                }
                _ => unreachable!("build phase"),
            }
        }
        phase
    }

    fn parse_build_node(pair: Pair<Rule>) -> BuildNode {
        match pair.as_rule() {
            Rule::build_node =>
                Self::parse_build_node(pair.into_inner().next().unwrap()),
            Rule::on_face => {
                let [face_name_pair, node_pair] = pair.into_inner().next_chunk().unwrap();
                let face_name = FaceAlias { name: parse_atom(face_name_pair) };
                let node = Self::parse_build_node(node_pair);
                Face {
                    face_name,
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

    pub fn is_growing(&self) -> bool {
        !self.buds.is_empty()
    }

    pub fn init(&mut self, fabric: &mut Fabric) {
        let node = self.root.expect("build phase has no root node");
        let face_alias = match &node {
            Face { .. } => {}
            Grow { .. } => {}
            Branch { .. } => {}
            Mark { .. } => unreachable!(),
        };
        let (buds, marks) =
            self.execute_node(fabric, Seeded {}, node, vec![]);
        self.buds = buds;
        self.marks = marks;
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
            // TODO: don't hardcode these names, look them up somewhere

            let (bot_alias, top_alias) = match spin {
                Spin::Left => (FaceAlias::new("Left::Bot".to_string()), FaceAlias::new("Left::Top".to_string())),
                Spin::Right => (FaceAlias::new("Right::Bot".to_string()), FaceAlias::new("Right::Top".to_string())),
            };
            let faces = fabric.attach_brick(&bot_alias, scale_factor, Some(face_id));
            buds.push(Bud {
                face_id: Self::find_face_id(top_alias, faces),
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
            Face { face_name, node } => {
                return self.execute_node(fabric, NamedFace { face_alias: face_name.clone() }, node, faces);
            }
            Grow { forward, scale_factor, post_growth_node, .. } => {
                let face_id = match launch {
                    Seeded { face_alias } => {
                        let faces = fabric.attach_brick(&face_alias, *scale_factor, None);
                        // TODO: extract the right next face alias from the newly attached brick
                        return self.execute_node(fabric, NamedFace { face_alias }, node, faces.to_vec());
                    }
                    NamedFace { face_alias } => Self::find_face_id(&face_alias, &faces, fabric),
                    IdentifiedFace { face_id } => face_id,
                };
                let node = post_growth_node.clone().map(|node_box| *node_box);
                buds.push(Bud { face_id, forward: forward.clone(), scale_factor: *scale_factor, node })
            }
            Branch { face_nodes } => {
                let pairs = Self::branch_pairs(face_nodes);
                let needs_double = pairs.iter().any(|(FaceAlias { name: name }, _)| !["Top", "Bot"].contains(&name.as_str()));
                let brick_name = |spin: Spin| BrickName(match spin {
                    Spin::Left if needs_double => "omni-left",
                    Spin::Right if needs_double => "omni-right",
                    Spin::Left => "single-left",
                    Spin::Right => "single-right",
                }.to_string());
                let (face_alias, face_id) = match launch {
                    Seeded { face_alias } => (face_alias, None),
                    NamedFace { face_alias } => {
                        let face_id = Self::find_face_id(&face_alias, &faces, fabric);
                        let spin = fabric.face(face_id).spin.opposite();
                        (brick_name(spin), Some(face_id))
                    }
                    IdentifiedFace { face_id } => {
                        let spin = fabric.face(face_id).spin.opposite();
                        (brick_name(spin), Some(face_id))
                    }
                };
                let twist_faces = fabric.attach_brick(&face_alias, 1.0, face_id);
                for (face_name, node) in pairs {
                    let (new_buds, new_marks) =
                        self.execute_node(fabric, NamedFace { face_alias: face_name }, node, twist_faces.clone());
                    buds.extend(new_buds);
                    marks.extend(new_marks);
                }
            }
            Mark { mark_name } => {
                let face_id = match launch {
                    NamedFace { face_alias } => Self::find_face_id(&face_alias, &faces, fabric),
                    IdentifiedFace { face_id } => face_id,
                    Seeded { .. } => unreachable!("Need launch face"),
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
                let Face { face_name, node } = face_node else {
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
            .find_map(|&face_id| fabric.face(face_id).has_alias(&alias.name).then_some(face_id))
            .expect("no such face")
    }
}