use crate::build::tenscript::brick_library::BrickLibrary;
use crate::build::tenscript::shape_phase::ShapeCommand::*;
use crate::build::tenscript::{FaceAlias, PairExt, PairsExt, Rule, Spin};
use crate::build::tenscript::{FaceMark, TenscriptError};
use crate::fabric::brick::BaseFace;
use crate::fabric::face::{vector_space, FaceRotation};
use crate::fabric::interval::Role;
use crate::fabric::{Fabric, UniqueId};
use crate::units::Seconds;
use cgmath::{EuclideanSpace, InnerSpace, Matrix4, MetricSpace, Point3, Quaternion, Vector3};
use pest::iterators::Pair;
use std::cmp::Ordering;

const DEFAULT_ADD_SHAPER_COUNTDOWN: Seconds = Seconds(25.0);
const DEFAULT_VULCANIZE_COUNTDOWN: Seconds = Seconds(5.0);
const DEFAULT_JOINER_COUNTDOWN: Seconds = Seconds(30.0);

#[derive(Debug)]
pub enum ShapeCommand {
    Noop,
    StartProgress(Seconds),
    Rigidity(f32),
    Drag(f32),
    Viscosity(f32),
    Terminate,
}

#[derive(Debug, Clone)]
pub enum ShapeOperation {
    During {
        seconds: Seconds,
        operations: Vec<ShapeOperation>,
    },
    Joiner {
        mark_name: String,
        seed: Option<usize>,
    },
    PointDownwards {
        mark_name: String,
    },
    Centralize {
        altitude: Option<f32>,
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
        role: Option<Role>,
    },
    Vulcanize,
    SetRigidity(f32),
    SetDrag(f32),
    SetViscosity(f32),
    Omit((usize, usize)),
    Add {
        alpha_index: usize,
        omega_index: usize,
        length_factor: f32,
    },
}

