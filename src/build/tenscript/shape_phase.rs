use cgmath::{EuclideanSpace, InnerSpace, Matrix4, MetricSpace, Point3, Quaternion, Vector3};
use itertools::Itertools;
use pest::iterators::Pair;
use std::cmp::Ordering;

use crate::build::tenscript::brick_library::BrickLibrary;
use crate::build::tenscript::shape_phase::ShapeCommand::*;
use crate::build::tenscript::{
    parse_float, parse_float_inside, parse_usize, FaceAlias, Rule, Spin,
};
use crate::build::tenscript::{FaceMark, TenscriptError};
use crate::fabric::brick::BaseFace;
use crate::fabric::face::{vector_space, FaceRotation};
use crate::fabric::material::Material::{GuyWireMaterial, PullMaterial};
use crate::fabric::{Fabric, UniqueId};

const DEFAULT_ADD_SHAPER_COUNTDOWN: usize = 25_000;
const DEFAULT_VULCANIZE_COUNTDOWN: usize = 5_000;
const DEFAULT_PRISM_COUNTDOWN: usize = 5_000;
const DEFAULT_JOINER_COUNTDOWN: usize = 30_000;

#[derive(Debug)]
pub enum ShapeCommand {
    Noop,
    StartCountdown(usize),
    Stiffness(f32),
    Drag(f32),
    Viscosity(f32),
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
    Anchor {
        joint_index: usize,
        surface: (f32, f32),
    },
    GuyLine {
        joint_index: usize,
        length: f32,
        surface: (f32, f32),
    },
    RemoveSpacers {
        mark_names: Vec<String>,
    },
    Vulcanize,
    FacesToTriangles,
    FacesToPrisms {
        mark_names: Vec<String>,
    },
    SetStiffness(f32),
    SetDrag(f32),
    SetViscosity(f32),
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
pub enum ShapeInterval {
    FaceJoiner {
        interval: UniqueId,
        alpha_face: UniqueId,
        omega_face: UniqueId,
        mark_name: String,
    },
    FaceSpacer {
        interval: UniqueId,
        alpha_face: UniqueId,
        omega_face: UniqueId,
        mark_name: String,
    },
    SurfaceAnchor {
        interval: UniqueId,
    },
    GuyLine {
        interval: UniqueId,
    },
}

#[derive(Debug, Clone)]
pub struct ShapePhase {
    pub operations: Vec<ShapeOperation>,
    pub marks: Vec<FaceMark>,
    pub shape_intervals: Vec<ShapeInterval>,
    shape_operation_index: usize,
}

impl ShapePhase {
    pub fn from_pair(pair: Pair<Rule>) -> Result<ShapePhase, TenscriptError> {
        let operations = Self::parse_shape_operations(pair.into_inner())?;
        Ok(ShapePhase {
            operations,
            marks: Vec::new(),
            shape_intervals: Vec::new(),
            shape_operation_index: 0,
        })
    }

