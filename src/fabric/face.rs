/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */
use cgmath::{EuclideanSpace, InnerSpace, Matrix3, Matrix4, Point3, Vector3};

use crate::build::tenscript::{FaceAlias, Spin};
use crate::fabric::interval::Interval;
use crate::fabric::joint::Joint;
use crate::fabric::{Fabric, UniqueId};

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
        let midpoint = self.midpoint(fabric);
        let radial = self.radial_joint_locations(fabric);
        let (a, b) = match rotation {
            FaceRotation::Zero => (radial[0], radial[1]),
            FaceRotation::OneThird => (radial[1], radial[2]),
            FaceRotation::TwoThirds => (radial[2], radial[0]),
        };
        let (x_axis, y_axis, scale) = (
            (a.to_vec() + b.to_vec() - midpoint * 2.0).normalize(),
            self.normal(fabric),
            self.scale,
        );
        let z_axis = x_axis.cross(y_axis).normalize();
        Matrix4::from_translation(midpoint)
            * Matrix4::from(Matrix3::from_cols(x_axis, y_axis, z_axis))
            * Matrix4::from_scale(scale)
    }
}
