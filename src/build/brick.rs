use std::f32::consts::PI;

use cgmath::{EuclideanSpace, Matrix4, MetricSpace, Point3, Quaternion, Rotation, Vector3};
use clap::ValueEnum;

use crate::build::tenscript::Spin;
use crate::build::tenscript::Spin::Left;
use crate::fabric::{Fabric, Link, UniqueId};
use crate::fabric::interval::{Interval, Role};
use crate::fabric::interval::Role::{Pull, Push};
use crate::fabric::joint::Joint;

#[derive(Debug, Clone)]
pub struct Brick {
    pub joints: Vec<Point3<f32>>,
    pub intervals: Vec<(usize, usize, Role)>,
    pub faces: Vec<([usize; 3], Spin)>,
}

impl Brick {
    pub fn into_code(self) -> String {
        let mut lines = Vec::<String>::new();
        lines.push("Brick {".to_string());

        lines.push("    joints: vec![".to_string());
        lines.extend(self.joints
            .into_iter()
            .map(|Point3 { x, y, z }|
                format!("        Point3::new({x:.6}, {y:.6}, {z:.6}),")));
        lines.push("    ],".to_string());

        lines.push("    intervals: vec![".to_string());
        lines.extend(self.intervals
            .into_iter()
            .map(|(alpha, omega, role)|
                format!("        ({alpha}, {omega}, {role:?}),")));
        lines.push("    ],".to_string());

        lines.push("    faces: vec![".to_string());
        lines.extend(self.faces
            .into_iter()
            .map(|(joints, spin)|
                format!("        ({joints:?}, {spin:?}),")));
        lines.push("    ],".to_string());

        lines.push("}".to_string());
        lines.join("\n")
    }
}

impl From<(Fabric, UniqueId)> for Brick {
    fn from((fabric_template, face_id): (Fabric, UniqueId)) -> Self {
        let mut fabric = fabric_template;
        let face = fabric.face(face_id);
        let midpoint = face.midpoint(&fabric);
        let radial_x = face.radial_joint_locations(&fabric)[0].to_vec();
        let length = midpoint.distance(radial_x);
        let matrix = Matrix4::from_scale(1.0 / length) *
            Matrix4::from(Quaternion::between_vectors(face.normal(&fabric), -Vector3::unit_y())) *
            Matrix4::from(Quaternion::between_vectors(radial_x - midpoint, Vector3::unit_x())) *
            Matrix4::from_translation(-midpoint);
        fabric.apply_matrix4(matrix);
        let joint_incident = fabric.joint_incident();
        Self {
            joints: fabric.joints
                .iter()
                .map(|Joint { location, .. }| *location)
                .collect(),
            intervals: fabric.interval_values()
                .filter_map(|Interval { alpha_index, omega_index, material, .. }|
                    joint_incident[*alpha_index]
                        .push
                        .map(|_| (*alpha_index, *omega_index, fabric.materials[*material].role)))
                .collect(),
            faces: fabric.faces
                .values()
                .map(|face| (face.radial_joints(&fabric), face.spin))
                .collect(),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
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
            BrickName::LeftTwist => Brick {
                joints: vec![
                    Point3::new(1.000000, 0.000000, -0.000000),
                    Point3::new(-0.500000, 0.000000, 0.866025),
                    Point3::new(-0.500000, 0.000000, -0.866025),
                    Point3::new(-0.993273, 1.998635, 0.612987),
                    Point3::new(-0.034226, 1.998633, -1.166693),
                    Point3::new(1.027498, 1.998633, 0.553705),
                    Point3::new(-0.000000, 0.000000, -0.000000),
                    Point3::new(-0.000001, 2.001832, -0.000000),
                ],
                intervals: vec![
                    (0, 5, Pull),
                    (1, 4, Push),
                    (2, 5, Push),
                    (0, 3, Push),
                    (1, 3, Pull),
                    (2, 4, Pull),
                ],
                faces: vec![
                    ([3, 4, 5], Left),
                    ([0, 1, 2], Left),
                ],
            },
            BrickName::RightTwist => Brick {
                joints: vec![
                    Point3::new(1.000000, 0.000000, 0.000000),
                    Point3::new(-0.500000, 0.000000, 0.866025),
                    Point3::new(-0.500000, 0.000000, -0.866025),
                    Point3::new(-0.993273, 1.998635, -0.612986),
                    Point3::new(1.027497, 1.998633, -0.553706),
                    Point3::new(-0.034227, 1.998634, 1.166691),
                    Point3::new(-0.000000, 0.000000, 0.000000),
                    Point3::new(0.000000, 2.001832, -0.000001),
                ],
                intervals: vec![
                    (2, 5, Push),
                    (0, 4, Pull),
                    (2, 3, Pull),
                    (1, 5, Pull),
                    (0, 3, Push),
                    (1, 4, Push),
                ],
                faces: vec![
                    ([0, 1, 2], Left),
                    ([3, 4, 5], Left),
                ],
            },
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

    pub fn prototype(name: BrickName) -> (Fabric, UniqueId) {
        let mut fabric = Fabric::default();
        let bot = [0, 1, 2].map(|index| {
            let angle = index as f32 * PI * 2.0 / 3.0;
            Point3::from([angle.cos(), 0.0, angle.sin()])
        });
        let face_id = match name {
            BrickName::LeftTwist | BrickName::RightTwist => {
                let top = bot.map(|point| point + Vector3::unit_y());
                let alpha_joints = bot.map(|point| fabric.create_joint(point));
                let omega_joints = top.map(|point| fabric.create_joint(point));
                let pushes = alpha_joints
                    .iter()
                    .zip(omega_joints.iter())
                    .map(|(&alpha_index, &omega_index)| fabric.create_interval(alpha_index, omega_index, Link::push(ROOT6 * 1.3)))
                    .next_chunk()
                    .unwrap();
                let alpha_midpoint = fabric.create_joint(middle(bot));
                let alpha_radials = alpha_joints.map(|joint| {
                    fabric.create_interval(alpha_midpoint, joint, Link::pull(1.0))
                });
                let alpha_face = fabric.create_face(1.0, Left, alpha_radials, pushes);
                let omega_midpoint = fabric.create_joint(middle(top));
                let omega_radials = omega_joints.map(|joint| {
                    fabric.create_interval(omega_midpoint, joint, Link::pull(1.0))
                });
                fabric.create_face(1.0, Left, omega_radials, pushes);
                let to_skip = match name {
                    BrickName::RightTwist => 1,
                    BrickName::LeftTwist => 2,
                    _ => unreachable!()
                };
                let advanced_omega = omega_joints.iter().cycle().skip(to_skip).take(3);
                for (&alpha_index, &omega_index) in alpha_joints.iter().zip(advanced_omega) {
                    fabric.create_interval(alpha_index, omega_index, Link::pull(ROOT3));
                }
                alpha_face
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
        };
        (fabric, face_id)
    }
}

fn middle(points: [Point3<f32>; 3]) -> Point3<f32> {
    (points[0] + points[1].to_vec() + points[2].to_vec()) / 3f32
}

#[test]
fn left_twist() {
    let mut fabric = Fabric::default();
    let [(_, face_id), ..] = fabric.double_twist(Left, 1.3, 1.0, None);
    fabric.set_altitude(10.0);
    let brick = Brick::from((fabric, face_id));
    dbg!(brick);
}
