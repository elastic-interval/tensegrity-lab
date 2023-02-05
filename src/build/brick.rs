use std::f32::consts::{PI, SQRT_2};

use cgmath::{EuclideanSpace, Matrix4, MetricSpace, Point3, Quaternion, Rotation, Vector3};
use clap::ValueEnum;

use crate::build::tenscript::{FaceName, Spin};
use crate::build::tenscript::FaceName::{*};
use crate::build::tenscript::Spin::{Left, Right};
use crate::fabric::{Fabric, Link, UniqueId};
use crate::fabric::interval::{Interval, Role};
use crate::fabric::interval::Role::{Pull, Push};
use crate::fabric::joint::Joint;

#[derive(Debug, Clone)]
pub struct Brick {
    pub joints: Vec<Point3<f32>>,
    pub intervals: Vec<(usize, usize, Role)>,
    pub faces: Vec<([usize; 3], FaceName, Spin)>,
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
            .map(|(joints, face_name, spin)|
                format!("        ({joints:?}, {face_name:?}, {spin:?} ),")));
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
            Matrix4::from(Quaternion::between_vectors(face.normal(&fabric), Vector3::unit_y())) *
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
                .map(|face| (face.radial_joints(&fabric), face.face_name, face.spin))
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
const ROOT5: f32 = 2.236_068;
const PHI: f32 = (1f32 + ROOT5) / 2f32;

impl Brick {
    pub fn new(name: BrickName) -> Brick {
        match name {
            BrickName::LeftTwist => Brick {
                joints: vec![
                    Point3::new(-0.499999, 0.000019, 0.866028),
                    Point3::new(-0.500001, -0.000019, -0.866028),
                    Point3::new(1.000000, 0.000000, -0.000000),
                    Point3::new(0.004655, 1.652024, -1.000031),
                    Point3::new(0.863694, 1.652055, 0.503981),
                    Point3::new(-0.868339, 1.652059, 0.495929),
                    Point3::new(-0.000004, 0.000068, 0.000003),
                    Point3::new(0.000002, 1.651976, -0.000046),
                ],
                intervals: vec![
                    (0, 3, Push),
                    (0, 5, Pull),
                    (1, 3, Pull),
                    (2, 5, Push),
                    (1, 4, Push),
                    (2, 4, Pull),
                ],
                faces: vec![
                    ([2, 1, 0], Aneg, Left),
                    ([3, 4, 5], Apos, Left),
                ],
            },
            BrickName::RightTwist => Brick {
                joints: vec![
                    Point3::new(-0.500001, -0.000005, 0.866024),
                    Point3::new(-0.499999, 0.000005, -0.866024),
                    Point3::new(1.000000, 0.000000, 0.000000),
                    Point3::new(0.863691, 1.652046, -0.504017),
                    Point3::new(0.004651, 1.652039, 0.999993),
                    Point3::new(-0.868338, 1.652043, -0.495961),
                    Point3::new(-0.000000, 0.000068, 0.000004),
                    Point3::new(0.000006, 1.651973, 0.000004),
                ],
                intervals: vec![
                    (2, 5, Push),
                    (2, 3, Pull),
                    (1, 4, Push),
                    (0, 4, Pull),
                    (0, 3, Push),
                    (1, 5, Pull),
                ],
                faces: vec![
                    ([2, 1, 0], Aneg, Right),
                    ([3, 4, 5], Apos, Right),
                ],
            },
            BrickName::LeftOmniTwist => unimplemented!(),
            BrickName::RightOmniTwist => unimplemented!(),
            BrickName::LeftMitosis => unimplemented!(),
            BrickName::RightMitosis => unimplemented!(),
        }
    }

