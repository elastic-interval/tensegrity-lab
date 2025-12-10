use crate::build::dsl::brick_dsl::BrickName::OmniSymmetrical;
use crate::build::dsl::brick_dsl::BrickRole::{OnSpinLeft, OnSpinRight};
use crate::build::dsl::brick_dsl::MarkName;
use crate::build::dsl::shape_phase::ShapeCommand::*;
use crate::build::dsl::{brick_library, FaceMark, Spin};
use crate::fabric::brick::BaseFace;
use crate::fabric::face::{vector_space, FaceRotation};
use crate::fabric::interval::Role;
use crate::fabric::{Fabric, UniqueId};
use crate::units::{Percent, Seconds};
use cgmath::{EuclideanSpace, InnerSpace, Matrix4, MetricSpace, Point3, Quaternion, Vector3};
use std::cmp::Ordering;

const DEFAULT_ADD_SHAPER_COUNTDOWN: Seconds = Seconds(25.0);
const DEFAULT_VULCANIZE_COUNTDOWN: Seconds = Seconds(5.0);
const DEFAULT_JOINER_COUNTDOWN: Seconds = Seconds(3.0);

#[derive(Debug)]
pub enum ShapeCommand {
    Noop,
    StartProgress(Seconds),
    Rigidity(f32),
    Terminate,
}

#[derive(Debug, Clone)]
pub enum ShapeOperation {
    During {
        seconds: Seconds,
        operations: Vec<ShapeOperation>,
    },
    Joiner {
        mark_name: MarkName,
    },
    PointDownwards {
        mark_name: MarkName,
    },
    Centralize {
        altitude: Option<f32>,
    },
    Spacer {
        mark_name: MarkName,
        distance: Percent,
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
    Vulcanize,
    SetRigidity(f32),
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
    pub(crate) shape_operation_index: usize,
}

impl ShapePhase {
    pub fn needs_shaping(&self) -> bool {
        !self.operations.is_empty()
    }

    pub fn shaping_step(&mut self, fabric: &mut Fabric) -> ShapeCommand {
        if let Some(countdown) = self.complete_joiners(fabric) {
            return countdown;
        }
        let Some(operation) = self.operations.get(self.shape_operation_index) else {
            self.cleanup(fabric);
            return Terminate;
        };
        self.shape_operation_index += 1;
        self.execute_shape_operation(fabric, operation.clone())
    }

    fn execute_shape_operation(
        &mut self,
        fabric: &mut Fabric,
        operation: ShapeOperation,
    ) -> ShapeCommand {
        match operation {
            ShapeOperation::Joiner { mark_name } => {
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
                        let base_face = BaseFace::Situated { spin, vector_space };
                        let brick_role = match spin {
                            Spin::Left => OnSpinLeft,
                            Spin::Right => OnSpinRight,
                        };
                        let brick = brick_library::get_brick(OmniSymmetrical, brick_role);
                        let (_, brick_faces) = fabric.attach_brick(
                            &brick,
                            brick_role,
                            FaceRotation::Zero,
                            scale,
                            base_face,
                        );
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
                let faces: Vec<_> = self
                    .marked_faces(&mark_name)
                    .into_iter()
                    .map(|id| fabric.expect_face(id))
                    .collect();
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
                distance,
            } => {
                let faces = self.marked_faces(&mark_name);
                let joints = self.marked_middle_joints(fabric, &faces);
                for alpha in 0..faces.len() - 1 {
                    for omega in (alpha + 1)..faces.len() {
                        let alpha_index = joints[alpha];
                        let omega_index = joints[omega];
                        let alpha_pt = fabric.joints[alpha_index].location;
                        let omega_pt = fabric.joints[omega_index].location;
                        let length = alpha_pt.distance(omega_pt) * distance.as_factor();
                        let interval =
                            fabric.create_interval(alpha_index, omega_index, length, Role::Pulling);
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
                    let _ = self.execute_shape_operation(fabric, operation);
                }
                StartProgress(seconds)
            }
            ShapeOperation::Vulcanize => {
                fabric.install_bow_ties();
                StartProgress(DEFAULT_VULCANIZE_COUNTDOWN)
            }
            ShapeOperation::SetRigidity(percent) => Rigidity(percent),
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
            } => {
                let (x, z) = surface;
                let base = fabric.create_joint(Point3::new(x, 0.0, z));
                fabric.create_interval(joint_index, base, length, Role::Support);
                StartProgress(DEFAULT_ADD_SHAPER_COUNTDOWN)
            }
            ShapeOperation::Centralize { altitude } => {
                let translation = fabric.centralize_translation(altitude);
                fabric.apply_translation(translation);
                Noop
            }
        }
    }

    fn marked_faces(&self, mark_name: &MarkName) -> Vec<UniqueId> {
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
