use std::convert::Into;

use pest::iterators::Pair;

use crate::build::tenscript::brick_library::BrickLibrary;
use crate::build::tenscript::build_phase::BuildNode::*;
use crate::build::tenscript::build_phase::Launch::*;
use crate::build::tenscript::Rule;
use crate::build::tenscript::{FaceAlias, FaceMark, TenscriptError};
use crate::fabric::face::FaceRotation;
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
        alias: FaceAlias,
        rotation: usize,
        scale_factor: f32,
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
            Grow {
                post_growth_node, ..
            } => {
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
pub struct BuildPhase {
    pub root: BuildNode,
    pub buds: Vec<Bud>,
    pub marks: Vec<FaceMark>,
}

impl BuildPhase {
    pub fn new(root: BuildNode) -> Self {
        Self {
            root,
            buds: Vec::new(),
            marks: Vec::new(),
        }
    }
}

impl BuildPhase {
    pub fn from_pair(pair: Pair<Rule>) -> Result<BuildPhase, TenscriptError> {
        pair.into_inner()
            .next()
            .map(|build_node_pair| Self::parse_build_node(build_node_pair).map(BuildPhase::new))
            .unwrap()
    }

    fn parse_build_node(pair: Pair<Rule>) -> Result<BuildNode, TenscriptError> {
        match pair.as_rule() {
            Rule::build_node => Self::parse_build_node(pair.into_inner().next().unwrap()),
            Rule::on_face => {
                let mut inner = pair.into_inner();
                let [face_name_pair, node_pair] = [inner.next().unwrap(), inner.next().unwrap()];
                let alias = FaceAlias::from_pair(face_name_pair);
                let node = Self::parse_build_node(node_pair)?;
                Ok(Face {
                    alias,
                    node: Box::new(node),
                })
            }
            Rule::grow => {
                let mut inner = pair.into_inner();
                let forward_string = inner.next().unwrap().as_str();
                let forward = match forward_string.parse() {
                    Ok(count) => "X".repeat(count),
                    Err(_) => forward_string[1..forward_string.len() - 1].into(),
                };
                let mut scale = None;
                let mut post_growth_node = None;
                for inner_pair in inner {
                    match inner_pair.as_rule() {
                        Rule::scale => {
                            let parsed_scale =
                                TenscriptError::parse_float_inside(inner_pair, "grow/scale")?;
                            scale = Some(parsed_scale);
                        }
                        Rule::build_node => {
                            let parsed_node = Self::parse_build_node(inner_pair)?;
                            post_growth_node = Some(Box::new(parsed_node))
                        }
                        _ => unreachable!(),
                    }
                }
                let scale_factor = scale.unwrap_or(1.0);
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
                let mut inner = pair.into_inner();
                let alias = FaceAlias::from_pair(inner.next().unwrap());
                let mut scale = None;
                let mut face_nodes = Vec::new();
                let mut rotation = 0;
                for node_pair in inner {
                    match node_pair.as_rule() {
                        Rule::face_rotation => {
                            rotation += 1;
                        }
                        Rule::scale => {
                            let parsed_scale =
                                TenscriptError::parse_float_inside(node_pair, "branch/scale")?;
                            scale = Some(parsed_scale);
                        }
                        Rule::on_face => {
                            let node = Self::parse_build_node(node_pair)?;
                            face_nodes.push(node);
                        }
                        _ => unreachable!("{:?}", node_pair),
                    }
                }
                let scale_factor = scale.unwrap_or(1.0);
                Ok(Branch {
                    alias,
                    rotation,
                    face_nodes,
                    scale_factor,
                })
            }
            _ => unreachable!("node {:?}", pair.as_rule()),
        }
    }

    fn _parse_scale(scale_pair: Option<Pair<Rule>>) -> f32 {
        match scale_pair {
            None => 1.0,
            Some(scale_pair) => {
                let scale_string = scale_pair.into_inner().next().unwrap().as_str();
                scale_string.parse().unwrap()
            }
        }
    }

    pub fn init(
        &mut self,
        fabric: &mut Fabric,
        brick_library: &BrickLibrary,
    ) -> Result<(), TenscriptError> {
        let (buds, marks) = Self::execute_node(fabric, Scratch, &self.root, vec![], brick_library)?;
        self.buds = buds;
        self.marks = marks;
        Ok(())
    }

    pub fn is_growing(&self) -> bool {
        !self.buds.is_empty()
    }

    pub fn growth_step(
        &mut self,
        fabric: &mut Fabric,
        brick_library: &BrickLibrary,
    ) -> Result<(), TenscriptError> {
        let buds = self.buds.clone();
        self.buds.clear();
        for bud in buds {
            let (new_buds, new_marks) = self.execute_bud(fabric, bud, brick_library)?;
            self.buds.extend(new_buds);
            self.marks.extend(new_marks);
        }
        Ok(())
    }

    fn execute_bud(
        &self,
        fabric: &mut Fabric,
        Bud {
            face_id,
            forward,
            scale_factor,
            node,
        }: Bud,
        brick_library: &BrickLibrary,
    ) -> Result<(Vec<Bud>, Vec<FaceMark>), TenscriptError> {
        let (mut buds, mut marks) = (vec![], vec![]);
        let face = fabric.expect_face(face_id)?;
        let spin = if forward.starts_with('X') {
            face.spin.opposite()
        } else {
            face.spin
        };
        if !forward.is_empty() {
            let face_alias = FaceAlias::single("Single") + &spin.into_alias();
            let (base_face, faces) = fabric.create_brick(
                &face_alias,
                FaceRotation::Zero,
                scale_factor,
                Some(face_id),
                brick_library,
            );
            fabric.join_faces(base_face, face_id);
            let top_face_alias = face_alias + &FaceAlias::single(":next-base");
            buds.push(Bud {
                face_id: top_face_alias
                    .find_face_in(&faces, fabric)
                    .expect("face matching top face alias"),
                forward: forward[1..].into(),
                scale_factor,
                node,
            });
        } else if let Some(node) = node {
            let (node_buds, node_marks) = Self::execute_node(
                fabric,
                IdentifiedFace { face_id },
                &node,
                vec![],
                brick_library,
            )?;
            buds.extend(node_buds);
            marks.extend(node_marks);
        };
        Ok((buds, marks))
    }

    fn execute_node(
        fabric: &mut Fabric,
        launch: Launch,
        node: &BuildNode,
        faces: Vec<UniqueId>,
        brick_library: &BrickLibrary,
    ) -> Result<(Vec<Bud>, Vec<FaceMark>), TenscriptError> {
        let mut buds: Vec<Bud> = vec![];
        let mut marks: Vec<FaceMark> = vec![];
        match node {
            Face { alias, node } => {
                let build_node = node.as_ref();
                return Self::execute_node(
                    fabric,
                    NamedFace {
                        face_alias: alias.clone(),
                    },
                    build_node,
                    faces,
                    brick_library,
                );
            }
            Grow {
                forward,
                scale_factor,
                post_growth_node,
                ..
            } => {
                let face_id = Self::find_launch_face(&launch, &faces, fabric)?;
                let face_id = face_id.ok_or(TenscriptError::FaceAlias("grow".to_string()))?;
                let node = post_growth_node.clone().map(|x| *x);
                buds.push(Bud {
                    face_id,
                    forward: forward.clone(),
                    scale_factor: *scale_factor,
                    node,
                })
            }
            Branch {
                face_nodes,
                rotation,
                alias,
                scale_factor,
            } => {
                let launch_face = Self::find_launch_face(&launch, &faces, fabric)?;
                let (base_face_id, brick_faces) = fabric.create_brick(
                    alias,
                    rotation.into(),
                    *scale_factor,
                    launch_face,
                    brick_library,
                );
                if let Some(face_id) = launch_face {
                    fabric.join_faces(base_face_id, face_id)
                }
                for (branch_face_alias, branch_node) in Self::branch_pairs(face_nodes) {
                    let (new_buds, new_marks) = Self::execute_node(
                        fabric,
                        NamedFace {
                            face_alias: branch_face_alias,
                        },
                        branch_node,
                        brick_faces.clone(),
                        brick_library,
                    )?;
                    buds.extend(new_buds);
                    marks.extend(new_marks);
                }
            }
            Mark { mark_name } => {
                let maybe_face_id = Self::find_launch_face(&launch, &faces, fabric)?;
                let face_id = maybe_face_id.ok_or(TenscriptError::Mark(mark_name.clone()))?;
                marks.push(FaceMark {
                    face_id,
                    mark_name: mark_name.clone(),
                });
            }
        };
        Ok((buds, marks))
    }

    fn find_launch_face(
        launch: &Launch,
        faces: &[UniqueId],
        fabric: &Fabric,
    ) -> Result<Option<UniqueId>, TenscriptError> {
        match launch {
            Scratch => Ok(None),
            NamedFace { face_alias } => match face_alias.find_face_in(faces, fabric) {
                None => Err(TenscriptError::FaceAlias(face_alias.to_string())),
                Some(face_alias) => Ok(Some(face_alias)),
            },
            IdentifiedFace { face_id } => Ok(Some(*face_id)),
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
}
