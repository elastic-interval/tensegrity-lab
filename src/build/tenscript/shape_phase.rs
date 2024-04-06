use std::cmp::Ordering;
use cgmath::{EuclideanSpace, InnerSpace, Matrix4, MetricSpace, Point3, Quaternion, Vector3};
use pest::iterators::Pair;

use crate::build::tenscript::shape_phase::ShapeCommand::*;
use crate::build::tenscript::{FaceAlias, Rule, Spin};
use crate::build::tenscript::{FaceMark, TenscriptError};
use crate::build::tenscript::brick_library::BrickLibrary;
use crate::fabric::{Fabric, Link, UniqueId};
use crate::fabric::brick::BaseFace;
use crate::fabric::face::{FaceRotation, vector_space};

const DEFAULT_ADD_SHAPER_COUNTDOWN: usize = 25_000;
const DEFAULT_VULCANIZE_COUNTDOWN: usize = 5_000;

#[derive(Debug)]
pub enum ShapeCommand {
    Noop,
    StartCountdown(usize),
    SetViscosity(f32),
    Terminate,
}

#[derive(Debug, Clone)]
pub enum ShapeOperation {
    Countdown {
        count: usize,
        operations: Vec<ShapeOperation>,
    },
    Joiner {
        mark_name: String,
        seed: Option<usize>,
    },
    PointDownwards {
        mark_name: String,
    },
    Spacer {
        mark_name: String,
        distance_factor: f32,
    },
    RemoveSpacers {
        mark_names: Vec<String>,
    },
    Vulcanize,
    FacesToTriangles,
    SetViscosity {
        viscosity: f32,
    },
}

