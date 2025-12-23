/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */
use cgmath::{EuclideanSpace, InnerSpace, Matrix3, Matrix4, MetricSpace, Point3, Vector3};

use crate::build::dsl::{FaceAlias, Spin};
use crate::fabric::interval::Role;
use crate::fabric::{Fabric, FaceKey, IntervalKey, JointKey};

const ROOT3: f32 = 1.732_050_8;

impl Fabric {
    pub fn create_face(
        &mut self,
        aliases: Vec<FaceAlias>,
        scale: f32,
        spin: Spin,
        radial_intervals: [IntervalKey; 3],
    ) -> FaceKey {
        self.faces.insert(Face {
            aliases,
            scale,
            spin,
            radial_intervals,
            ending: FaceEnding::default(),
        })
    }

    pub fn face(&self, id: FaceKey) -> &Face {
        self.faces
            .get(id)
            .unwrap_or_else(|| panic!("face not found {id:?}"))
    }

    pub fn remove_face(&mut self, id: FaceKey) {
        let face = self.face(id);
        let middle_joint = face.middle_joint(self);
        let is_radial = face.ending == FaceEnding::Radial;
        let radial_intervals = face.radial_intervals;

        if is_radial {
            // For radial faces, convert radials to Pulling instead of removing
            for interval_key in radial_intervals {
                if let Some(interval) = self.intervals.get_mut(interval_key) {
                    interval.role = Role::Pulling;
                }
            }
        } else {
            // Normal face removal: delete radials
            for interval_key in radial_intervals {
                self.remove_interval(interval_key);
            }
            self.remove_joint(middle_joint);
        }
        self.faces.remove(id);
    }

    pub fn join_faces(&mut self, alpha_key: FaceKey, omega_key: FaceKey) {
        let (alpha, omega) = (self.face(alpha_key), self.face(omega_key));
        let (mut alpha_ends, omega_ends) = (alpha.radial_joints(self), omega.radial_joints(self));
        if alpha.spin == omega.spin {
            alpha_ends.reverse();
        }
        let (mut alpha_points, omega_points) = (
            alpha_ends.map(|idx| self.location(idx)),
            omega_ends.map(|idx| self.location(idx)),
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
            self.create_interval(alpha_rotated[a], omega_ends[b], ideal, Role::Circumference);
        }
        self.remove_face(alpha_key);
        self.remove_face(omega_key);
    }

    pub fn add_face_triangle(&mut self, face_key: FaceKey) {
        let face = self.face(face_key);
        let side_length = face.scale * ROOT3;
        let radial_joints = face.radial_joints(self);
        for (alpha, omega) in [(0, 1), (1, 2), (2, 0)] {
            self.create_interval(
                radial_joints[alpha],
                radial_joints[omega],
                side_length,
                Role::Pulling,
            );
        }
    }

    pub fn add_face_prism(&mut self, face_key: FaceKey) {
        let face = self.face(face_key);
        let push_length = face.scale * 1.5;
        let radial_joints = face.radial_joints(self);
        let normal = face.normal(&self);
        let midpoint = face.midpoint(&self);
        let middle_joint_key = face.middle_joint(self);
        // Calculate actual distance from face center to radial joints
        let radial_distance = self.joints[radial_joints[0]]
            .location
            .distance(Point3::from_vec(midpoint));
        // Alpha/omega are at distance `push_length/2` along the normal.
        // By Pythagorean theorem: pull_length² = radial_distance² + (push_length/2)²
        let pull_length =
            (radial_distance * radial_distance + (push_length / 2.0) * (push_length / 2.0)).sqrt();

        // Prism joints extend the middle joint's path with a prism branch (P=15)
        // local_index 0 = prism alpha (below), 1 = prism omega (above)
        // e.g., if middle is "AA6", prism alpha is "AAP0", prism omega is "AAP1"
        let middle_path = &self.joints[middle_joint_key].path;
        let alpha_path = middle_path.extend(15).with_local_index(0);
        let omega_path = middle_path.extend(15).with_local_index(1);

        let alpha = self.create_joint_with_path(
            Point3::from_vec(midpoint - normal * push_length / 2.0),
            alpha_path,
        );
        let omega = self.create_joint_with_path(
            Point3::from_vec(midpoint + normal * push_length / 2.0),
            omega_path,
        );

        self.create_interval(alpha, omega, push_length, Role::Pushing);

        // Connect prism push joints to radials
        for radial in radial_joints {
            self.create_interval(alpha, radial, pull_length, Role::PrismPull);
            self.create_interval(omega, radial, pull_length, Role::PrismPull);
        }
        // Mark the face as having a prism
        if let Some(face) = self.faces.get_mut(face_key) {
            face.ending = FaceEnding::Prism;
        }
    }

