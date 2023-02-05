use std::f32::consts::PI;
use cgmath::{EuclideanSpace, InnerSpace, Matrix4, MetricSpace, Point3, Quaternion, Rotation, Vector3};
use crate::build::tenscript::Spin;
use crate::fabric::{Fabric, Link, UniqueId};
use crate::fabric::interval::{Interval, Role};
use crate::fabric::joint::Joint;

#[derive(Debug, Clone)]
pub struct Brick {
    joints: Vec<Point3<f32>>,
    intervals: Vec<(usize, usize, Role)>,
    faces: Vec<([usize; 3], Spin)>,
}

impl From<(Fabric, UniqueId)> for Brick {
    fn from((fabric_template, face_id): (Fabric, UniqueId)) -> Self {
        let mut fabric = fabric_template;
        let face = fabric.face(face_id);
        let midpoint = face.midpoint(&fabric);
        let radial_x = face.radial_joint_locations(&fabric)[0].to_vec();
        let length = midpoint.distance(radial_x);
        fabric.apply_matrix4(
            Matrix4::from_scale(1.0 / length) *
                Matrix4::from(Quaternion::between_vectors(face.normal(&fabric), -Vector3::unit_y())) *
                Matrix4::from(Quaternion::between_vectors(radial_x - midpoint, Vector3::unit_x())) *
                Matrix4::from_translation(-midpoint)
        );
        Self {
            joints: fabric.joints
                .iter()
                .map(|Joint { location, .. }| *location)
                .collect(),
            intervals: fabric.interval_values()
                .map(|Interval { alpha_index, omega_index, material, .. }|
                    (*alpha_index, *omega_index, fabric.materials[*material].role)
                )
                .collect(),
            faces: fabric.faces
                .values()
                .map(|face| (face.radial_joints(&fabric), face.spin))
                .collect(),
        }
    }
}

pub enum BrickName {
    LeftTwist,
    RightTwist,
    LeftOmniTwist,
    RightOmniTwist,
    LeftMitosis,
    RightMitosis,
}

const ROOT3: f32 = 1.732_050_8;
const ROOT6: f32 = 2.449_489_8;

impl Brick {
    pub fn new(name: BrickName) -> Brick {
        match name {
            BrickName::LeftTwist => {
                unimplemented!()
            }
            BrickName::RightTwist => {
                unimplemented!()
            }
            BrickName::LeftOmniTwist => {
                unimplemented!()
            }
            BrickName::RightOmniTwist => {
                unimplemented!()
            }
            BrickName::LeftMitosis => {
                unimplemented!()
            }
            BrickName::RightMitosis => {
                unimplemented!()
            }
        }
    }

    pub fn prototype(name: BrickName) -> Fabric {
        let mut fabric = Fabric::default();
        let bot = [0, 1, 2].map(|index| {
            let angle = index as f32 * PI * 2.0 / 3.0;
            Point3::from([angle.cos(), 0.0, angle.sin()])
        });
        match name {
            BrickName::LeftTwist => {
                let top = bot.map(|point| point + Vector3::unit_y());
                let alpha_joints = bot.map(|point|fabric.create_joint(point));
                let omega_joints = top.map(|point|fabric.create_joint(point));
                for (&alpha_index, &omega_index) in alpha_joints.iter().zip(omega_joints.iter()) {
                    fabric.create_interval(alpha_index, omega_index, Link::push(ROOT6));
                }
                let alpha_midpoint = fabric.create_joint(middle(bot));
                for outer in alpha_joints {
                    fabric.create_interval(alpha_midpoint, outer, Link::pull(1.0));
                }
                let omega_midpoint = fabric.create_joint(middle(top));
                for outer in omega_joints {
                    fabric.create_interval(omega_midpoint, outer, Link::pull(1.0));
                }
                let advanced_omega = omega_joints.iter().cycle().skip(1).take(3);
                for (&alpha_index, &omega_index) in alpha_joints.iter().zip(advanced_omega) {
                    fabric.create_interval(alpha_index, omega_index, Link::pull(ROOT3));
                }
            }
            BrickName::RightTwist => {}
            BrickName::LeftOmniTwist => {}
            BrickName::RightOmniTwist => {}
            BrickName::LeftMitosis => {}
            BrickName::RightMitosis => {}
        }
        fabric
    }
}

fn middle(points: [Point3<f32>; 3]) -> Point3<f32> {
    (points[0] + points[1].to_vec() + points[2].to_vec()) / 3f32
}

#[test]
fn left_twist() {
    let mut fabric = Fabric::default();
    let [(_, face_id), ..] = fabric.double_twist(Spin::Left, 1.3, 1.0, None);
    fabric.set_altitude(10.0);
    let brick = Brick::from((fabric, face_id));
    dbg!(brick);
}
