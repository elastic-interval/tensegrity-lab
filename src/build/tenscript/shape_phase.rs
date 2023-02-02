use std::default::Default;

use cgmath::MetricSpace;
use pest::iterators::Pair;

use crate::build::tenscript::FaceMark;
use crate::build::tenscript::parser::{ParseError, Rule};
use crate::build::tenscript::shape_phase::Command::{*};
use crate::fabric::{Fabric, Link, UniqueId};

const DEFAULT_ADD_SHAPER_COUNTDOWN: usize = 20_000;
const DEFAULT_VULCANIZE_COUNTDOWN: usize = 5_000;

pub enum Command {
    Noop,
    StartCountdown(usize),
    SetViscosity(f32),
    Terminate,
}

#[derive(Debug, Clone)]
pub enum Operation {
    Countdown {
        count: usize,
        operations: Vec<Operation>,
    },
    Join { mark_name: String },
    Distance { mark_name: String, distance_factor: f32 },
    RemoveShapers { mark_names: Vec<String> },
    Vulcanize,
    ReplaceFaces,
    SetViscosity { viscosity: f32 },
}

#[derive(Debug, Clone)]
pub struct Shaper {
    interval: UniqueId,
    alpha_face: UniqueId,
    omega_face: UniqueId,
    mark_name: String,
    join: bool,
}

#[derive(Debug, Default, Clone)]
pub struct ShapePhase {
    pub operations: Vec<Operation>,
    pub marks: Vec<FaceMark>,
    pub shapers: Vec<Shaper>,
    shape_operation_index: usize,
}

impl ShapePhase {
    pub(crate) fn from_pair(pair: Pair<Rule>) -> Result<ShapePhase, ParseError> {
        let shape_phase = ShapePhase {
            operations: pair
                .into_inner()
                .map(Self::parse_shape_operation)
                .collect(),
            ..ShapePhase::default()
        };
        Ok(shape_phase)
    }

    fn parse_shape_operation(pair: Pair<Rule>) -> Operation {
        match pair.as_rule() {
            Rule::basic_shape_operation | Rule::shape_operation =>
                Self::parse_shape_operation(pair.into_inner().next().unwrap()),
            Rule::space => {
                let [mark_name, distance_string] = pair.into_inner().next_chunk().unwrap().map(|p| p.as_str());
                let distance_factor = distance_string.parse().unwrap();
                Operation::Distance {
                    mark_name: mark_name[1..].into(),
                    distance_factor,
                }
            }
            Rule::join => {
                let mark_name = pair.into_inner().next().unwrap().as_str();
                Operation::Join { mark_name: mark_name[1..].into() }
            }
            Rule::countdown_block => {
                let mut inner = pair.into_inner();
                let count = inner.next().unwrap().as_str().parse().unwrap();
                let operations = inner.map(Self::parse_shape_operation).collect();
                Operation::Countdown { count, operations }
            }
            Rule::remove_shapers => {
                let mark_names = pair.into_inner().map(|p| p.as_str()[1..].into()).collect();
                Operation::RemoveShapers { mark_names }
            }
            Rule::replace_faces =>
                Operation::ReplaceFaces,
            Rule::vulcanize =>
                Operation::Vulcanize,
            Rule::set_viscosity => {
                let viscosity = pair.into_inner().next().unwrap().as_str().parse().unwrap();
                Operation::SetViscosity { viscosity }
            }
            _ => unreachable!("shape phase: {pair}")
        }
    }


    pub fn needs_shaping(&self) -> bool {
        !self.operations.is_empty()
    }

    pub fn shaping_step(&mut self, fabric: &mut Fabric) -> Command {
        let Some(operation) = self.operations.get(self.shape_operation_index) else {
            self.complete_all_shapers(fabric);
            return Terminate;
        };
        self.shape_operation_index += 1;
        self.execute_shape_operation(fabric, operation.clone())
    }

    fn complete_all_shapers(&mut self, fabric: &mut Fabric) {
        for shaper in self.shapers.split_off(0) {
            self.complete_shaper(fabric, shaper);
        }
    }

    fn execute_shape_operation(&mut self, fabric: &mut Fabric, operation: Operation) -> Command {
        match operation {
            Operation::Join { mark_name } => {
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
            Operation::Distance { mark_name, distance_factor } => {
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
            Operation::RemoveShapers { mark_names } => {
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
            Operation::Countdown { count, operations } => {
                for operation in operations {
                    // ignores the countdown returned from each sub-operation
                    self.execute_shape_operation(fabric, operation);
                }
                StartCountdown(count)
            }
            Operation::Vulcanize => {
                fabric.install_bow_ties();
                StartCountdown(DEFAULT_VULCANIZE_COUNTDOWN)
            }
            Operation::ReplaceFaces => {
                fabric.replace_faces();
                Noop
            }
            Operation::SetViscosity { viscosity } =>
                SetViscosity(viscosity),
        }
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
}