use cgmath::MetricSpace;
use pest::iterators::Pair;

use crate::build::tenscript::{FaceMark, TenscriptError};
use crate::build::tenscript::Rule;
use crate::build::tenscript::shape_phase::ShapeCommand::{*};
use crate::fabric::{Fabric, Link, UniqueId};

const DEFAULT_ADD_SHAPER_COUNTDOWN: usize = 25_000;
const DEFAULT_VULCANIZE_COUNTDOWN: usize = 5_000;

pub enum ShapeCommand {
    Noop,
    StartCountdown(usize),
    SetViscosity(f32),
    Terminate,
}

#[derive(Debug, Clone)]
pub enum ShapeOperation {
    Countdown { count: usize, operations: Vec<ShapeOperation> },
    Join { mark_name: String },
    Distance { mark_name: String, distance_factor: f32 },
    RemoveShapers { mark_names: Vec<String> },
    Vulcanize,
    ReplaceFaces,
    SetViscosity { viscosity: f32 },
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
    join: bool,
}

#[derive(Debug, Clone)]
pub struct ShapePhase {
    pub operations: Vec<ShapeOperation>,
    pub marks: Vec<FaceMark>,
    pub shapers: Vec<Shaper>,
    shape_operation_index: usize,
}

impl ShapePhase {
    pub fn from_pair(pair: Pair<Rule>) -> Result<ShapePhase, TenscriptError> {
        let operations = Self::parse_shape_operations(pair.into_inner())?;
        Ok(ShapePhase {
            operations,
            marks: Vec::new(),
            shapers: Vec::new(),
            shape_operation_index: 0,
        })
    }

    fn parse_shape_operations(pairs: impl Iterator<Item=Pair<Rule>>) -> Result<Vec<ShapeOperation>, TenscriptError> {
        pairs
            .map(Self::parse_shape_operation)
            .collect()
    }

    fn parse_shape_operation(pair: Pair<Rule>) -> Result<ShapeOperation, TenscriptError> {
        match pair.as_rule() {
            Rule::basic_shape_operation | Rule::shape_operation =>
                Self::parse_shape_operation(pair.into_inner().next().unwrap()),
            Rule::space => {
                let [mark_name, distance_string] = pair.into_inner().next_chunk().unwrap().map(|p| p.as_str());
                let distance_factor = TenscriptError::parse_float(distance_string, "space: distance_factor")?;
                Ok(ShapeOperation::Distance { mark_name: mark_name[1..].into(), distance_factor })
            }
            Rule::join => {
                let mark_name = pair.into_inner().next().unwrap().as_str();
                Ok(ShapeOperation::Join { mark_name: mark_name[1..].into() })
            }
            Rule::countdown_block => {
                let mut inner = pair.into_inner();
                let count = TenscriptError::parse_usize(inner.next().unwrap().as_str(), "countdown_block")?;
                let operations = Self::parse_shape_operations(inner)?;
                Ok(ShapeOperation::Countdown { count, operations })
            }
            Rule::remove_shapers => {
                let mark_names = pair.into_inner().map(|p| p.as_str()[1..].into()).collect();
                Ok(ShapeOperation::RemoveShapers { mark_names })
            }
            Rule::replace_faces => Ok(ShapeOperation::ReplaceFaces),
            Rule::vulcanize => Ok(ShapeOperation::Vulcanize),
            Rule::set_viscosity => {
                let viscosity = TenscriptError::parse_float_inside(pair, "viscosity")?;
                Ok(ShapeOperation::SetViscosity { viscosity })
            }
            _ => unreachable!("shape phase: {pair}")
        }
    }


    pub fn needs_shaping(&self) -> bool {
        !self.operations.is_empty()
    }

    pub fn shaping_step(&mut self, fabric: &mut Fabric) -> ShapeCommand {
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

    fn execute_shape_operation(&mut self, fabric: &mut Fabric, operation: ShapeOperation) -> ShapeCommand {
        match operation {
            ShapeOperation::Join { mark_name } => {
                let faces = self.marked_faces(&mark_name);
                let joints = self.marked_middle_joints(fabric, &faces);
                match (joints.as_slice(), faces.as_slice()) {
                    (&[alpha_index, omega_index], &[alpha_face, omega_face]) => {
                        let interval = fabric.create_interval(alpha_index, omega_index, Link::pull(0.01));
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
                        let index = self.shapers
                            .iter()
                            .enumerate()
                            .find_map(|(index, shaper)| (shaper.mark_name == mark_name).then_some(index))
                            .expect("undefined mark");
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
                self.complete_all_shapers(fabric);
                for face_id in fabric.replace_faces() {
                    fabric.remove_face(face_id);
                }
                Noop
            }
            ShapeOperation::SetViscosity { viscosity } =>
                SetViscosity(viscosity),
        }
    }

    fn complete_shaper(&self, fabric: &mut Fabric, Shaper { interval, alpha_face, omega_face, join, .. }: Shaper) {
        fabric.remove_interval(interval);
        if join {
            fabric.join_faces(alpha_face, omega_face);
        }
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