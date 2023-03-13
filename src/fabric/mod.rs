/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use std::cmp::Ordering;
use std::collections::HashMap;

use cgmath::{EuclideanSpace, Matrix4, MetricSpace, Point3, Transform, Vector3};
use cgmath::num_traits::zero;

use crate::build::tenscript::{FaceAlias, Spin};
use crate::build::tenscript::brick::{Baked, BakedInterval, BrickFace};
use crate::build::tenscript::brick_library::BrickLibrary;
use crate::fabric::face::{Face, FaceRotation};
use crate::fabric::interval::{Interval, Material};
use crate::fabric::interval::Role::{Pull, Push};
use crate::fabric::interval::Span::{Approaching, Fixed};
use crate::fabric::joint::Joint;
use crate::fabric::physics::Physics;
use crate::fabric::progress::Progress;

pub mod face;
pub mod interval;
pub mod joint;
pub mod physics;
pub mod progress;
pub mod vulcanize;
pub mod lab;

#[derive(Clone, Debug)]
pub struct Fabric {
    pub age: u64,
    pub progress: Progress,
    pub muscle_nuance: f32,
    pub joints: Vec<Joint>,
    pub intervals: HashMap<UniqueId, Interval>,
    pub faces: HashMap<UniqueId, Face>,
    pub rings: Vec<[UniqueId; 6]>,
    pub bricks: Vec<Vec<UniqueId>>,
    pub materials: Vec<Material>,
    unique_id: usize,
}

impl Default for Fabric {
    fn default() -> Fabric {
        Fabric {
            age: 0,
            progress: Progress::default(),
            muscle_nuance: 0.5,
            joints: Vec::new(),
            intervals: HashMap::new(),
            faces: HashMap::new(),
            rings: Vec::new(),
            bricks: Vec::new(),
            materials: MATERIALS.into(),
            unique_id: 0,
        }
    }
}

impl Fabric {
    pub fn material(&self, sought_name: String) -> usize {
        self.materials
            .iter()
            .position(|&Material { name, .. }| name == sought_name)
            .unwrap_or_else(|| panic!("missing material {sought_name}"))
    }

    pub fn create_joint(&mut self, point: Point3<f32>) -> usize {
        let index = self.joints.len();
        self.joints.push(Joint::new(point));
        index
    }

    pub fn location(&self, index: usize) -> Point3<f32> {
        self.joints[index].location
    }

    pub fn remove_joint(&mut self, index: usize) {
        self.joints.remove(index);
        self.intervals.values_mut().for_each(|interval| interval.joint_removed(index));
    }

    pub fn distance(&self, alpha_index: usize, omega_index: usize) -> f32 {
        self.location(alpha_index).distance(self.location(omega_index))
    }

    pub fn ideal(&self, alpha_index: usize, omega_index: usize, strain: f32) -> f32 {
        let distance = self.distance(alpha_index, omega_index);
        distance / (1.0 + strain)
    }

    pub fn create_interval(&mut self, alpha_index: usize, omega_index: usize, Link { ideal, material_name }: Link) -> UniqueId {
        let id = self.create_id();
        let initial = self.joints[alpha_index].location.distance(self.joints[omega_index].location);
        let material = self.material(material_name);
        let interval = Interval::new(alpha_index, omega_index, material, Approaching { initial, length: ideal });
        self.intervals.insert(id, interval);
        id
    }

    pub fn interval(&self, id: UniqueId) -> &Interval {
        self.intervals.get(&id).unwrap()
    }

    pub fn remove_interval(&mut self, id: UniqueId) {
        self.intervals.remove(&id);
    }

    pub fn interval_values(&self) -> impl Iterator<Item=&Interval> {
        self.intervals.values()
    }

    pub fn create_face(&mut self, aliases: Vec<FaceAlias>, scale: f32, spin: Spin, radial_intervals: [UniqueId; 3]) -> UniqueId {
        let id = self.create_id();
        self.faces.insert(id, Face { aliases, scale, spin, radial_intervals });
        id
    }