    fn parse_shape_operations<'a>(
        pairs: impl Iterator<Item = Pair<'a, Rule>>,
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
                let distance_factor = parse_float(distance_string, "(space ..)")?;
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
                        let index = parse_usize(
                            seed_pair.into_inner().next().unwrap().as_str(),
                            "(seed ...)",
                        )?;
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
                let count = parse_usize(inner.next().unwrap().as_str(), "(during ...)")?;
                let operations = Self::parse_shape_operations(inner)?;
                Ok(ShapeOperation::Countdown { count, operations })
            }
            Rule::remove_spacers => {
                let mark_names = pair.into_inner().map(|p| p.as_str()[1..].into()).collect();
                Ok(ShapeOperation::RemoveSpacers { mark_names })
            }
            Rule::faces_to_triangles => Ok(ShapeOperation::FacesToTriangles),
            Rule::faces_to_prisms => {
                let mark_names = pair.into_inner().map(|p| p.as_str()[1..].into()).collect();
                Ok(ShapeOperation::FacesToPrisms { mark_names })
            }
            Rule::vulcanize => Ok(ShapeOperation::Vulcanize),
            Rule::set_stiffness => {
                let percent = parse_float_inside(pair, "(set-stiffness ..)")?;
                Ok(ShapeOperation::SetStiffness(percent))
            }
            Rule::set_drag => {
                let percent = parse_float_inside(pair, "(set-drag ..)")?;
                Ok(ShapeOperation::SetDrag(percent))
            }
            Rule::set_viscosity => {
                let percent = parse_float_inside(pair, "(set-viscosity ..)")?;
                Ok(ShapeOperation::SetViscosity(percent))
            }
            Rule::anchor => {
                let mut inner = pair.into_inner();
                let joint_index = parse_usize(inner.next().unwrap().as_str(), "(anchor ...)")?;
                let surface = Self::parse_surface_location(inner.next().unwrap())?;
                let operation = ShapeOperation::Anchor {
                    joint_index,
                    surface,
                };
                Ok(operation)
            }
            Rule::guy_line => {
                let mut inner = pair.into_inner();
                let joint_index =
                    parse_usize(inner.next().unwrap().as_str(), "(guy-line joint-index ...)")?;
                let length =
                    parse_float(inner.next().unwrap().as_str(), "(guy-line <> length ...)")?;
                let surface = Self::parse_surface_location(inner.next().unwrap())?;
                let operation = ShapeOperation::GuyLine {
                    joint_index,
                    length,
                    surface,
                };
                Ok(operation)
            }
            _ => unreachable!("shape phase: {pair}"),
        }
    }

    fn parse_surface_location(pair: Pair<Rule>) -> Result<(f32, f32), TenscriptError> {
        let mut inner = pair.into_inner();
        let x = parse_float(inner.next().unwrap().as_str(), "(surface x ..)")?;
        let z = parse_float(inner.next().unwrap().as_str(), "(surface .. z)")?;
        Ok((x, z))
    }

    pub fn needs_shaping(&self) -> bool {
        !self.operations.is_empty()
    }

    pub fn shaping_step(
        &mut self,
        fabric: &mut Fabric,
        brick_library: &BrickLibrary,
    ) -> Result<ShapeCommand, TenscriptError> {
        if let Some(countdown) = self.complete_joiners(fabric) {
            return Ok(countdown);
        }
        let Some(operation) = self.operations.get(self.shape_operation_index) else {
            self.remove_spacers(fabric, vec![]);
            self.remove_anchors(fabric);
            return Ok(Terminate);
        };
        self.shape_operation_index += 1;
        self.execute_shape_operation(fabric, brick_library, operation.clone())
    }

    pub fn complete_joiners(&mut self, fabric: &mut Fabric) -> Option<ShapeCommand> {
        let before = self.shape_intervals.len();
        self.shape_intervals = self
            .shape_intervals
            .iter()
            .cloned()
            .filter(|shape_interval| {
                if let ShapeInterval::FaceJoiner {
                    interval,
                    alpha_face,
                    omega_face,
                    ..
                } = shape_interval
                {
                    fabric.remove_interval(*interval);
                    fabric.join_faces(*alpha_face, *omega_face);
                    false
                } else {
                    true
                }
            })
            .collect();
        (self.shape_intervals.len() < before).then_some(StartCountdown(DEFAULT_JOINER_COUNTDOWN))
    }

    fn execute_shape_operation(
        &mut self,
        fabric: &mut Fabric,
        brick_library: &BrickLibrary,
        operation: ShapeOperation,
    ) -> Result<ShapeCommand, TenscriptError> {
        Ok(match operation {
            ShapeOperation::Joiner { mark_name, seed } => {
                let face_ids = self.marked_faces(&mark_name);
                let joints = self.marked_middle_joints(fabric, &face_ids);
                match face_ids.len() {
                    2 => {
                        let interval =
                            fabric.create_interval(joints[0], joints[1], 0.01, PullMaterial);
                        self.shape_intervals.push(ShapeInterval::FaceJoiner {
                            interval,
                            alpha_face: face_ids[0],
                            omega_face: face_ids[1],
                            mark_name,
                        });
                    }
                    3 => {
                        let face_ids = [face_ids[0], face_ids[1], face_ids[2]];
                        let faces = face_ids.map(|id| fabric.face(id));
                        let spin = faces[0].spin;
                        if faces[1].spin != spin || faces[2].spin != spin {
                            panic!("Faces must have the same spin");
                        }
                        let scale = (faces[0].scale + faces[1].scale + faces[2].scale) / 3.0;
                        let face_midpoints = faces.map(|face| face.midpoint(fabric));
                        let face_normals = faces.map(|face| face.normal(fabric));
                        let normal =
                            (face_normals[0] + face_normals[1] + face_normals[2]).normalize();
                        let midpoint = (face_midpoints[0] + face_midpoints[1] + face_midpoints[2])
                            / 3.0
                            + normal * 3.0;
                        let rays = face_midpoints
                            .map(|face_mid| (face_mid - midpoint).normalize_to(scale));
                        let spin_normal = match spin {
                            Spin::Left => rays[0].cross(rays[1]).normalize(),
                            Spin::Right => rays[1].cross(rays[0]).normalize(),
                        };
                        let ordered_rays = if spin_normal.dot(normal) > 0.0 {
                            [rays[0], rays[1], rays[2]]
                        } else {
                            [rays[0], rays[2], rays[1]]
                        };
                        let points = ordered_rays.map(|ray| ray + midpoint).map(Point3::from_vec);
                        let vector_space = vector_space(points, scale, spin, FaceRotation::Zero);
                        let base_face = BaseFace::Situated {
                            spin,
                            vector_space,
                            seed,
                        };
                        let alias = FaceAlias::single("Omni");
                        let (_, brick_faces) = fabric.create_brick(
                            &alias,
                            FaceRotation::Zero,
                            scale,
                            base_face,
                            brick_library,
                        )?;
                        let mut brick_face_midpoints = Vec::new();
                        for brick_face_id in brick_faces {
                            let face = fabric.face(brick_face_id);
                            brick_face_midpoints.push((
                                brick_face_id,
                                face.midpoint(fabric),
                                face.middle_joint(fabric),
                            ));
                        }
                        let mut far_face_midpoints = Vec::new();
                        for face_id in face_ids {
                            let face = fabric.face(face_id);
                            far_face_midpoints.push((
                                face_id,
                                face.midpoint(fabric),
                                face.middle_joint(fabric),
                            ));
                        }
                        let shapers = far_face_midpoints.into_iter().map(
                            |(far_face_id, far_face_midpoint, far_joint)| {
                                let brick_face = brick_face_midpoints.iter().min_by(
                                    |(_, location_a, _), (_, location_b, _)| {
                                        let (dx, dy) = (
                                            location_a.distance2(far_face_midpoint),
                                            location_b.distance2(far_face_midpoint),
                                        );
                                        if dx < dy {
                                            Ordering::Less
                                        } else if dx > dy {
                                            Ordering::Greater
                                        } else {
                                            Ordering::Equal
                                        }
                                    },
                                );
                                let (near_face_id, _, near_joint) =
                                    *brick_face.expect("Expected a closest face");
                                (near_face_id, near_joint, far_face_id, far_joint)
                            },
                        );
                        for (near_face_id, near_joint, far_face_id, far_joint) in shapers {
                            let interval =
                                fabric.create_interval(near_joint, far_joint, 0.01, PullMaterial);
                            self.shape_intervals.push(ShapeInterval::FaceJoiner {
                                interval,
                                alpha_face: near_face_id,
                                omega_face: far_face_id,
                                mark_name: mark_name.clone(),
                            })
                        }
                    }
                    _ => unimplemented!("Join can only be 2 or three faces"),
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
                fabric.centralize(Some(0.0));
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
                        let length = fabric.joints[alpha_index]
                            .location
                            .distance(fabric.joints[omega_index].location)
                            * distance_factor;
                        let interval =
                            fabric.create_interval(alpha_index, omega_index, length, PullMaterial);
                        self.shape_intervals.push(ShapeInterval::FaceSpacer {
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
                self.remove_spacers(fabric, mark_names);
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
                self.remove_spacers(fabric, vec![]);
                let face_ids: Vec<UniqueId> = fabric.faces.keys().cloned().collect();
                for face_id in face_ids {
                    fabric.face_to_triangle(face_id);
                    fabric.remove_face(face_id);
                }
                Noop
            }
            ShapeOperation::FacesToPrisms { mark_names } => {
                if mark_names.is_empty() {
                    let face_ids: Vec<UniqueId> = fabric.faces.keys().cloned().collect();
                    for face_id in face_ids {
                        fabric.face_to_prism(face_id);
                        fabric.remove_face(face_id);
                    }
                } else {
                    for mark_name in mark_names {
                        self.marks
                            .iter()
                            .filter(|mark| mark.mark_name == mark_name)
                            .sorted_by(|&mark_a, &mark_b| {
                                Ord::cmp(&mark_a.face_id, &mark_b.face_id)
                            })
                            .for_each(|&FaceMark { face_id, .. }| {
                                fabric.face_to_prism(face_id);
                                fabric.remove_face(face_id);
                            });
                    }
                }
                StartCountdown(DEFAULT_PRISM_COUNTDOWN)
            }
            ShapeOperation::SetStiffness(percent) => Stiffness(percent),
            ShapeOperation::SetDrag(percent) => Drag(percent),
            ShapeOperation::SetViscosity(percent) => Viscosity(percent),
            ShapeOperation::Anchor {
                joint_index,
                surface,
            } => {
                let (x, z) = surface;
                let base = fabric.create_fixed_joint(Point3::new(x, 0.0, z));
                let interval = fabric.create_interval(joint_index, base, 0.01, PullMaterial);
                self.shape_intervals
                    .push(ShapeInterval::SurfaceAnchor { interval });
                StartCountdown(DEFAULT_ADD_SHAPER_COUNTDOWN)
            }
            ShapeOperation::GuyLine {
                joint_index,
                length,
                surface,
            } => {
                let (x, z) = surface;
                let base = fabric.create_fixed_joint(Point3::new(x, 0.0, z));
                fabric.create_interval(joint_index, base, length, GuyWireMaterial);
                StartCountdown(DEFAULT_ADD_SHAPER_COUNTDOWN)
            }
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

    fn remove_spacers(&mut self, fabric: &mut Fabric, mark_names: Vec<String>) {
        self.shape_intervals = self
            .shape_intervals
            .iter()
            .cloned()
            .filter(|shape_interval| {
                if let ShapeInterval::FaceSpacer {
                    interval,
                    mark_name,
                    ..
                } = shape_interval
                {
                    let marked = mark_names.is_empty() || mark_names.contains(&mark_name);
                    if marked {
                        fabric.remove_interval(*interval);
                    }
                    !marked // discard if marked
                } else {
                    true
                }
            })
            .collect();
    }

    fn remove_anchors(&mut self, fabric: &mut Fabric) {
        self.shape_intervals = self
            .shape_intervals
            .iter()
            .cloned()
            .filter(|shape_interval| {
                if let ShapeInterval::SurfaceAnchor { interval } = shape_interval {
                    let omega_id = fabric.interval(*interval).omega_index;
                    fabric.remove_interval(*interval);
                    fabric.remove_joint(omega_id);
                    false // discard
                } else {
                    true
                }
            })
            .collect();
    }
}
