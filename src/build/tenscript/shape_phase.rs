use cgmath::MetricSpace;
use crate::build::tenscript::FaceMark;
use crate::build::tenscript::shape_phase::ShapeCommand::{*};
use crate::fabric::{Fabric, Link, UniqueId};

const DEFAULT_ADD_SHAPER_COUNTDOWN: usize = 20_000;
const DEFAULT_VULCANIZE_COUNTDOWN: usize = 5_000;

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
    Join { mark_name: String },
    Distance { mark_name: String, distance_factor: f32 },
    RemoveShapers { mark_names: Vec<String> },
    Vulcanize,
    ReplaceFaces,
    SetViscosity { viscosity: f32 },
}

#[derive(Debug)]
pub struct Shaper {
    interval: UniqueId,
    alpha_face: UniqueId,
    omega_face: UniqueId,
    mark_name: String,
    join: bool,
}

#[derive(Debug, Default)]
pub struct ShapePhase {
    pub operations: Vec<ShapeOperation>,
    pub marks: Vec<FaceMark>,
    pub shapers: Vec<Shaper>,
    shape_operation_index: usize,
}

impl ShapePhase {
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