impl ShapeOperation {
    pub fn traverse(&self, f: &mut impl FnMut(&Self)) {
        f(self);
        if let ShapeOperation::Countdown { operations, .. } = self {
            for operation in operations {
                operation.traverse(f);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Shaper {
    interval: UniqueId,
    alpha_face: UniqueId,
    omega_face: UniqueId,
    mark_name: String,
}

#[derive(Debug, Clone)]
pub struct ShapePhase {
    pub operations: Vec<ShapeOperation>,
    pub marks: Vec<FaceMark>,
    pub joiners: Vec<Shaper>,
    pub spacers: Vec<Shaper>,
    shape_operation_index: usize,
}

impl ShapePhase {
    pub fn from_pair_option(pair: Option<Pair<Rule>>) -> Result<ShapePhase, TenscriptError> {
        let operations = match pair {
            None => Vec::new(),
            Some(pair) => Self::parse_shape_operations(pair.into_inner())?,
        };
        Ok(ShapePhase {
            operations,
            marks: Vec::new(),
            joiners: Vec::new(),
            spacers: Vec::new(),
            shape_operation_index: 0,
        })
    }

    fn parse_shape_operations<'a>(
        pairs: impl Iterator<Item=Pair<'a, Rule>>,
    ) -> Result<Vec<ShapeOperation>, TenscriptError> {
        pairs.map(Self::parse_shape_operation).collect()
    }

    fn parse_shape_operation(pair: Pair<Rule>) -> Result<ShapeOperation, TenscriptError> {
        match pair.as_rule() {
            Rule::basic_shape_operation | Rule::shape_operation => {
                Self::parse_shape_operation(pair.into_inner().next().unwrap())
            }
            Rule::spacer => {
                let mut inner = pair.into_inner();
                let [mark_name, distance_string] = [
                    inner.next().unwrap().as_str(),
                    inner.next().unwrap().as_str(),
                ];
                let distance_factor = TenscriptError::parse_float(distance_string, "(space ..)")?;
                Ok(ShapeOperation::Spacer {
                    mark_name: mark_name[1..].into(),
                    distance_factor,
                })
            }
            Rule::joiner => {
                let mut inner = pair.into_inner();
                let mark_name = inner.next().unwrap().as_str();
                let seed = match inner.next() {
                    None => None,
                    Some(seed_pair) => {
                        let index =
                            TenscriptError::parse_usize(seed_pair.into_inner().next().unwrap().as_str(), "(seed ...)")?;
                        Some(index)
                    }
                };
                Ok(ShapeOperation::Joiner {
                    mark_name: mark_name[1..].into(),
                    seed,
                })
            }
            Rule::down => {
                let mark_name = pair.into_inner().next().unwrap().as_str();
                Ok(ShapeOperation::PointDownwards {
                    mark_name: mark_name[1..].into(),
                })
            }
            Rule::during_count => {
                let mut inner = pair.into_inner();
                let count =
                    TenscriptError::parse_usize(inner.next().unwrap().as_str(), "(during ...)")?;
                let operations = Self::parse_shape_operations(inner)?;
                Ok(ShapeOperation::Countdown { count, operations })
            }
            Rule::remove_spacers => {
                let mark_names = pair.into_inner().map(|p| p.as_str()[1..].into()).collect();
                Ok(ShapeOperation::RemoveSpacers { mark_names })
            }
            Rule::faces_to_triangles => Ok(ShapeOperation::FacesToTriangles),
            Rule::vulcanize => Ok(ShapeOperation::Vulcanize),
            Rule::set_viscosity => {
                let viscosity = TenscriptError::parse_float_inside(pair, "(viscosity ..)")?;
                Ok(ShapeOperation::SetViscosity { viscosity })
            }
            _ => unreachable!("shape phase: {pair}"),
        }
    }

    pub fn needs_shaping(&self) -> bool {
        !self.operations.is_empty()
    }

    pub fn shaping_step(&mut self, fabric: &mut Fabric, brick_library: &BrickLibrary) -> Result<ShapeCommand, TenscriptError> {
        if let Some(countdown) = self.complete_joiners(fabric) {
            return Ok(countdown);
        }
        let Some(operation) = self.operations.get(self.shape_operation_index) else {
            self.remove_spacers(fabric);
            return Ok(Terminate);
        };
        self.shape_operation_index += 1;
        self.execute_shape_operation(fabric, brick_library, operation.clone())
    }

    pub fn complete_joiners(&mut self, fabric: &mut Fabric) -> Option<ShapeCommand> {
        let mut removed = false;
        for Shaper {
            interval,
            alpha_face,
            omega_face,
            ..
        } in self.joiners.split_off(0)
        {
            fabric.remove_interval(interval);
            fabric.join_faces(alpha_face, omega_face);
            removed = true;
        }
        removed.then_some(StartCountdown(30000))
    }

    fn execute_shape_operation(
        &mut self,
        fabric: &mut Fabric,
        brick_library: &BrickLibrary,
        operation: ShapeOperation,
    ) -> Result<ShapeCommand, TenscriptError> {
        Ok(match operation {
            ShapeOperation::Joiner { mark_name, seed } => {
                let faces = self.marked_faces(&mark_name);
                let joints = self.marked_middle_joints(fabric, &faces);
                match faces.len() {
                    2 => {
                        let interval = fabric.create_interval(joints[0], joints[1], Link::pull(0.01));
                        self.joiners.push(Shaper { interval, alpha_face: faces[0], omega_face: faces[1], mark_name })
                    }
                    3 => {
                        let face0 = fabric.face(faces[0]);
                        let face1 = fabric.face(faces[1]);
                        let face2 = fabric.face(faces[2]);
                        let spin = face0.spin;
                        if face1.spin != spin || face2.spin != spin {
                            panic!("Faces must have the same spin");
                        }
                        let scale = (face0.scale + face1.scale + face2.scale) / 3.0;
                        let midpoint0 = face0.midpoint(fabric);
                        let midpoint1 = face1.midpoint(fabric);
                        let midpoint2 = face2.midpoint(fabric);
                        let normal0 = face0.normal(fabric);
                        let normal1 = face1.normal(fabric);
                        let normal2 = face2.normal(fabric);
                        let normal = (normal0 + normal1 + normal2).normalize();
                        let midpoint = (midpoint0 + midpoint1 + midpoint2) / 3.0 + normal * 3.0;
                        let ray0 = (midpoint0 - midpoint).normalize_to(scale);
                        let ray1 = (midpoint1 - midpoint).normalize_to(scale);
                        let ray2 = (midpoint2 - midpoint).normalize_to(scale);
                        let spin_normal = match spin {
                            Spin::Left => ray0.cross(ray1).normalize(),
                            Spin::Right => ray1.cross(ray0).normalize(),
                        };
                        let points = if spin_normal.dot(normal) > 0.0 {
                            [ray0 + midpoint, ray1 + midpoint, ray2 + midpoint]
                        } else {
                            [ray0 + midpoint, ray2 + midpoint, ray1 + midpoint]
                        }.map(Point3::from_vec);
                        let vector_space = vector_space(points, scale, spin, FaceRotation::Zero);
                        let base_face = BaseFace::Situated { spin, vector_space, seed };
                        let alias = FaceAlias::single("Omni");
                        let (_base_face_id, brick_faces) = fabric.create_brick(
                            &alias,
                            FaceRotation::Zero,
                            scale,
                            base_face,
                            brick_library,
                        );
                        let mut brick_face_midpoints = Vec::new();
                        for brick_face_id in brick_faces {
                            let face = fabric.face(brick_face_id);
                            brick_face_midpoints.push((brick_face_id, face.midpoint(fabric), face.middle_joint(fabric)));
                        }
                        let mut far_face_midpoints = Vec::new();
                        for face_id in faces {
                            let face = fabric.face(face_id);
                            far_face_midpoints.push((face_id, face.midpoint(fabric), face.middle_joint(fabric)));
                        }
                        let shapers = far_face_midpoints
                            .into_iter()
                            .map(|(far_face_id, far_face_midpoint, far_joint)| {
                                let brick_face = brick_face_midpoints
                                    .iter()
                                    .min_by(|(_, location_a, _), (_, location_b, _)| {
                                        let (dx, dy) = (location_a.distance2(far_face_midpoint), location_b.distance2(far_face_midpoint));
                                        if dx < dy {
                                            Ordering::Less
                                        } else if dx > dy {
                                            Ordering::Greater
                                        } else {
                                            Ordering::Equal
                                        }
                                    });
                                let (near_face_id, _, near_joint) = *brick_face.expect("Expected a closest face");
                                (near_face_id, near_joint, far_face_id, far_joint)
                            });
                        for (near_face_id, near_joint, far_face_id, far_joint) in shapers {
                            let interval = fabric.create_interval(near_joint, far_joint, Link::pull(0.01));
                            self.joiners.push(Shaper { interval, alpha_face: near_face_id, omega_face: far_face_id, mark_name: mark_name.clone() })
                        }
                    }
                    _ => unimplemented!("Join can only be 2 or three faces")
                }
                StartCountdown(DEFAULT_ADD_SHAPER_COUNTDOWN)
            }
            ShapeOperation::PointDownwards { mark_name } => {
                let results: Result<Vec<_>, TenscriptError> = self
                    .marked_faces(&mark_name)
                    .into_iter()
                    .map(|id| fabric.expect_face(id))
                    .collect();
                let faces = results?;
                let down = faces
                    .into_iter()
                    .map(|face| face.normal(fabric))
                    .sum::<Vector3<f32>>()
                    .normalize();
                let quaternion = Quaternion::from_arc(down, -Vector3::unit_y(), None);
                fabric.apply_matrix4(Matrix4::from(quaternion));
                fabric.centralize();
                fabric.set_altitude(1.0);
                Noop
            }
            ShapeOperation::Spacer {
                mark_name,
                distance_factor,
            } => {
                let faces = self.marked_faces(&mark_name);
                let joints = self.marked_middle_joints(fabric, &faces);
                for alpha in 0..faces.len() - 1 {
                    for omega in (alpha + 1)..faces.len() {
                        let alpha_index = joints[alpha];
                        let omega_index = joints[omega];
                        let length = fabric
                            .joints[alpha_index]
                            .location
                            .distance(fabric.joints[omega_index].location) * distance_factor;
                        let interval = fabric.create_interval(alpha_index, omega_index, Link::pull(length));
                        self.spacers.push(Shaper {
                            interval,
                            alpha_face: faces[alpha],
                            omega_face: faces[omega],
                            mark_name: mark_name.clone(),
                        })
                    }
                }
                StartCountdown(DEFAULT_ADD_SHAPER_COUNTDOWN)
            }
            ShapeOperation::RemoveSpacers { mark_names } => {
                if mark_names.is_empty() {
                    self.remove_spacers(fabric);
                } else {
                    for mark_name in mark_names {
                        let index = self
                            .spacers
                            .iter()
                            .enumerate()
                            .find_map(|(index, shaper)| {
                                (shaper.mark_name == mark_name).then_some(index)
                            })
                            .expect("undefined mark");
                        let Shaper { interval, .. } = self.spacers.remove(index);
                        fabric.remove_interval(interval);
                    }
                }
                Noop
            }
            ShapeOperation::Countdown { count, operations } => {
                for operation in operations {
                    // ignores the countdown returned from each sub-operation
                    let _ = self.execute_shape_operation(fabric, brick_library, operation);
                }
                StartCountdown(count)
            }
            ShapeOperation::Vulcanize => {
                fabric.install_bow_ties();
                StartCountdown(DEFAULT_VULCANIZE_COUNTDOWN)
            }
            ShapeOperation::FacesToTriangles => {
                self.remove_spacers(fabric);
                for face_id in fabric.faces_to_triangles() {
                    fabric.remove_face(face_id);
                }
                Noop
            }
            ShapeOperation::SetViscosity { viscosity } => SetViscosity(viscosity),
        })
    }

    fn marked_faces(&self, mark_name: &String) -> Vec<UniqueId> {
        self.marks
            .iter()
            .filter(|post_mark| *mark_name == post_mark.mark_name)
            .map(|FaceMark { face_id, .. }| *face_id)
            .collect()
    }

    fn marked_middle_joints(&self, fabric: &Fabric, face_ids: &[UniqueId]) -> Vec<usize> {
        face_ids
            .iter()
            .map(|face_id| fabric.face(*face_id).middle_joint(fabric))
            .collect()
    }

    fn remove_spacers(&mut self, fabric: &mut Fabric) {
        for Shaper { interval, .. } in self.spacers.split_off(0) {
            fabric.remove_interval(interval);
        }
    }
}
