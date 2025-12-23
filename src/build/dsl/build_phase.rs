use crate::build::dsl::brick_dsl::FaceName::AttachNext;
use crate::build::dsl::brick_dsl::{BrickName, BrickRole, MarkName};
use crate::build::dsl::build_phase::BuildNode::*;
use crate::build::dsl::build_phase::Launch::*;
use crate::build::dsl::{brick_library, FaceAlias, FaceMark, Spin};
use crate::fabric::brick::BaseFace;
use crate::fabric::face::FaceRotation;
use crate::fabric::joint::{JointPath, COLUMN_MARKER};
use crate::fabric::{Fabric, FaceKey};
use crate::units::Percent;
use std::convert::Into;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chirality {
    Chiral,
    Alternating,
}

#[derive(Debug, Clone, Copy)]
pub struct ColumnStyle {
    pub count: usize,
    pub chirality: Chirality,
}

impl ColumnStyle {
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

    pub fn decrement(&self) -> Option<ColumnStyle> {
        if self.count > 1 {
            Some(ColumnStyle {
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
    face_key: FaceKey,
    column_style: Option<ColumnStyle>,
    scale: Percent,
    nodes: Vec<BuildNode>,
    branch_path: JointPath,
}

#[derive(Debug, Clone)]
pub enum BuildNode {
    Face {
        alias: FaceAlias,
        node: Box<BuildNode>,
    },
    Column {
        style: ColumnStyle,
        scale: Percent,
        post_column_nodes: Vec<BuildNode>,
    },
    Mark {
        mark_name: MarkName,
    },
    Hub {
        brick_name: BrickName,
        brick_role: BrickRole,
        rotation: usize,
        scale: Percent,
        face_nodes: Vec<BuildNode>,
    },
    Prism,
    /// Mark face as radial (radials only, no triangle)
    Radial,
}

impl BuildNode {
    pub fn traverse(&self, f: &mut impl FnMut(&Self)) {
        f(self);
        match self {
            Mark { .. } | Prism { .. } | Radial { .. } => {}
            Face { node, .. } => {
                node.traverse(f);
            }
            Column {
                post_column_nodes, ..
            } => {
                for node in post_column_nodes {
                    node.traverse(f);
                }
            }
            Hub { face_nodes, .. } => {
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
    IdentifiedFace(FaceKey),
}

#[derive(Debug, Clone)]
pub struct BuildPhase {
    pub root: BuildNode,
    pub buds: Vec<Bud>,
    pub marks: Vec<FaceMark>,
    pub seed_altitude: f32,
}

impl BuildPhase {
    pub fn new(root: BuildNode, seed_altitude: f32) -> Self {
        Self {
            root,
            buds: Vec::new(),
            marks: Vec::new(),
            seed_altitude,
        }
    }
}

impl BuildPhase {
    pub fn init(&mut self, fabric: &mut Fabric) {
        let (buds, marks) = Self::execute_node(
            fabric,
            Scratch,
            &self.root,
            vec![],
            self.seed_altitude,
            JointPath::default(),
        );
        self.buds = buds;
        self.marks = marks;
    }

    pub fn is_building(&self) -> bool {
        !self.buds.is_empty()
    }

    pub fn build_step(&mut self, fabric: &mut Fabric) {
        let buds = self.buds.clone();
        self.buds.clear();
        for bud in buds {
            let (new_buds, new_marks) = self.execute_bud(fabric, bud);
            self.buds.extend(new_buds);
            self.marks.extend(new_marks);
        }
    }

    fn execute_bud(
        &self,
        fabric: &mut Fabric,
        Bud {
            face_key,
            column_style,
            scale,
            nodes,
            branch_path,
        }: Bud,
    ) -> (Vec<Bud>, Vec<FaceMark>) {
        let (mut buds, mut marks) = (vec![], vec![]);
        if let Some(style) = column_style.filter(|s| s.count > 0) {
            let face = fabric.expect_face(face_key);
            let spin = if style.is_alternating() {
                face.spin.mirror()
            } else {
                face.spin
            };
            let (brick_name, brick_role) = match spin {
                Spin::Left => (BrickName::SingleTwistLeft, BrickRole::OnSpinLeft),
                Spin::Right => (BrickName::SingleTwistRight, BrickRole::OnSpinRight),
            };
            let brick = brick_library::get_brick(brick_name, brick_role);
            let next_path = branch_path.extend(COLUMN_MARKER);
            let (base_face, brick_faces) = fabric.attach_brick(
                &brick,
                brick_role,
                FaceRotation::Zero,
                scale.as_factor(),
                BaseFace::ExistingFace(face_key),
                &next_path,
            );
            fabric.join_faces(base_face, face_key);
            // Filter out base_face since it was deleted by join_faces
            let next_face_key: FaceKey = brick_faces
                .into_iter()
                .filter(|brick_face| *brick_face != base_face)
                .find(|brick_face| {
                    fabric
                        .expect_face(*brick_face)
                        .aliases
                        .iter()
                        .any(|FaceAlias { face_name, .. }| *face_name == AttachNext)
                })
                .expect(format!("Brick {}: next face not found", brick_name).as_str());
            buds.push(Bud {
                face_key: next_face_key,
                column_style: style.decrement(),
                scale,
                nodes,
                branch_path: next_path,
            });
        } else if !nodes.is_empty() {
            for (branch_index, child_node) in nodes.iter().enumerate() {
                let child_path = branch_path.extend(branch_index as u8);
                let (node_buds, node_marks) = Self::execute_node(
                    fabric,
                    IdentifiedFace(face_key),
                    child_node,
                    vec![],
                    self.seed_altitude,
                    child_path,
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
        faces: Vec<FaceKey>,
        seed_altitude: f32,
        branch_path: JointPath,
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
                    seed_altitude,
                    branch_path,
                );
            }
            Column {
                style,
                scale,
                post_column_nodes,
                ..
            } => {
                let face_key =
                    Self::find_launch_face(&launch, &faces, fabric).expect("No launch face");
                buds.push(Bud {
                    face_key,
                    column_style: Some(*style),
                    scale: *scale,
                    nodes: post_column_nodes.clone(),
                    branch_path,
                })
            }
            Hub {
                brick_name,
                brick_role,
                face_nodes,
                rotation,
                scale,
            } => {
                let brick = brick_library::get_brick(*brick_name, *brick_role);
                let launch_face = Self::find_launch_face(&launch, &faces, fabric);
                let base_face =
                    launch_face
                        .map(BaseFace::ExistingFace)
                        .unwrap_or(BaseFace::Seeded {
                            altitude: seed_altitude,
                        });
                let (base_face_key, brick_faces) = fabric.attach_brick(
                    &brick,
                    *brick_role,
                    rotation.into(),
                    scale.as_factor(),
                    base_face,
                    &branch_path,
                );
                // Filter out base_face_key only if it was deleted by join_faces
                let available_faces: Vec<_> = if let Some(face_key) = launch_face {
                    fabric.join_faces(base_face_key, face_key);
                    brick_faces
                        .iter()
                        .copied()
                        .filter(|&f| f != base_face_key)
                        .collect()
                } else {
                    brick_faces.clone()
                };
                for (branch_index, (hub_face_alias, hub_node)) in
                    Self::hub_pairs(face_nodes).into_iter().enumerate()
                {
                    let child_path = branch_path.extend(branch_index as u8);
                    let (new_buds, new_marks) = Self::execute_node(
                        fabric,
                        NamedFace(hub_face_alias),
                        hub_node,
                        available_faces.clone(),
                        seed_altitude,
                        child_path,
                    );
                    buds.extend(new_buds);
                    marks.extend(new_marks);
                }
            }
            Mark { mark_name } => {
                let face_key = Self::find_launch_face(&launch, &faces, fabric)
                    .expect(&format!("Unable to find face for mark: {}", mark_name));
                marks.push(FaceMark {
                    face_key,
                    mark_name: *mark_name,
                });
            }
            Prism => {
                let face_key = Self::find_launch_face(&launch, &faces, fabric)
                    .expect("Unable to find face for prism");
                fabric.add_face_prism(face_key);
            }
            Radial => {
                let face_key = Self::find_launch_face(&launch, &faces, fabric)
                    .expect("Unable to find face for radial");
                fabric.set_face_radial(face_key);
            }
        };
        (buds, marks)
    }

    fn find_launch_face(launch: &Launch, faces: &[FaceKey], fabric: &Fabric) -> Option<FaceKey> {
        match launch {
            Scratch => None,
            NamedFace(face_alias) => faces
                .iter()
                .copied()
                .find(|key| fabric.expect_face(*key).aliases.contains(face_alias)),
            IdentifiedFace(face_key) => Some(*face_key),
        }
    }

    fn hub_pairs(nodes: &[BuildNode]) -> Vec<(FaceAlias, &BuildNode)> {
        nodes
            .iter()
            .map(|face_node| {
                let Face { alias, node } = face_node else {
                    unreachable!("Hub can only contain Face nodes");
                };
                (alias.clone(), node.as_ref())
            })
            .collect()
    }
}
