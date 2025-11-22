/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */
use cgmath::{EuclideanSpace, InnerSpace, Matrix3, Matrix4, MetricSpace, Point3, Vector3};

use crate::build::tenscript::{FaceAlias, Spin};
use crate::fabric::interval::{Interval, Role};
use crate::fabric::joint::Joint;
use crate::fabric::{Fabric, UniqueId};

const ROOT3: f32 = 1.732_050_8;

impl Fabric {
    pub fn create_face(
        &mut self,
        aliases: Vec<FaceAlias>,
        scale: f32,
        spin: Spin,
        radial_intervals: [UniqueId; 3],
    ) -> UniqueId {
        let id = self.create_id();
        self.faces.insert(
            id,
            Face {
                aliases,
                scale,
                spin,
                radial_intervals,
                has_prism: false,
            },
        );
        id
    }

    pub fn face(&self, id: UniqueId) -> &Face {
        self.faces
            .get(&id)
            .unwrap_or_else(|| panic!("face not found {id:?}"))
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

    pub fn join_faces(&mut self, alpha_id: UniqueId, omega_id: UniqueId) {
        let (alpha, omega) = (self.face(alpha_id), self.face(omega_id));
        let (mut alpha_ends, omega_ends) = (alpha.radial_joints(self), omega.radial_joints(self));
        if alpha.spin == omega.spin {
            alpha_ends.reverse();
        }
        let (mut alpha_points, omega_points) = (
            alpha_ends.map(|id| self.location(id)),
            omega_ends.map(|id| self.location(id)),
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
        self.remove_face(alpha_id);
        self.remove_face(omega_id);
    }

    pub fn add_face_triangle(&mut self, face_id: UniqueId) {
        let face = self.face(face_id);
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

    pub fn add_face_prism(&mut self, face_id: UniqueId) {
        let face = self.face(face_id);
        let push_length = face.scale * 1.5;
        let radial_joints = face.radial_joints(self);
        let normal = face.normal(&self);
        let midpoint = face.midpoint(&self);
        // Calculate actual distance from face center to radial joints
        let radial_distance = self.joints[radial_joints[0]].location.distance(Point3::from_vec(midpoint));
        // Alpha/omega are at distance `push_length/2` along the normal.
        // By Pythagorean theorem: pull_length² = radial_distance² + (push_length/2)²
        let pull_length =
            (radial_distance * radial_distance + (push_length / 2.0) * (push_length / 2.0)).sqrt();
        
        let alpha = self.create_joint(Point3::from_vec(midpoint - normal * push_length / 2.0));
        let omega = self.create_joint(Point3::from_vec(midpoint + normal * push_length / 2.0));
        
        self.create_interval(alpha, omega, push_length, Role::Pushing);
        
        // Connect prism push joints to radials
        for joint in 0..3 {
            let radial = radial_joints[joint];
            self.create_interval(alpha, radial, pull_length, Role::PrismPull);
            self.create_interval(omega, radial, pull_length, Role::PrismPull);
        }
        // Mark the face as having a prism
        if let Some(face) = self.faces.get_mut(&face_id) {
            face.has_prism = true;
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

#[derive(Clone, Debug)]
pub struct Face {
    pub aliases: Vec<FaceAlias>,
    pub scale: f32,
    pub spin: Spin,
    pub radial_intervals: [UniqueId; 3],
    pub has_prism: bool,
}

impl Face {
    pub fn alias(&self) -> &FaceAlias {
        assert_eq!(self.aliases.len(), 1);
        &self.aliases[0]
    }

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
            .map(|joint_index| &fabric.joints[joint_index])
            .map(|Joint { location, .. }| location.current())
    }

    pub fn middle_joint(&self, fabric: &Fabric) -> usize {
        fabric.interval(self.radial_intervals[0]).alpha_index
    }

    pub fn radial_joints(&self, fabric: &Fabric) -> [usize; 3] {
        self.radial_intervals
            .map(|id| fabric.interval(id))
            .map(|Interval { omega_index, .. }| *omega_index)
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
