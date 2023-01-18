use cgmath::MetricSpace;
use crate::fabric::{Fabric, UniqueId};
use crate::fabric::interval::Role::Pull;
use crate::build::tenscript::{BuildPhase, FabricPlan, FaceName, ShapePhase, Spin};
use crate::build::tenscript::FaceName::Apos;
use crate::build::tenscript::TenscriptNode;
use crate::build::tenscript::TenscriptNode::{Branch, Face, Grow, Mark};

#[allow(dead_code)]
#[derive(Clone)]
pub enum MarkAction {
    Join,
    ShapingDistance { length_factor: f32 },
    PretenstDistance { length_factor: f32 },
    Subtree { node: TenscriptNode },
}

#[derive(Clone, Debug)]
pub struct Bud {
    face_id: UniqueId,
    forward: String,
    scale_factor: f32,
    node: Option<TenscriptNode>,
}

#[derive(Clone, Debug)]
pub struct PostMark {
    face_id: UniqueId,
    mark_name: String,
}

#[derive(Debug)]
pub struct Shaper {
    interval: UniqueId,
    alpha_face: UniqueId,
    omega_face: UniqueId,
}

#[derive(Debug)]
pub struct Growth {
    pub plan: FabricPlan,
    pub pretenst_factor: f32,
    pub buds: Vec<Bud>,
    pub marks: Vec<PostMark>,
    pub shapers: Vec<Shaper>,
}

impl Growth {
    pub fn new(plan: FabricPlan) -> Self {
        Self {
            plan,
            pretenst_factor: 1.3,
            buds: vec![],
            marks: vec![],
            shapers: vec![],
        }
    }

    pub fn init(&mut self, fabric: &mut Fabric) {
        let BuildPhase { seed, root } = &self.plan.build_phase;
        let spin = seed.unwrap_or(Spin::Left);
        let node = root.clone().unwrap();
        let (buds, marks) = self.execute_node(fabric, spin, Apos, node, vec![]);
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

    pub fn needs_shaping(&self) -> bool {
        !self.marks.is_empty()
    }

    pub fn create_shapers(&mut self, fabric: &mut Fabric) {
        let ShapePhase { pull_together, .. } = &self.plan.shape_phase;
        for mark_name in pull_together {
            self.shapers.extend(self.attach_shapers(fabric, mark_name));
        }
        self.marks.clear();
    }

    pub fn complete_shapers(&mut self, fabric: &mut Fabric) {
        for shaper in &self.shapers {
            self.complete_shaper(fabric, shaper)
        }
        self.shapers.clear();
    }

    fn execute_bud(&self, fabric: &mut Fabric, Bud { face_id, forward, scale_factor, node }: Bud) -> (Vec<Bud>, Vec<PostMark>) {
        let (mut buds, mut marks) = (vec![], vec![]);
        let face = fabric.face(face_id);
        let spin = if forward.starts_with('X') { face.spin.opposite() } else { face.spin };
        if !forward.is_empty() {
            let [_, (_, a_pos_face_id)] =
                fabric.single_twist(spin, self.pretenst_factor, scale_factor, Some(face_id));
            buds.push(Bud {
                face_id: a_pos_face_id,
                forward: forward[1..].into(),
                scale_factor,
                node,
            });
        } else if let Some(node) = node {
            let (node_buds, node_marks) =
                self.execute_node(fabric, spin, Apos, node, vec![(Apos, face_id)]);
            buds.extend(node_buds);
            marks.extend(node_marks);
        };
        (buds, marks)
    }

    fn attach_shapers(&self, fabric: &mut Fabric, sought_mark_name: &str) -> Vec<Shaper> {
        let marks: Vec<_> = self.marks
            .iter()
            .filter(|PostMark { mark_name, .. }| sought_mark_name == *mark_name)
            .map(|PostMark { face_id, .. }| face_id)
            .collect();
        let mut shapers: Vec<Shaper> = vec![];
        match *marks.as_slice() {
            [alpha_id, omega_id] => {
                let (alpha, omega) = (fabric.face(*alpha_id).middle_joint(fabric), fabric.face(*omega_id).middle_joint(fabric));
                let interval = fabric.create_interval(alpha, omega, Pull, 0.3);
                shapers.push(Shaper { interval, alpha_face: *alpha_id, omega_face: *omega_id })
            }
            [_, _, _] => unimplemented!(),
            _ => panic!()
        }
        shapers
    }

    fn execute_node(&self, fabric: &mut Fabric, spin: Spin, face_name: FaceName, node: TenscriptNode, faces: Vec<(FaceName, UniqueId)>) -> (Vec<Bud>, Vec<PostMark>) {
        let mut buds: Vec<Bud> = vec![];
        let mut marks: Vec<PostMark> = vec![];
        match node {
            Face { face_name, node } => {
                let (new_buds, new_marks) = self.execute_node(fabric, spin, face_name, *node, faces);
                buds.extend(new_buds);
                marks.extend(new_marks);
            }
            Grow { forward, scale_factor, branch, .. } => {
                let face_id = faces.iter().find(|(name, _)| *name == face_name).map(|(_, face_id)| *face_id);
                let [_, (_, a_pos_face)] = fabric.single_twist(spin, self.pretenst_factor, scale_factor, face_id);
                let node = branch.map(|node_box| *node_box);
                buds.push(Bud {
                    face_id: a_pos_face,
                    forward,
                    scale_factor,
                    node,
                })
            }
            Branch { face_nodes } => {
                let pairs: Vec<(FaceName, &TenscriptNode)> = face_nodes
                    .iter()
                    .map(|face_node| {
                        let Face { face_name, node } = face_node else {
                            panic!("Branch may only contain Face nodes");
                        };
                        (*face_name, node.as_ref())
                    })
                    .collect();
                let needs_double = pairs
                    .iter()
                    .any(|(face_name, _)| *face_name != Apos);
                let base_face_id = faces
                    .iter()
                    .find(|(face_name, _)| *face_name == Apos)
                    .map(|(_, face_id)| *face_id);
                let twist_faces = if needs_double {
                    fabric.double_twist(spin, self.pretenst_factor, 1.0, base_face_id).to_vec()
                } else {
                    fabric.single_twist(spin, self.pretenst_factor, 1.0, base_face_id).to_vec()
                };
                for (face_name, node) in pairs {
                    let (new_buds, new_marks) =
                        self.execute_node(fabric, spin, face_name, node.clone(), twist_faces.clone());
                    buds.extend(new_buds);
                    marks.extend(new_marks);
                }
            }
            Mark { mark_name } => {
                let face = faces.iter().find(|(name, _)| *name == face_name);
                if let Some((_, face_id)) = face {
                    marks.push(PostMark { face_id: *face_id, mark_name });
                }
            }
        }
        (buds, marks)
    }

    fn complete_shaper(&self, fabric: &mut Fabric, Shaper { interval, alpha_face, omega_face }: &Shaper) {
        let (alpha, omega) = (fabric.face(*alpha_face), fabric.face(*omega_face));
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
        let scale = (alpha.scale + omega.scale) / 2.0;
        for (a, b) in links {
            fabric.create_interval(alpha_rotated[a], omega_ends[b], Pull, scale);
        }
        fabric.remove_interval(*interval);
        fabric.remove_face(*alpha_face);
        fabric.remove_face(*omega_face);
    }
}