    pub fn face(&self, id: UniqueId) -> &Face {
        self.faces.get(&id).unwrap_or_else(|| panic!("face not found {id:?}"))
    }

    pub fn remove_face(&mut self, id: UniqueId) {
        let face = self.face(id);
        let middle_joint = face.middle_joint(self);
        for interval_id in face.radial_intervals {
            self.remove_interval(interval_id);
        }
        self.remove_joint(middle_joint);
        self.faces.remove(&id);
    }

    pub fn create_brick(
        &mut self,
        face_alias: &FaceAlias,
        rotation: FaceRotation,
        scale_factor: f32,
        face_id: Option<UniqueId>,
        brick_library: &BrickLibrary,
    ) -> (UniqueId, Vec<UniqueId>) {
        let face = face_id.map(|id| self.face(id));
        let scale = face.map(|Face { scale, .. }| *scale).unwrap_or(1.0) * scale_factor;
        let spin_alias = face_alias
            .spin()
            .or(face.map(|face| face.spin.opposite()))
            .map(Spin::into_alias);
        let search_alias = match spin_alias {
            None => face_alias.with_seed(),
            Some(spin_alias) => spin_alias + face_alias,
        };
        let brick = brick_library.new_brick(&search_alias);
        let matrix = face.map(|face| face.vector_space(self, rotation));
        let joints: Vec<usize> = brick.joints
            .into_iter()
            .map(|point| self.create_joint(match matrix {
                None => point,
                Some(matrix) => matrix.transform_point(point),
            }))
            .collect();
        let brick_intervals = brick.intervals
            .into_iter()
            .map(|BakedInterval { alpha_index, omega_index, material_name, strain } |{
                let (alpha_index, omega_index) = (joints[alpha_index], joints[omega_index]);
                let ideal = self.ideal(alpha_index, omega_index, strain);
                self.create_interval(alpha_index, omega_index, Link { ideal, material_name })
            })
            .collect();
        self.bricks.push(brick_intervals);
        let brick_faces = brick.faces
            .into_iter()
            .map(|BrickFace { joints: brick_joints, aliases, spin }| {
                let midpoint = brick_joints
                    .map(|index| self.joints[joints[index]].location.to_vec())
                    .into_iter()
                    .sum::<Vector3<f32>>() / 3.0;
                let alpha_index = self.create_joint(Point3::from_vec(midpoint));
                let radial_intervals = brick_joints.map(|omega| {
                    let omega_index = joints[omega];
                    let ideal = self.ideal(alpha_index, omega_index, Baked::TARGET_FACE_STRAIN);
                    self.create_interval(alpha_index, omega_index, Link::pull(ideal))
                });
                let single_alias: Vec<_> = aliases
                    .into_iter()
                    .filter(|alias| search_alias.matches(alias))
                    .collect();
                assert_eq!(single_alias.len(), 1, "filter must leave exactly one face alias");
                self.create_face(single_alias, scale, spin, radial_intervals)
            })
            .collect::<Vec<_>>();
        let search_base = search_alias.with_base();
        let base_face = brick_faces
            .iter()
            .find(|&&face_id| search_base.matches(self.face(face_id).alias()))
            .expect("missing face after creating brick");
        (*base_face, brick_faces)
    }