impl ShapeOperation {
    pub fn traverse(&self, f: &mut impl FnMut(&Self)) {
        f(self);
        if let ShapeOperation::During { operations, .. } = self {
            for operation in operations {
                operation.traverse(f);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Joiner {
    interval: UniqueId,
    alpha_face: UniqueId,
    omega_face: UniqueId,
}

#[derive(Debug, Clone)]
pub struct ShapePhase {
    pub operations: Vec<ShapeOperation>,
    pub marks: Vec<FaceMark>,
    pub spacers: Vec<UniqueId>,
    pub joiners: Vec<Joiner>,
    pub anchors: Vec<UniqueId>,
    shape_operation_index: usize,
}

impl ShapePhase {
    pub fn from_pair(pair: Pair<Rule>) -> Result<ShapePhase, TenscriptError> {
        let operations = Self::parse_shape_operations(pair.into_inner())?;
        Ok(ShapePhase {
            operations,
            marks: Vec::new(),
            spacers: Vec::new(),
            joiners: Vec::new(),
            anchors: Vec::new(),
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
                let mark_name = inner.next_atom();
                let distance_factor = inner.next_float("(space ..)")?;
                Ok(ShapeOperation::Spacer {
                    mark_name,
                    distance_factor,
                })
            }
            Rule::joiner => {
                let mut inner = pair.into_inner();
                let mark_name = inner.next_atom();
                let seed = inner.next()
                    .map(|seed_pair| seed_pair.parse_usize_inner("(seed ...)"))
                    .transpose()?;
                Ok(ShapeOperation::Joiner {
                    mark_name,
                    seed,
                })
            }
            Rule::down => {
                let mark_name = pair.into_inner().next_atom();
                Ok(ShapeOperation::PointDownwards {
                    mark_name,
                })
            }
            Rule::centralize => {
                let altitude = pair.into_inner().next()
                    .map(|p| p.parse_float_str("(centralize)"))
                    .transpose()?;
                Ok(ShapeOperation::Centralize { altitude })
            }
            Rule::during => {
                let mut inner = pair.into_inner();
                let seconds = inner.next_float("(during ...)")?;
                let operations = Self::parse_shape_operations(inner)?;
                Ok(ShapeOperation::During {
                    seconds: Seconds(seconds),
                    operations,
                })
            }
            Rule::vulcanize => Ok(ShapeOperation::Vulcanize),
            Rule::set_rigidity => {
                let percent = pair.parse_float_inner("(set-rigidity ..)")?;
                Ok(ShapeOperation::SetRigidity(percent))
            }
            Rule::set_drag => {
                let percent = pair.parse_float_inner("(set-drag ..)")?;
                Ok(ShapeOperation::SetDrag(percent))
            }
            Rule::set_viscosity => {
                let percent = pair.parse_float_inner("(set-viscosity ..)")?;
                Ok(ShapeOperation::SetViscosity(percent))
            }
            Rule::omit => {
                let mut inner = pair.into_inner();
                let alpha_index = inner.next_usize("(omit ...)")?;
                let omega_index = inner.next_usize("(omit ...)")?;
                Ok(ShapeOperation::Omit((alpha_index, omega_index)))
            }
            Rule::add => {
                let mut inner = pair.into_inner();
                let alpha_index = inner.next_usize("(add ...)")?;
                let omega_index = inner.next_usize("(add ...)")?;
                let length_factor = inner.next()
                    .map(|p| p.parse_float_str("(add ...)"))
                    .transpose()?
                    .unwrap_or(1.0);
                Ok(ShapeOperation::Add {
                    alpha_index,
                    omega_index,
                    length_factor,
                })
            }
            Rule::anchor => {
                let mut inner = pair.into_inner();
                let joint_index = inner.next_usize("(anchor ...)")?;
                let surface = Self::parse_surface_location(inner.next().unwrap())?;
                Ok(ShapeOperation::Anchor {
                    joint_index,
                    surface,
                })
            }
            Rule::guy_line => {
                let mut inner = pair.into_inner();
                let joint_index = inner.next_usize("(guy-line joint-index ...)")?;
                let length = inner.next_float("(guy-line <> length ...)")?;
                let mut surface = inner.next().unwrap().into_inner();
                let x = surface.next_float("(surface x ..)")?;
                let z = surface.next_float("(surface .. z)")?;
                let role = inner.next()
                    .and_then(|p| Role::from_label(&p.as_atom()));
                Ok(ShapeOperation::GuyLine {
                    joint_index,
                    length,
                    surface: (x, z),
                    role,
                })
            }
            _ => unreachable!("shape phase: {pair}"),
        }
    }

    fn parse_surface_location(pair: Pair<Rule>) -> Result<(f32, f32), TenscriptError> {
        let mut inner = pair.into_inner();
        let x = inner.next_float("(surface x ..)")?;
        let z = inner.next_float("(surface .. z)")?;
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
            self.cleanup(fabric);
            return Ok(Terminate);
        };
        self.shape_operation_index += 1;
        self.execute_shape_operation(fabric, brick_library, operation.clone())
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
                            fabric.create_interval(joints[0], joints[1], 0.01, Role::Pulling);
                        self.joiners.push(Joiner {
                            interval,
                            alpha_face: face_ids[0],
                            omega_face: face_ids[1],
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
                                fabric.create_interval(near_joint, far_joint, 0.01, Role::Pulling);
                            self.joiners.push(Joiner {
                                interval,
                                alpha_face: near_face_id,
                                omega_face: far_face_id,
                            })
                        }
                    }
                    _ => unimplemented!("Join can only be 2 or three faces"),
                }
                StartProgress(DEFAULT_ADD_SHAPER_COUNTDOWN)
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
                        let interval = fabric.create_interval(
                            alpha_index,
                            omega_index,
                            length,
                            Role::Pulling,
                        );
                        self.spacers.push(interval);
                    }
                }
                StartProgress(DEFAULT_ADD_SHAPER_COUNTDOWN)
            }
            ShapeOperation::During {
                seconds,
                operations,
            } => {
                for operation in operations {
                    // ignores the countdown returned from each sub-operation
                    let _ = self.execute_shape_operation(fabric, brick_library, operation);
                }
                StartProgress(seconds)
            }
            ShapeOperation::Vulcanize => {
                fabric.install_bow_ties();
                StartProgress(DEFAULT_VULCANIZE_COUNTDOWN)
            }
            ShapeOperation::SetRigidity(percent) => Rigidity(percent),
            ShapeOperation::SetDrag(percent) => Drag(percent),
            ShapeOperation::SetViscosity(percent) => Viscosity(percent),
            ShapeOperation::Omit(pair) => {
                fabric.joining(pair).map(|id| fabric.remove_interval(id));
                Noop
            }
            ShapeOperation::Add {
                alpha_index,
                omega_index,
                length_factor,
            } => {
                let ideal = fabric.distance(alpha_index, omega_index) * length_factor;
                fabric.create_interval(alpha_index, omega_index, ideal, Role::Pulling);
                Noop
            }
            ShapeOperation::Anchor {
                joint_index,
                surface,
            } => {
                let (x, z) = surface;
                let base = fabric.create_joint(Point3::new(x, 0.0, z));
                let interval_id = fabric.create_interval(joint_index, base, 0.01, Role::Support);
                self.anchors.push(interval_id);
                StartProgress(DEFAULT_ADD_SHAPER_COUNTDOWN)
            }
            ShapeOperation::GuyLine {
                joint_index,
                length,
                surface,
                role,
            } => {
                let (x, z) = surface;
                let base = fabric.create_joint(Point3::new(x, 0.0, z));
                let role = role.unwrap_or(Role::Support);
                fabric.create_interval(joint_index, base, length, role);
                StartProgress(DEFAULT_ADD_SHAPER_COUNTDOWN)
            }
            ShapeOperation::Centralize { altitude } => {
                let translation = fabric.centralize_translation(altitude);
                fabric.apply_translation(translation);
                Noop
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
    pub fn complete_joiners(&mut self, fabric: &mut Fabric) -> Option<ShapeCommand> {
        let joiner_active = !self.joiners.is_empty();
        for Joiner {
            interval,
            alpha_face,
            omega_face,
            ..
        } in self.joiners.drain(..)
        {
            fabric.remove_interval(interval);
            fabric.join_faces(alpha_face, omega_face);
        }
        joiner_active.then_some(StartProgress(DEFAULT_JOINER_COUNTDOWN))
    }
    fn cleanup(&mut self, fabric: &mut Fabric) {
        for interval in self.spacers.drain(..) {
            fabric.remove_interval(interval);
        }
        for interval in self.anchors.drain(..) {
            fabric.remove_interval(interval);
        }
    }
}
