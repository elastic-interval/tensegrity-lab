use cgmath::{EuclideanSpace, InnerSpace, Matrix4, MetricSpace, Quaternion, Rotation, Vector3};

use ShapeCommand::StartCountdown;

use crate::build::tenscript::{BuildPhase, FabricPlan, FaceName, Seed, ShapeOperation, ShapePhase, Spin};
use crate::build::tenscript::BuildNode;
use crate::build::tenscript::BuildNode::{Branch, Face, Grow, Mark};
use crate::build::tenscript::FaceName::Apos;
use crate::build::tenscript::growth::Launch::{IdentifiedFace, NamedFace, Seeded};
use crate::build::tenscript::growth::ShapeCommand::{Noop, SetViscosity};
use crate::fabric::{Fabric, Link, UniqueId};

#[allow(dead_code)]
#[derive(Clone)]
pub enum MarkAction {
    Join,
    ShapingDistance { length_factor: f32 },
    PretenstDistance { length_factor: f32 },
    Subtree { node: BuildNode },
}

#[derive(Clone, Debug)]
pub struct Bud {
    face_id: UniqueId,
    forward: String,
    scale_factor: f32,
    node: Option<BuildNode>,
}

#[derive(Clone, Debug)]
pub struct FaceMark {
    face_id: UniqueId,
    mark_name: String,
}

#[derive(Debug)]
pub struct Shaper {
    interval: UniqueId,
    alpha_face: UniqueId,
    omega_face: UniqueId,
    mark_name: String,
    join: bool,
}

enum Launch {
    Seeded { seed: Seed },
    NamedFace { face_name: FaceName },
    IdentifiedFace { face_id: UniqueId },
}

#[derive(Debug)]
pub struct Growth {
    pub plan: FabricPlan,
    pub pretenst_factor: f32,
    pub buds: Vec<Bud>,
    pub marks: Vec<FaceMark>,
    pub shapers: Vec<Shaper>,
    shape_operation_index: usize,
}

const DEFAULT_ADD_SHAPER_COUNTDOWN: usize = 20_000;
const DEFAULT_VULCANIZE_COUNTDOWN: usize = 5_000;

pub enum ShapeCommand {
    Noop,
    StartCountdown(usize),
    SetViscosity(f32),
    Terminate,
}

impl Growth {
    pub fn new(plan: FabricPlan) -> Self {
        Self {
            plan,
            pretenst_factor: 1.3,
            buds: vec![],
            marks: vec![],
            shapers: vec![],
            shape_operation_index: 0,
        }
    }