    /// Mark a face as radial (radials only, no triangle or prism)
    /// The radial intervals will be converted from FaceRadial to Pulling when removed
    pub fn set_face_radial(&mut self, face_key: FaceKey) {
        if let Some(face) = self.faces.get_mut(face_key) {
            face.ending = FaceEnding::Radial;
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub enum FaceRotation {
    Zero,
    OneThird,
    TwoThirds,
}

impl From<&usize> for FaceRotation {
    fn from(value: &usize) -> Self {
        match value % 3 {
            0 => FaceRotation::Zero,
            1 => FaceRotation::OneThird,
            2 => FaceRotation::TwoThirds,
            _ => unreachable!(),
        }
    }
}

/// How a face should be treated when transitioning from build to pretensing phase
#[derive(Clone, Debug, Default, PartialEq)]
pub enum FaceEnding {
    /// Default: add triangle cables between the radial joints
    #[default]
    Triangle,
    /// Add a prism (push strut with cables to radials)
    Prism,
    /// Keep only the radials (converted to Pulling), no triangle or prism
    Radial,
}

#[derive(Clone, Debug)]
pub struct Face {
    pub aliases: Vec<FaceAlias>,
    pub scale: f32,
    pub spin: Spin,
    pub radial_intervals: [IntervalKey; 3],
    pub ending: FaceEnding,
}

impl Face {
    pub fn midpoint(&self, fabric: &Fabric) -> Vector3<f32> {
        let loc = self.radial_joint_locations(fabric);
        (loc[0].to_vec() + loc[1].to_vec() + loc[2].to_vec()) / 3.0
    }

    fn normal_to(&self, fabric: &Fabric, length: f32) -> Vector3<f32> {
        let loc = self.radial_joint_locations(fabric);
        let v1 = loc[1] - loc[0];
        let v2 = loc[2] - loc[0];
        match self.spin {
            Spin::Left => v2.cross(v1),
            Spin::Right => v1.cross(v2),
        }
        .normalize_to(length)
    }

    pub fn normal(&self, fabric: &Fabric) -> Vector3<f32> {
        self.normal_to(fabric, 1.0)
    }

    pub fn visible_points(&self, fabric: &Fabric) -> (Point3<f32>, Point3<f32>, Point3<f32>) {
        let alpha = self.midpoint(fabric);
        let omega = alpha + self.normal_to(fabric, 1.5) * self.scale;
        let middle = (alpha + omega) / 2.0;
        (
            Point3::from_vec(alpha),
            Point3::from_vec(middle),
            Point3::from_vec(omega),
        )
    }

    pub fn radial_joint_locations(&self, fabric: &Fabric) -> [Point3<f32>; 3] {
        self.radial_joints(fabric)
            .map(|joint_key| fabric.joints[joint_key].location)
    }

    pub fn middle_joint(&self, fabric: &Fabric) -> JointKey {
        fabric.interval(self.radial_intervals[0]).alpha_key
    }

    pub fn radial_joints(&self, fabric: &Fabric) -> [JointKey; 3] {
        self.radial_intervals
            .map(|key| fabric.interval(key))
            .map(|interval| interval.omega_key)
    }

    pub fn strain(&self, fabric: &Fabric) -> f32 {
        self.radial_intervals
            .iter()
            .map(|id| fabric.interval(*id).strain)
            .sum::<f32>()
            / 3.0
    }

    pub fn vector_space(&self, fabric: &Fabric, rotation: FaceRotation) -> Matrix4<f32> {
        vector_space(
            self.radial_joint_locations(fabric),
            self.scale,
            self.spin,
            rotation,
        )
    }
}

pub fn vector_space(
    p: [Point3<f32>; 3],
    scale: f32,
    spin: Spin,
    rotation: FaceRotation,
) -> Matrix4<f32> {
    let midpoint = (p[0].to_vec() + p[1].to_vec() + p[2].to_vec()) / 3.0;
    let (a, b) = match rotation {
        FaceRotation::Zero => (p[0], p[1]),
        FaceRotation::OneThird => (p[1], p[2]),
        FaceRotation::TwoThirds => (p[2], p[0]),
    };
    let v1 = p[1] - p[0];
    let v2 = p[2] - p[0];
    let y_axis = match spin {
        Spin::Left => v2.cross(v1).normalize(),
        Spin::Right => v1.cross(v2).normalize(),
    };
    let x_axis = (a.to_vec() + b.to_vec() - midpoint * 2.0).normalize();
    let z_axis = x_axis.cross(y_axis).normalize();
    Matrix4::from_translation(midpoint)
        * Matrix4::from(Matrix3::from_cols(x_axis, y_axis, z_axis))
        * Matrix4::from_scale(scale)
}