    pub fn join_faces(&mut self, alpha_id: UniqueId, omega_id: UniqueId) {
        let (alpha, omega) = (self.face(alpha_id), self.face(omega_id));
        let (mut alpha_ends, omega_ends) = (alpha.radial_joints(self), omega.radial_joints(self));
        if alpha.spin == omega.spin {
            alpha_ends.reverse();
        }
        let (mut alpha_points, omega_points) = (
            alpha_ends.map(|id| self.location(id)),
            omega_ends.map(|id| self.location(id))
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
        let ring = links.map(|(a, b)|
            self.create_interval(alpha_rotated[a], omega_ends[b], Link::pull(ideal))
        );
        self.rings.push(ring);
        self.remove_face(alpha_id);
        self.remove_face(omega_id);
    }

    pub fn apply_matrix4(&mut self, matrix: Matrix4<f32>) {
        for joint in &mut self.joints {
            joint.location = matrix.transform_point(joint.location);
            joint.velocity = matrix.transform_vector(joint.velocity);
        }
    }

    pub fn centralize(&mut self) {
        let mut midpoint: Vector3<f32> = zero();
        for joint in self.joints.iter() {
            midpoint += joint.location.to_vec();
        }
        midpoint /= self.joints.len() as f32;
        midpoint.y = 0.0;
        for joint in self.joints.iter_mut() {
            joint.location -= midpoint;
        }
    }

    pub fn set_altitude(&mut self, altitude: f32) {
        let Some(low_y) = self.joints
            .iter()
            .map(|joint| joint.location.y)
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal)) else {
            return;
        };
        let up = altitude - low_y;
        if up > 0.0 {
            for joint in &mut self.joints {
                joint.location.y += up;
            }
        }
    }

    pub fn prepare_for_pretensing(&mut self, push_extension: f32) {
        for interval in self.intervals.values_mut() {
            let length = interval.length(&self.joints);
            let Material { role, .. } = self.materials[interval.material];
            interval.span = match role {
                Push => Approaching { initial: length, length: length * push_extension },
                Pull => Fixed { length }
            };
        }
        for joint in self.joints.iter_mut() {
            joint.force = zero();
            joint.velocity = zero();
        }
        self.centralize();
        self.set_altitude(1.0);
    }

    pub fn iterate(&mut self, physics: &Physics) -> f32 {
        for joint in &mut self.joints {
            joint.reset();
        }
        for interval in self.intervals.values_mut() {
            interval.iterate(&mut self.joints, &self.materials, &self.progress, self.muscle_nuance, physics);
        }
        let mut max_speed_squared = 0.0;
        for joint in &mut self.joints {
            let speed_squared = joint.iterate(physics);
            if speed_squared > max_speed_squared {
                max_speed_squared = speed_squared;
            }
        }
        if self.progress.step() { // final step
            for interval in self.intervals.values_mut() {
                if let Approaching { length, .. } = interval.span {
                    interval.span = Fixed { length }
                }
            }
        }
        self.age += 1;
        max_speed_squared
    }

    pub fn midpoint(&self) -> Point3<f32> {
        let mut midpoint: Point3<f32> = Point3::origin();
        for joint in &self.joints {
            midpoint += joint.location.to_vec();
        }
        let denominator = if self.joints.is_empty() { 1 } else { self.joints.len() } as f32;
        midpoint / denominator
    }

    fn create_id(&mut self) -> UniqueId {
        let id = UniqueId(self.unique_id);
        self.unique_id += 1;
        id
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Default, Hash, Eq, Ord, PartialOrd)]
pub struct UniqueId(usize);

const MATERIALS: [Material; 5] = [
    Material {
        name: ":push",
        role: Push,
        stiffness: 3.0,
        mass: 1.0,
    },
    Material {
        name: ":pull",
        role: Pull,
        stiffness: 1.0,
        mass: 0.1,
    },
    Material {
        name: ":bow-tie",
        role: Pull,
        stiffness: 0.7,
        mass: 0.1,
    },
    Material {
        name: ":north",
        role: Pull,
        stiffness: 0.5,
        mass: 0.01,
    },
    Material {
        name: ":south",
        role: Pull,
        stiffness: 0.5,
        mass: 0.01,
    }
];

#[derive(Clone, Debug)]
pub struct Link {
    pub ideal: f32,
    pub material_name: String,
}

impl Link {
    pub fn push(ideal: f32) -> Self {
        Self { ideal, material_name: ":push".to_string() }
    }

    pub fn pull(ideal: f32) -> Self {
        Self { ideal, material_name: ":pull".to_string() }
    }
}

