use std::convert::Into;

use crate::build::dsl::brick_dsl::{BrickName, BrickRole, MarkName};
use crate::build::dsl::brick_library::BrickLibrary;
use crate::build::dsl::build_phase::BuildNode::*;
use crate::build::dsl::build_phase::Launch::*;
use crate::build::dsl::{FaceAlias, FaceMark, FaceTag, Spin};
use crate::fabric::brick::BaseFace;
use crate::fabric::face::FaceRotation;
use crate::fabric::{Fabric, UniqueId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chirality {
    Chiral,
    Alternating,
}

#[derive(Debug, Clone, Copy)]
pub struct GrowStyle {
    pub count: usize,
    pub chirality: Chirality,
}

impl GrowStyle {
    pub fn new(count: usize, chirality: Chirality) -> Self {
        Self { count, chirality }
    }

    pub fn alternating(count: usize) -> Self {
        Self {
            count,
            chirality: Chirality::Alternating,
        }
    }

    pub fn chiral(count: usize) -> Self {
        Self {
            count,
            chirality: Chirality::Chiral,
        }
    }

    pub fn is_alternating(&self) -> bool {
        self.chirality == Chirality::Alternating
    }

    pub fn decrement(&self) -> Option<GrowStyle> {
        if self.count > 1 {
            Some(GrowStyle {
                count: self.count - 1,
                chirality: self.chirality,
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Bud {
    face_id: UniqueId,
    grow_style: Option<GrowStyle>,
    scale_factor: f32,
    nodes: Vec<BuildNode>,
}

#[derive(Debug, Clone)]
pub enum BuildNode {
    Face {
        alias: FaceAlias,
        node: Box<BuildNode>,
    },
    Grow {
        style: GrowStyle,
        scale_factor: f32,
        post_growth_nodes: Vec<BuildNode>,
    },
    Mark {
        mark_name: MarkName,
    },
    Branch {
        brick_name: BrickName,
        brick_role: BrickRole,
        rotation: usize,
        scale_factor: f32,
        face_nodes: Vec<BuildNode>,
    },
    Prism,
}

impl BuildNode {
    pub fn traverse(&self, f: &mut impl FnMut(&Self)) {
        f(self);
        match self {
            Mark { .. }| Prism{..} => {}
            Face { node, .. } => {
                node.traverse(f);
            }
            Grow {
                post_growth_nodes, ..
            } => {
                for node in post_growth_nodes {
                    node.traverse(f);
                }
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
    NamedFace(FaceAlias),
    IdentifiedFace(UniqueId),
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
    pub fn init(
        &mut self,
        fabric: &mut Fabric,
        brick_library: &BrickLibrary,
    ) {
        let (buds, marks) = Self::execute_node(fabric, Scratch, &self.root, vec![], brick_library);
        self.buds = buds;
        self.marks = marks;
    }

    pub fn is_growing(&self) -> bool {
        !self.buds.is_empty()
    }

    pub fn growth_step(
        &mut self,
        fabric: &mut Fabric,
        brick_library: &BrickLibrary,
    ) {
        let buds = self.buds.clone();
        self.buds.clear();
        for bud in buds {
            let (new_buds, new_marks) = self.execute_bud(fabric, bud, brick_library);
            self.buds.extend(new_buds);
            self.marks.extend(new_marks);
        }
    }

    fn execute_bud(
        &self,
        fabric: &mut Fabric,
        Bud {
            face_id,
            grow_style,
            scale_factor,
            nodes,
        }: Bud,
        brick_library: &BrickLibrary,
    ) -> (Vec<Bud>, Vec<FaceMark>) {
        let (mut buds, mut marks) = (vec![], vec![]);

        if let Some(style) = grow_style {
            let face = fabric.expect_face(face_id);
            let spin = if style.is_alternating() {
                face.spin.opposite()
            } else {
                face.spin
            };
            // TODO: Create either a Left or Right single brick, depending on Spin
            let brick_name = match spin {
                Spin::Left => BrickName::SingleLeftBrick,
                Spin::Right => BrickName::SingleRightBrick,
            };
            let (base_face, brick_faces) = fabric.create_brick(
                brick_name,
                FaceRotation::Zero,
                scale_factor,
                BaseFace::ExistingFace(face_id),
                brick_library,
            );
            fabric.join_faces(base_face, face_id);
            let attach_next = FaceAlias::single(FaceTag::AttachNext(spin));
            buds.push(Bud {
                face_id: attach_next
                    .find_face_in(&brick_faces, fabric)
                    .expect("face matching top face alias"),
                grow_style: style.decrement(),
                scale_factor,
                nodes,
            });
        } else if !nodes.is_empty() {
            for child_node in &nodes {
                let (node_buds, node_marks) = Self::execute_node(
                    fabric,
                    IdentifiedFace(face_id),
                    child_node,
                    vec![],
                    brick_library,
                );
                buds.extend(node_buds);
                marks.extend(node_marks);
            }
        };
        (buds, marks)
    }

    fn execute_node(
        fabric: &mut Fabric,
        launch: Launch,
        node: &BuildNode,
        faces: Vec<UniqueId>,
        brick_library: &BrickLibrary,
    ) -> (Vec<Bud>, Vec<FaceMark>) {
        let mut buds: Vec<Bud> = vec![];
        let mut marks: Vec<FaceMark> = vec![];
        match node {
            Face { alias, node } => {
                let build_node = node.as_ref();
                return Self::execute_node(
                    fabric,
                    NamedFace(alias.clone()),
                    build_node,
                    faces,
                    brick_library,
                );
            }
            Grow {
                style,
                scale_factor,
                post_growth_nodes,
                ..
            } => {
                let face_id = Self::find_launch_face(&launch, &faces, fabric);
                let face_id = face_id.expect("Unable to find the launch face by id in execute_node");
                buds.push(Bud {
                    face_id,
                    grow_style: Some(*style),
                    scale_factor: *scale_factor,
                    nodes: post_growth_nodes.clone(),
                })
            }
            Branch {
                brick_name,
                face_nodes,
                rotation,
                seed,
                scale_factor,
            } => {
                let launch_face = Self::find_launch_face(&launch, &faces, fabric);
                let base_face = launch_face
                    .map(BaseFace::ExistingFace)
                    .unwrap_or((*seed).map(BaseFace::Seeded).unwrap_or(BaseFace::Baseless));
                let (base_face_id, brick_faces) = fabric.create_brick(
                    *brick_name,
                    rotation.into(),
                    *scale_factor,
                    base_face,
                    brick_library,
                );
                if let Some(face_id) = launch_face {
                    fabric.join_faces(base_face_id, face_id)
                }
                for (branch_face_alias, branch_node) in Self::branch_pairs(face_nodes) {
                    let (new_buds, new_marks) = Self::execute_node(
                        fabric,
                        NamedFace(branch_face_alias),
                        branch_node,
                        brick_faces.clone(),
                        brick_library,
                    );
                    buds.extend(new_buds);
                    marks.extend(new_marks);
                }
            }
            Mark { mark_name } => {
                let maybe_face_id = Self::find_launch_face(&launch, &faces, fabric);
                let face_id = maybe_face_id.expect(&format!("Unable to find face for mark: {}", mark_name));
                marks.push(FaceMark {
                    face_id,
                    mark_name: mark_name.clone(),
                });
            }
            Prism => {
                let maybe_face_id = Self::find_launch_face(&launch, &faces, fabric);
                let face_id = maybe_face_id.expect("Unable to find face for prism");
                fabric.add_face_prism(face_id);
            }
        };
        (buds, marks)
    }

    fn find_launch_face(
        launch: &Launch,
        faces: &[UniqueId],
        fabric: &Fabric,
    ) -> Option<UniqueId> {
        match launch {
            Scratch => None,
            NamedFace(face_alias) => face_alias.find_face_in(faces, fabric)
                .or_else(|| panic!("Unable to find face alias {:?}", face_alias)),
            IdentifiedFace(face_id) => Some(*face_id),
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
