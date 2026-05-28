use crate::build::dsl::brick::{Axis, BrickPrototype};
use crate::build::dsl::brick_dsl::FaceName::AttachNext;
use crate::build::dsl::brick_dsl::{BrickName, BrickRole, FaceLabel, JointName, OmniCategory};
use crate::build::dsl::build_phase::BuildNode::*;
use crate::build::dsl::build_phase::Launch::*;
use crate::build::dsl::fabric_dsl::Rotation;
use crate::build::dsl::{brick_library, FaceAlias, FaceLabelBinding};
use crate::fabric::brick::BaseFace;
use crate::fabric::face::FaceRotation;
use crate::fabric::joint::JointLabel;
use crate::fabric::joint_path::{JointPath, COLUMN_MARKER, PRISM_MARKER};
use crate::fabric::{Fabric, FaceKey, JointKey};
use crate::units::{Percent, Unit};
use std::collections::HashMap;
use std::convert::Into;

#[derive(Debug, Default, Clone, Copy)]
pub struct LabelContext {
    pub letter: Option<char>,
    pub brick_depth: u8,
    pub twist: u8,
}

#[derive(Debug, Default, Clone)]
pub struct Bud {
    face_key: FaceKey,
    column_count: usize,
    scale: Percent,
    nodes: Vec<BuildNode>,
    branch_path: JointPath,
    label_context: LabelContext,
    /// Rotation for the first column-brick attach of this bud. Set by
    /// `.rotate(...)` on the FaceColumnBuilder; consumed by the first
    /// brick then reset to `Zero` for subsequent depths.
    rotation: Rotation,
}

#[derive(Debug, Clone)]
pub enum BuildNode {
    Face {
        alias: FaceAlias,
        node: Box<BuildNode>,
    },
    Column {
        count: usize,
        scale: Percent,
        /// Rotation (about the attach-face normal) applied to the FIRST
        /// brick of the column. Subsequent bricks extend off AttachNext
        /// with no extra rotation.
        rotation: Rotation,
        post_column_nodes: Vec<BuildNode>,
    },
    Label {
        face_label: FaceLabel,
    },
    Hub {
        brick_name: BrickName,
        /// `None` = auto-pick from the parent face spin at attach time.
        /// `Some(role)` = explicit override (or seed orientation).
        brick_role: Option<BrickRole>,
        scale: Percent,
        face_nodes: Vec<BuildNode>,
    },
    Prism { outer_percent: Percent },
    RadialsOnly,
    Open,
}

