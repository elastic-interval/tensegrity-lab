use cgmath::{InnerSpace, Matrix4, MetricSpace, Quaternion, Vector3};
use pest::iterators::Pair;

use crate::build::tenscript::shape_phase::ShapeCommand::*;
use crate::build::tenscript::Rule;
use crate::build::tenscript::{FaceMark, TenscriptError};
use crate::fabric::{Fabric, Link, UniqueId};

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
                let mark_name = pair.into_inner().next().unwrap().as_str();
                Ok(ShapeOperation::Joiner {
                    mark_name: mark_name[1..].into(),
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

    pub fn shaping_step(&mut self, fabric: &mut Fabric) -> Result<ShapeCommand, TenscriptError> {
        if let Some(countdown) = self.complete_joiners(fabric) {
            return Ok(countdown);
        }
        let Some(operation) = self.operations.get(self.shape_operation_index) else {
            self.remove_spacers(fabric);
            return Ok(Terminate);
        };
        self.shape_operation_index += 1;
        self.execute_shape_operation(fabric, operation.clone())
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
        operation: ShapeOperation,
    ) -> Result<ShapeCommand, TenscriptError> {
        Ok(match operation {
            ShapeOperation::Joiner { mark_name } => {
                let faces = self.marked_faces(&mark_name);
                let joints = self.marked_middle_joints(fabric, &faces);
                match (joints.as_slice(), faces.as_slice()) {
                    (&[alpha_index, omega_index], &[alpha_face, omega_face]) => {
                        let interval =
                            fabric.create_interval(alpha_index, omega_index, Link::pull(0.01));
                        self.joiners.push(Shaper {
                            interval,
                            alpha_face,
                            omega_face,
                            mark_name,
                        })
                    }
                    _ => unimplemented!(),
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
                            .joints[alpha]
                            .location
                            .distance(fabric.joints[omega].location)
                            * distance_factor;
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
                    let _ = self.execute_shape_operation(fabric, operation);
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
