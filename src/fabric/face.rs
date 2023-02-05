/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */
use cgmath::{EuclideanSpace, InnerSpace, Matrix4, Point3, Quaternion, Rotation, Vector3};

use crate::build::tenscript::{FaceName, Spin};
use crate::fabric::{Fabric, UniqueId};
use crate::fabric::interval::Interval;
use crate::fabric::joint::Joint;

#[derive(Clone, Debug)]
pub struct Face {
    pub face_name: FaceName,
    pub scale: f32,
    pub spin: Spin,
    pub radial_intervals: [UniqueId; 3],
}

impl Face {
    pub fn midpoint(&self, fabric: &Fabric) -> Vector3<f32> {
        let loc = self.radial_joint_locations(fabric);
        (loc[0].to_vec() + loc[1].to_vec() + loc[2].to_vec()) / 3.0
    }

    pub fn normal(&self, fabric: &Fabric) -> Vector3<f32> {
        let loc = self.radial_joint_locations(fabric);
        let v1 = loc[1] - loc[0];
        let v2 = loc[2] - loc[0];
        v1.cross(v2).normalize()
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

    pub fn space(&self, fabric: &Fabric) -> Matrix4<f32> {
        let midpoint = self.midpoint(fabric);
        let [radial0, radial1, _] = self.radial_joint_locations(fabric);
        let radial_x = radial0.to_vec() + radial1.to_vec() - midpoint * 2.0;
        Matrix4::from_translation(midpoint) *
            Matrix4::from_scale(self.scale) *
            Matrix4::from(Quaternion::between_vectors(Vector3::unit_y(), -self.normal(fabric))) *
            Matrix4::from(Quaternion::between_vectors(Vector3::unit_x(), radial_x))
    }
}
