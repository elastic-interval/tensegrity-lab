use cgmath::{EuclideanSpace, InnerSpace, Matrix4, Quaternion, Rotation, Vector3};
use crate::build::tenscript::{BuildNode, FaceMark, FaceName, Seed, Spin};
use crate::build::tenscript::build_phase::Launch::{*};
use crate::build::tenscript::BuildNode::{*};
use crate::build::tenscript::FaceName::Apos;
use crate::fabric::{Fabric, UniqueId};

#[derive(Debug, Default)]
pub struct Bud {
    face_id: UniqueId,
    forward: String,
    scale_factor: f32,
    node: Option<BuildNode>,
}

#[derive(Debug)]
enum Launch {
    Seeded { seed: Seed },
    NamedFace { face_name: FaceName },
    IdentifiedFace { face_id: UniqueId },
}

#[derive(Debug, Default)]
pub struct BuildPhase {
    pub seed: Seed,
    pub root: Option<BuildNode>,
    pub pretenst_factor: f32,
    pub buds: Vec<Bud>,
    pub marks: Vec<FaceMark>,
}

impl BuildPhase {
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
                        let faces = fabric.single_twist(seed.spin(), self.pretenst_factor, *scale_factor, None);
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
                fabric.double_twist(spin, self.pretenst_factor, 1.0, face_id).to_vec()
            } else {
                fabric.single_twist(spin, self.pretenst_factor, 1.0, face_id).to_vec()
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
}