impl BuildNode {
    pub fn traverse(&self, f: &mut impl FnMut(&Self)) {
        f(self);
        match self {
            Label { .. } | Prism { .. } | RadialsOnly | Open => {}
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
    pub labels: Vec<FaceLabelBinding>,
    pub seed_altitude: f32,
}

impl BuildPhase {
    pub fn new(root: BuildNode, seed_altitude: f32) -> Self {
        Self {
            root,
            buds: Vec::new(),
            labels: Vec::new(),
            seed_altitude,
        }
    }
}

impl BuildPhase {
    pub fn init(&mut self, fabric: &mut Fabric) {
        let build_scale = fabric.dimensions.scale.f32();
        let (buds, labels) = Self::execute_node(
            fabric,
            Scratch,
            &self.root,
            vec![],
            self.seed_altitude,
            build_scale,
            JointPath::default(),
            LabelContext::default(),
        );
        self.buds = buds;
        self.labels = labels;
    }

    pub fn is_building(&self) -> bool {
        !self.buds.is_empty()
    }

    pub fn build_step(&mut self, fabric: &mut Fabric) {
        let buds = self.buds.clone();
        self.buds.clear();
        for bud in buds {
            let (new_buds, new_labels) = self.execute_bud(fabric, bud);
            self.buds.extend(new_buds);
            self.labels.extend(new_labels);
        }
    }

    fn execute_bud(
        &self,
        fabric: &mut Fabric,
        Bud {
            face_key,
            column_count,
            scale,
            nodes,
            branch_path,
            label_context,
            rotation,
        }: Bud,
    ) -> (Vec<Bud>, Vec<FaceLabelBinding>) {
        let (mut buds, mut labels) = (vec![], vec![]);
        if column_count > 0 {
            let face = fabric.expect_face(face_key);
            let brick_name = BrickName::SingleTwistLeft;
            let brick_role = BrickRole::OnSpin(face.spin.mirror());
            let brick = brick_library::get_brick(brick_name, brick_role);
            let next_path = branch_path.extend(COLUMN_MARKER);
            let attached = fabric.attach_brick(
                &brick,
                brick_role,
                rotation.into(),
                scale.as_factor(),
                BaseFace::ExistingFace(face_key),
                &next_path,
            );
            label_off_axis_joints(
                fabric,
                &attached.structural_joints,
                label_context.letter,
                label_context.brick_depth,
                label_context.twist,
            );
            fabric.join_faces(attached.base_face, face_key);
            let next_face_key: FaceKey = attached
                .brick_faces
                .into_iter()
                .filter(|brick_face| *brick_face != attached.base_face)
                .find(|brick_face| {
                    fabric
                        .expect_face(*brick_face)
                        .aliases
                        .iter()
                        .any(|FaceAlias { face_name, .. }| *face_name == AttachNext)
                })
                .expect(format!("Brick {}: next face not found", brick_name).as_str());
            // Consecutive column bricks attach on alternating-spin
            // faces (AttachNext flips spin relative to Attach), so the
            // rotational offset between LEFT and RIGHT sides reverses
            // direction at each depth. Flip twist for the next bud.
            buds.push(Bud {
                face_key: next_face_key,
                column_count: column_count - 1,
                scale,
                nodes,
                branch_path: next_path,
                label_context: LabelContext {
                    letter: label_context.letter,
                    brick_depth: label_context.brick_depth + 1,
                    twist: (3 - label_context.twist) % 3,
                },
                // First-brick rotation is consumed; the rest of the
                // column extends off AttachNext with no extra twist.
                rotation: Rotation::Zero,
            });
        } else if !nodes.is_empty() {
            for (branch_index, child_node) in nodes.iter().enumerate() {
                let child_path = branch_path.extend(branch_index as u8);
                let (node_buds, node_labels) = Self::execute_node(
                    fabric,
                    IdentifiedFace(face_key),
                    child_node,
                    vec![],
                    self.seed_altitude,
                    1.0,
                    child_path,
                    label_context,
                );
                buds.extend(node_buds);
                labels.extend(node_labels);
            }
        };
        (buds, labels)
    }

    fn execute_node(
        fabric: &mut Fabric,
        launch: Launch,
        node: &BuildNode,
        faces: Vec<FaceKey>,
        seed_altitude: f32,
        build_scale: f32,
        branch_path: JointPath,
        label_context: LabelContext,
    ) -> (Vec<Bud>, Vec<FaceLabelBinding>) {
        let mut buds: Vec<Bud> = vec![];
        let mut labels: Vec<FaceLabelBinding> = vec![];
        match node {
            Face { alias, node } => {
                let build_node = node.as_ref();
                return Self::execute_node(
                    fabric,
                    NamedFace(alias.clone()),
                    build_node,
                    faces,
                    seed_altitude,
                    build_scale,
                    branch_path,
                    label_context,
                );
            }
            Column {
                count,
                scale,
                rotation,
                post_column_nodes,
            } => {
                let face_key =
                    Self::find_launch_face(&launch, &faces, fabric).expect("No launch face");
                buds.push(Bud {
                    face_key,
                    column_count: *count,
                    scale: *scale,
                    nodes: post_column_nodes.clone(),
                    branch_path,
                    label_context,
                    rotation: *rotation,
                })
            }
            Hub {
                brick_name,
                brick_role,
                face_nodes,
                scale,
            } => {
                let proto = brick_library::get_prototype(*brick_name);
                let launch_face = Self::find_launch_face(&launch, &faces, fabric);
                let is_seed = launch_face.is_none();
                // Resolve role: explicit override → that. Otherwise it's
                // `OnSpin(parent_face.spin.mirror())` — the spin the brick's
                // Attach face must show to match the parent. Seeds must
                // always be explicit.
                let resolved_role = match brick_role {
                    Some(r) => *r,
                    None => {
                        let parent_face = launch_face
                            .expect("hub(...) without role must have a parent face");
                        BrickRole::OnSpin(fabric.face(parent_face).spin.mirror())
                    }
                };
                let brick = brick_library::get_brick(*brick_name, resolved_role);
                let (base_face, effective_scale) = if let Some(fk) = launch_face {
                    (BaseFace::ExistingFace(fk), scale.as_factor())
                } else {
                    fabric.labeller = Some(std::sync::Arc::new(
                        crate::build::dsl::labelling::SymmetricOrbitLabeller,
                    ));
                    (
                        BaseFace::Seeded {
                            altitude: seed_altitude * build_scale,
                        },
                        scale.as_factor() * build_scale,
                    )
                };
                let attached = fabric.attach_brick(
                    &brick,
                    resolved_role,
                    FaceRotation::Zero,
                    effective_scale,
                    base_face,
                    &branch_path,
                );

                // Build per-face letter context for this hub's children.
                // Only labeled faces (which carry a BuildNode::Label) get letters.
                let face_letters: Vec<Option<char>> = if is_seed {
                    Self::seed_face_letters(face_nodes)
                } else {
                    // Non-seed hub: all descendants inherit the parent's letter.
                    vec![label_context.letter; face_nodes.len()]
                };
                let face_twists: Vec<u8> = if is_seed {
                    Self::seed_face_twists(face_nodes, &proto)
                } else {
                    // A non-seed Hub attach applies its own perm to the
                    // 12 hub joints (based on the bud's incoming twist),
                    // which leaves the hub's child faces mirror-aligned
                    // across the LEFT/RIGHT partnership. Children
                    // therefore start with twist 0.
                    vec![0; face_nodes.len()]
                };

                // Label the hub's structural joints.
                if is_seed {
                    let axis_to_letter = Self::seed_axis_to_letter(face_nodes, &face_letters);
                    let joint_names: Vec<JointName> = proto
                        .joints
                        .iter()
                        .copied()
                        .chain(proto.pushes.iter().flat_map(|p| [p.alpha, p.omega]))
                        .collect();
                    label_seed_joints(
                        fabric,
                        &attached.structural_joints,
                        &joint_names,
                        &axis_to_letter,
                    );
                } else {
                    label_off_axis_joints(
                        fabric,
                        &attached.structural_joints,
                        label_context.letter,
                        label_context.brick_depth,
                        label_context.twist,
                    );
                }

                let available_faces: Vec<_> = if let Some(face_key) = launch_face {
                    fabric.join_faces(attached.base_face, face_key);
                    attached
                        .brick_faces
                        .iter()
                        .copied()
                        .filter(|&f| f != attached.base_face)
                        .collect()
                } else {
                    attached.brick_faces.clone()
                };
                let child_brick_depth = if is_seed { 1 } else { label_context.brick_depth + 1 };
                for (branch_index, (mut hub_face_alias, hub_node)) in
                    Self::hub_pairs(face_nodes).into_iter().enumerate()
                {
                    // The DSL stamps face aliases at construction time with
                    // a placeholder role; rewrite to the role we actually
                    // resolved so find_launch_face matches.
                    hub_face_alias.brick_role = resolved_role;
                    let child_path = branch_path.extend(branch_index as u8);
                    let child_label_context = LabelContext {
                        letter: face_letters.get(branch_index).copied().flatten(),
                        brick_depth: child_brick_depth,
                        twist: face_twists.get(branch_index).copied().unwrap_or(label_context.twist),
                    };
                    let (new_buds, new_labels) = Self::execute_node(
                        fabric,
                        NamedFace(hub_face_alias),
                        hub_node,
                        available_faces.clone(),
                        seed_altitude,
                        1.0,
                        child_path,
                        child_label_context,
                    );
                    buds.extend(new_buds);
                    labels.extend(new_labels);
                }
            }
            Label { face_label } => {
                let face_key = Self::find_launch_face(&launch, &faces, fabric)
                    .expect(&format!("Unable to find face for label: {}", face_label));
                labels.push(FaceLabelBinding {
                    face_key,
                    face_label: *face_label,
                });
            }
            Prism { outer_percent } => {
                let face_key = Self::find_launch_face(&launch, &faces, fabric)
                    .expect("Unable to find face for prism");
                let middle_path = fabric.joints[fabric.face(face_key).middle_joint(fabric)].path.clone();
                fabric.add_face_prism(face_key, *outer_percent);
                let prism_keys: Vec<JointKey> = (0..2)
                    .filter_map(|i| {
                        let p = middle_path.extend(PRISM_MARKER).with_local_index(i);
                        fabric.joint_key_by_path(&p)
                    })
                    .collect();
                label_off_axis_joints(fabric, &prism_keys, label_context.letter, label_context.brick_depth, 0);
            }
            RadialsOnly => {
                let face_key = Self::find_launch_face(&launch, &faces, fabric)
                    .expect("Unable to find face for radials_only");
                fabric.set_face_radials_only(face_key);
            }
            Open => {
                let face_key = Self::find_launch_face(&launch, &faces, fabric)
                    .expect("Unable to find face for open");
                fabric.set_face_open(face_key);
            }
        };
        (buds, labels)
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

    /// For each seed face_node, return its assigned letter (A, B, C, ...)
    /// if it contains a Label anywhere in its subtree, otherwise None.
    fn seed_face_letters(face_nodes: &[BuildNode]) -> Vec<Option<char>> {
        let mut next_letter: u8 = b'A';
        face_nodes
            .iter()
            .map(|face_node| {
                if Self::face_node_is_labeled(face_node) {
                    let letter = next_letter as char;
                    next_letter += 1;
                    Some(letter)
                } else {
                    None
                }
            })
            .collect()
    }

    fn face_node_is_labeled(face_node: &BuildNode) -> bool {
        let mut found = false;
        face_node.traverse(&mut |n| {
            if matches!(n, Label { .. }) {
                found = true;
            }
        });
        found
    }

    /// For each seed face_node, look up the twist (0/1/2) declared on
    /// the brick FaceName referenced by its outer `Face { alias, .. }`.
    /// Faces with no twist declaration get 0.
    fn seed_face_twists(
        face_nodes: &[BuildNode],
        proto: &BrickPrototype,
    ) -> Vec<u8> {
        face_nodes
            .iter()
            .map(|face_node| {
                if let Face { alias, .. } = face_node {
                    proto.face_twist(alias.face_name)
                } else {
                    0
                }
            })
            .collect()
    }

    fn seed_axis_to_letter(
        face_nodes: &[BuildNode],
        face_letters: &[Option<char>],
    ) -> HashMap<Axis, char> {
        let mut map = HashMap::new();
        for (face_node, letter) in face_nodes.iter().zip(face_letters.iter()) {
            let Some(letter) = letter else { continue };
            let Face { alias, .. } = face_node else { continue };
            if let Some(axis) = alias.face_name.axis() {
                map.insert(axis, *letter);
            }
        }
        map
    }
}

fn label_off_axis_joints(
    fabric: &mut Fabric,
    joint_keys: &[JointKey],
    letter: Option<char>,
    brick: u8,
    twist: u8,
) {
    let Some(letter) = letter else { return };
    let n = joint_keys.len();
    // Bricks attached off mirror-paired seed faces emerge rotated
    // relative to each other (in units of 1/3 turns: twist 0/1/2). Apply
    // (twist * N / 3) label shift so mirror-partner joints share suffix.
    let shift = if n % 3 == 0 { (twist as usize * n / 3) % n } else { 0 };
    for (local_index, &key) in joint_keys.iter().enumerate() {
        let position = ((local_index + shift) % n) as u8 + 1;
        fabric.joints[key].label = Some(JointLabel::OffAxis {
            letter,
            brick,
            position,
        });
    }
}

fn label_seed_joints(
    fabric: &mut Fabric,
    joint_keys: &[JointKey],
    proto_joint_names: &[JointName],
    axis_to_letter: &HashMap<Axis, char>,
) {
    for (local_index, &key) in joint_keys.iter().enumerate() {
        let off = proto_joint_names
            .get(local_index)
            .and_then(|name| name.omni_decode())
            .and_then(|(category, axis)| {
                axis_to_letter.get(&axis).map(|&letter| JointLabel::OffAxis {
                    letter,
                    brick: 0,
                    position: omni_category_position(category),
                })
            });
        if let Some(label) = off {
            fabric.joints[key].label = Some(label);
        }
        // Unlabeled seed joints get Axial via the post-build pass.
    }
}

/// Walk structural joints (those with a push attached) and assign Axial
/// labels to any still unlabeled after the build. Runs once after the
/// build phase completes.
pub fn assign_axial_labels(fabric: &mut Fabric) {
    let mut counter: u16 = 1;
    let keys: Vec<JointKey> = fabric.joints.keys().collect();
    for key in keys {
        if fabric.joints[key].label.is_some() {
            continue;
        }
        if fabric.find_push_at(key).is_none() {
            continue;
        }
        fabric.joints[key].label = Some(JointLabel::Axial { index: counter });
        counter += 1;
    }
}

/// Omni-brick seed-joint altitude rank (1 = topmost push, 4 = bottommost),
/// based on Top-vs-Bot and Alpha-vs-Omega ordering.
fn omni_category_position(category: OmniCategory) -> u8 {
    match category {
        OmniCategory::TopOmega => 1,
        OmniCategory::TopAlpha => 2,
        OmniCategory::BotOmega => 3,
        OmniCategory::BotAlpha => 4,
    }
}