    pub fn prototype(name: BrickName) -> (Fabric, UniqueId) {
        let pretenst_factor = 1.3;
        let mut fabric = Fabric::default();
        let face_id = match name {
            BrickName::LeftTwist | BrickName::RightTwist => {
                let (spin, to_skip) = match name {
                    BrickName::RightTwist => (Right, 1),
                    BrickName::LeftTwist => (Left, 2),
                    _ => unreachable!()
                };
                let bot = [0, 1, 2].map(|index| {
                    let angle = index as f32 * PI * 2.0 / 3.0;
                    Point3::from([angle.cos(), 0.0, angle.sin()])
                });
                let top = bot.map(|point| point + Vector3::unit_y());
                let alpha_joints = bot.map(|point| fabric.create_joint(point));
                let omega_joints = top.map(|point| fabric.create_joint(point));
                for (&alpha_index, &omega_index) in alpha_joints.iter().zip(omega_joints.iter()) {
                    fabric.create_interval(alpha_index, omega_index, Link::push(ROOT6 * pretenst_factor));
                }
                let alpha_midpoint = fabric.create_joint(middle(bot));
                let alpha_radials = alpha_joints.map(|joint| {
                    fabric.create_interval(alpha_midpoint, joint, Link::pull(1.0))
                });
                let mut alpha_radials_reversed = alpha_radials;
                alpha_radials_reversed.reverse();
                let alpha_face = fabric.create_face(Aneg, 1.0, spin, alpha_radials_reversed);
                let omega_midpoint = fabric.create_joint(middle(top));
                let omega_radials = omega_joints.map(|joint| {
                    fabric.create_interval(omega_midpoint, joint, Link::pull(1.0))
                });
                fabric.create_face(Apos, 1.0, spin, omega_radials);
                let advanced_omega = omega_joints.iter().cycle().skip(to_skip).take(3);
                for (&alpha_index, &omega_index) in alpha_joints.iter().zip(advanced_omega) {
                    fabric.create_interval(alpha_index, omega_index, Link::pull(ROOT3));
                }
                alpha_face
            }
            BrickName::LeftOmniTwist | BrickName::RightOmniTwist => {
                let factor = PHI * ROOT3 / 2.0 / SQRT_2;
                let points @ [aaa, bbb, ccc, ddd] =
                    [(1.0, 1.0, 1.0), (1.0, -1.0, -1.0), (-1.0, -1.0, 1.0), (-1.0, 1.0, -1.0)]
                        .map(|(x, y, z)| Point3::new(x, y, z) * factor);
                let opposing_points = [[bbb, ddd, ccc], [aaa, ccc, ddd], [aaa, ddd, bbb], [bbb, ccc, aaa]]
                    .map(|points| points.map(Point3::to_vec).iter().sum::<Vector3<f32>>() / 3.0)
                    .map(Point3::from_vec);
                let mut joint_at = |point: Point3<f32>| fabric.create_joint(point);
                let [
                a, ab, ac, ad,
                b, ba, bc, bd,
                c, ca, cb, cd,
                d, da, db, dc
                ] = points
                    .into_iter()
                    .flat_map(|point|
                        [joint_at(point), joint_at(point), joint_at(point), joint_at(point)])
                    .next_chunk()
                    .unwrap();
                let pairs = [(ab, ba), (ac, ca), (ad, da), (bc, cb), (bd, db), (cd, dc)];
                let [bdc, acd, adb, bca] = opposing_points.map(joint_at);
                for (alpha_index, omega_index) in pairs {
                    fabric.create_interval(alpha_index, omega_index, Link::push(PHI * ROOT3));
                }
                let face_facts = [
                    (a, [ab, ac, ad], false, Aneg),
                    (b, [ba, bc, bd], false, Bneg),
                    (c, [ca, cb, cd], false, Cneg),
                    (d, [da, db, dc], false, Dneg),
                    (bdc, [bd, dc, cb], true, Apos),
                    (acd, [ac, cd, da], true, Bpos),
                    (adb, [ad, db, ba], true, Cpos),
                    (bca, [bc, ca, ab], true, Dpos),
                ];
                let faces = face_facts
                    .map(|(alpha_index, omega_indexes, opposite, face_name)| {
                        let radials = omega_indexes.map(|omega_index| {
                            fabric.create_interval(alpha_index, omega_index, Link::pull(1.0))
                        });
                        let spin = match name {
                            BrickName::LeftOmniTwist if !opposite => Left,
                            BrickName::LeftOmniTwist if opposite => Right,
                            BrickName::RightOmniTwist if opposite => Left,
                            BrickName::RightOmniTwist if !opposite => Right,
                            _ => unreachable!()
                        };
                        fabric.create_face(face_name, 1.0, spin, radials)
                    });
                faces[0]
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