    pub fn init(&mut self, fabric: &mut Fabric) {
        let BuildPhase { seed, root, .. } = &self.plan.build_phase;
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

    pub fn needs_shaping(&self) -> bool {
        !self.plan.shape_phase.operations.is_empty()
    }

    pub fn shaping_step(&mut self, fabric: &mut Fabric) -> ShapeCommand {
        let ShapePhase { operations } = &self.plan.shape_phase;
        let Some(operation) = operations.get(self.shape_operation_index) else {
            self.complete_all_shapers(fabric);
            return ShapeCommand::Terminate;
        };
        self.shape_operation_index += 1;
        self.execute_shape_operation(fabric, operation.clone())
    }

    fn complete_all_shapers(&mut self, fabric: &mut Fabric) {
        for shaper in self.shapers.split_off(0) {
            self.complete_shaper(fabric, shaper);
        }
    }

    fn execute_shape_operation(&mut self, fabric: &mut Fabric, operation: ShapeOperation) -> ShapeCommand {
        match operation {
            ShapeOperation::Join { mark_name } => {
                let faces = self.marked_faces(&mark_name);
                let joints = self.marked_middle_joints(fabric, &faces);
                match (joints.as_slice(), faces.as_slice()) {
                    (&[alpha_index, omega_index], &[alpha_face, omega_face]) => {
                        let interval = fabric.create_interval(alpha_index, omega_index, Link::pull(0.3));
                        self.shapers.push(Shaper { interval, alpha_face, omega_face, mark_name, join: true })
                    }
                    _ => unimplemented!()
                }
                StartCountdown(DEFAULT_ADD_SHAPER_COUNTDOWN)
            }
            ShapeOperation::Distance { mark_name, distance_factor } => {
                let faces = self.marked_faces(&mark_name);
                let joints = self.marked_middle_joints(fabric, &faces);
                match (joints.as_slice(), faces.as_slice()) {
                    (&[alpha_index, omega_index], &[alpha_face, omega_face]) => {
                        let length = fabric.joints[alpha_index].location.distance(fabric.joints[omega_index].location) * distance_factor;
                        let interval = fabric.create_interval(alpha_index, omega_index, Link::pull(length));
                        self.shapers.push(Shaper { interval, alpha_face, omega_face, mark_name, join: false })
                    }
                    _ => println!("Wrong number of faces for mark {mark_name}"),
                }
                StartCountdown(DEFAULT_ADD_SHAPER_COUNTDOWN)
            }
            ShapeOperation::RemoveShapers { mark_names } => {
                if mark_names.is_empty() {
                    self.complete_all_shapers(fabric);
                } else {
                    for mark_name in mark_names {
                        let Some(index) = self.shapers
                            .iter()
                            .enumerate()
                            .find_map(|(index, shaper)| (shaper.mark_name == mark_name).then_some(index)) else {
                            panic!("no such shaper with mark name: '{mark_name}'")
                        };
                        let shaper = self.shapers.remove(index);
                        self.complete_shaper(fabric, shaper);
                    }
                }
                Noop
            }
            ShapeOperation::Countdown { count, operations } => {
                for operation in operations {
                    // ignores the countdown returned from each sub-operation
                    self.execute_shape_operation(fabric, operation);
                }
                StartCountdown(count)
            }
            ShapeOperation::Vulcanize => {
                fabric.install_bow_ties();
                StartCountdown(DEFAULT_VULCANIZE_COUNTDOWN)
            }
            ShapeOperation::ReplaceFaces => {
                fabric.replace_faces();
                Noop
            }
            ShapeOperation::SetViscosity { viscosity } =>
                SetViscosity(viscosity),
        }
    }

    fn execute_bud(&self, fabric: &mut Fabric, Bud { face_id, forward, scale_factor, node }: Bud) -> (Vec<Bud>, Vec<FaceMark>) {
        let (mut buds, mut marks) = (vec![], vec![]);
        let face = fabric.face(face_id);
        let spin = if forward.starts_with('X') { face.spin.opposite() } else { face.spin };
        if !forward.is_empty() {
            let faces = fabric.single_twist(spin, self.pretenst_factor, scale_factor, Some(face_id));
            buds.push(Bud {
                face_id: Growth::find_face_id(Apos, faces.to_vec()),
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
                        let faces = fabric.single_twist(seed.spin(), self.pretenst_factor, *scale_factor, None);
                        return self.execute_node(fabric, NamedFace { face_name: Apos }, node, faces.to_vec());
                    }
                    NamedFace { face_name } => Growth::find_face_id(face_name, faces),
                    IdentifiedFace { face_id } => face_id,
                };
                let node = post_growth_node.clone().map(|node_box| *node_box);
                buds.push(Bud { face_id, forward: forward.clone(), scale_factor: *scale_factor, node })
            }
            Branch { face_nodes } => {
                let pairs = Growth::branch_pairs(face_nodes);
                let any_special_face = pairs.iter().any(|(face_name, _)| *face_name != Apos);
                let (spin, face_id, needs_double) = match launch {
                    Seeded { seed } => (seed.spin(), None, seed.needs_double()),
                    NamedFace { face_name } => {
                        let face_id = Growth::find_face_id(face_name, faces);
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
                    NamedFace { face_name } => Growth::find_face_id(face_name, faces),
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
                fabric.double_twist(spin, self.pretenst_factor, 1.0, face_id).to_vec()
            } else {
                fabric.single_twist(spin, self.pretenst_factor, 1.0, face_id).to_vec()
            };
        let Seed { down_faces, .. } = &self.plan.build_phase.seed;
        if face_id.is_none() && !down_faces.is_empty() {
            Self::orient_fabric(fabric, &faces, down_faces);
        }
        faces
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

    fn marked_middle_joints(&self, fabric: &Fabric, face_ids: &[UniqueId]) -> Vec<usize> {
        face_ids
            .iter()
            .map(|face_id| fabric.face(*face_id).middle_joint(fabric))
            .collect()
    }

    fn marked_faces(&self, mark_name: &String) -> Vec<UniqueId> {
        self.marks
            .iter()
            .filter(|post_mark| *mark_name == post_mark.mark_name)
            .map(|FaceMark { face_id, .. }| *face_id)
            .collect()
    }

    fn complete_shaper(&self, fabric: &mut Fabric, Shaper { interval, alpha_face, omega_face, join, .. }: Shaper) {
        if join {
            self.join_faces(fabric, alpha_face, omega_face);
        }
        fabric.remove_interval(interval);
    }

    fn join_faces(&self, fabric: &mut Fabric, alpha_id: UniqueId, omega_id: UniqueId) {
        let (alpha, omega) = (fabric.face(alpha_id), fabric.face(omega_id));
        let (mut alpha_ends, omega_ends) = (alpha.radial_joints(fabric), omega.radial_joints(fabric));
        alpha_ends.reverse();
        let (mut alpha_points, omega_points) = (
            alpha_ends.map(|id| fabric.location(id)),
            omega_ends.map(|id| fabric.location(id))
        );
        let links = [(0, 0), (0, 1), (1, 1), (1, 2), (2, 2), (2, 0)];
        let (_, alpha_rotated) = (0..3)
            .map(|rotation| {
                let length: f32 = links
                    .map(|(a, b)| alpha_points[a].distance(omega_points[b]))
                    .iter()
                    .sum();
                alpha_points.rotate_right(1);
                let mut rotated = alpha_ends;
                rotated.rotate_right(rotation);
                (length, rotated)
            })
            .min_by(|(length_a, _), (length_b, _)| length_a.partial_cmp(length_b).unwrap())
            .unwrap();
        let ideal = (alpha.scale + omega.scale) / 2.0;
        for (a, b) in links {
            fabric.create_interval(alpha_rotated[a], omega_ends[b], Link::pull(ideal));
        }
        fabric.remove_face(alpha_id);
        fabric.remove_face(omega_id);
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

    fn find_face_id(face_name: FaceName, face_list: Vec<(FaceName, UniqueId)>) -> UniqueId {
        face_list
            .iter()
            .find(|(name, _)| *name == face_name)
            .map(|(_, face_id)| *face_id)
            .unwrap()
    }
}
