/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */
use cgmath::{EuclideanSpace, InnerSpace, Matrix3, Matrix4, MetricSpace, Point3, Vector3};

use crate::build::tenscript::{FaceAlias, Spin};
use crate::fabric::interval::Interval;
use crate::fabric::joint::Joint;
use crate::fabric::{Fabric, Link, UniqueId};

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
            self.create_interval(alpha_rotated[a], omega_ends[b], Link::pull(ideal));
        }
        self.remove_face(alpha_id);
        self.remove_face(omega_id);
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
            .map(|joint_index| fabric.joints[joint_index])
            .map(|Joint { location, .. }| location)
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
        vector_space(self.radial_joint_locations(fabric), self.scale, self.spin, rotation)
    }
}

pub fn vector_space(p: [Point3<f32>; 3], scale: f32, spin: Spin, rotation: FaceRotation) -> Matrix4<f32> {
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