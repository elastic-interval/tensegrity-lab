use std::f32::consts::{PI, SQRT_2};

use cgmath::{EuclideanSpace, Matrix4, MetricSpace, Point3, Quaternion, Rotation, Vector3};
use clap::ValueEnum;

use crate::build::tenscript::Spin;
use crate::build::tenscript::Spin::{Left, Right};
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
const ROOT5: f32 = 2.236_068;
const PHI: f32 = (1f32 + ROOT5) / 2f32;

impl Brick {
    pub fn new(name: BrickName) -> Brick {
        match name {
            BrickName::LeftTwist => Brick {
                joints: vec![
                    Point3::new(1.000000, -0.000000, 0.000000),
                    Point3::new(-0.500000, 0.000000, 0.866026),
                    Point3::new(-0.500000, -0.000000, -0.866026),
                    Point3::new(-0.938857, 1.693923, 0.344309),
                    Point3::new(0.171248, 1.693924, -0.985228),
                    Point3::new(0.767608, 1.693923, 0.640920),
                    Point3::new(-0.000002, -0.001239, -0.000001),
                    Point3::new(0.000001, 1.695163, -0.000001),
                ],
                intervals: vec![
                    (0, 3, Push),
                    (1, 4, Push),
                    (2, 5, Push),
                    (1, 3, Pull),
                    (0, 5, Pull),
                    (2, 4, Pull),
                ],
                faces: vec![
                    ([0, 1, 2], Left),
                    ([3, 4, 5], Left),
                ],
            },
            BrickName::RightTwist => Brick {
                joints: vec![
                    Point3::new(1.000000, 0.000000, -0.000000),
                    Point3::new(-0.499999, -0.000000, 0.866026),
                    Point3::new(-0.500001, 0.000000, -0.866026),
                    Point3::new(-0.938857, 1.693923, -0.344308),
                    Point3::new(0.767608, 1.693923, -0.640920),
                    Point3::new(0.171248, 1.693924, 0.985229),
                    Point3::new(-0.000002, -0.001239, 0.000001),
                    Point3::new(0.000001, 1.695163, 0.000002),
                ],
                intervals: vec![
                    (1, 5, Pull),
                    (1, 4, Push),
                    (2, 5, Push),
                    (2, 3, Pull),
                    (0, 3, Push),
                    (0, 4, Pull),
                ],
                faces: vec![
                    ([3, 4, 5], Left),
                    ([0, 1, 2], Left),
                ],
            },
            BrickName::LeftOmniTwist => Brick {
                joints: vec![
                    Point3::new(0.069160, -0.078281, 0.044261),
                    Point3::new(0.788675, 0.577350, -0.211324),
                    Point3::new(-0.463127, 0.089371, 0.881771),
                    Point3::new(-0.325548, -0.666722, -0.670447),
                    Point3::new(1.204150, -1.597471, -0.358807),
                    Point3::new(0.420426, -2.228622, -0.376282),
                    Point3::new(1.265051, -0.881760, -1.063732),
                    Point3::new(2.044837, -1.882700, 0.115274),
                    Point3::new(1.703401, -0.360764, 1.048281),
                    Point3::new(2.274030, -0.249920, 0.226720),
                    Point3::new(1.913956, -1.110014, 1.686371),
                    Point3::new(1.183209, 0.431500, 1.386798),
                    Point3::new(0.195100, -1.553342, 1.296029),
                    Point3::new(0.318349, -0.931358, 2.077575),
                    Point3::new(-0.683932, -1.546607, 0.805994),
                    Point3::new(0.779496, -2.370084, 1.230572),
                    Point3::new(1.613147, -1.825731, 1.032297),
                    Point3::new(0.327003, -0.104228, 1.489032),
                    Point3::new(-0.238745, -1.505622, -0.105411),
                    Point3::new(1.470411, -0.154249, -0.386156),
                ],
                intervals: vec![
                    (2, 9, Push),
                    (6, 10, Push),
                    (7, 14, Push),
                    (3, 13, Push),
                    (11, 15, Push),
                    (1, 5, Push),
                ],
                faces: vec![
                    ([6, 9, 1], Right),
                    ([7, 15, 10], Right),
                    ([3, 14, 5], Right),
                    ([1, 2, 3], Left),
                    ([13, 14, 15], Left),
                    ([9, 10, 11], Left),
                    ([2, 11, 13], Right),
                    ([5, 6, 7], Left),
                ],
            },
            BrickName::RightOmniTwist => Brick {
                joints: vec![
                    Point3::new(-0.039469, -1.976720, 1.436485),
                    Point3::new(-0.776073, -2.444549, 0.962878),
                    Point3::new(-0.243782, -1.497003, 2.281403),
                    Point3::new(0.896021, -2.260179, 1.262532),
                    Point3::new(0.643358, -1.243642, -0.192720),
                    Point3::new(1.512439, -1.014397, 0.229061),
                    Point3::new(0.396622, -2.200590, -0.288229),
                    Point3::new(0.211326, -0.577351, -0.788675),
                    Point3::new(-1.111118, -0.773390, 0.406172),
                    Point3::new(-1.134685, -1.395108, -0.367563),
                    Point3::new(-0.952014, 0.192133, 0.238231),
                    Point3::new(-1.559271, -1.043814, 1.249846),
                    Point3::new(0.425009, -0.123790, 1.342283),
                    Point3::new(-0.422847, 0.151095, 1.779693),
                    Point3::new(1.084923, -0.648079, 1.867061),
                    Point3::new(0.740688, 0.385218, 0.550444),
                    Point3::new(0.000868, 0.044102, -0.032056),
                    Point3::new(-0.772880, -0.786597, 1.814100),
                    Point3::new(1.215229, -1.319471, 1.135452),
                    Point3::new(-0.525449, -2.055575, 0.074689),
                ],
                intervals: vec![
                    (11, 15, Push),
                    (1, 5, Push),
                    (7, 14, Push),
                    (6, 10, Push),
                    (2, 9, Push),
                    (3, 13, Push),
                ],
                faces: vec![
                    ([6, 9, 1], Right),
                    ([1, 2, 3], Left),
                    ([3, 14, 5], Right),
                    ([2, 11, 13], Right),
                    ([5, 6, 7], Left),
                    ([7, 15, 10], Right),
                    ([9, 10, 11], Left),
                    ([13, 14, 15], Left),
                ],
            },
            BrickName::LeftMitosis => {
                unimplemented!()
            }
            BrickName::RightMitosis => {
                unimplemented!()
            }
        }
    }

    pub fn prototype(name: BrickName) -> (Fabric, UniqueId) {
        let pretenst_factor = 1.3;
        let mut fabric = Fabric::default();
        let face_id = match name {
            BrickName::LeftTwist | BrickName::RightTwist => {
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
                let alpha_face = fabric.create_face(1.0, Left, alpha_radials);
                let omega_midpoint = fabric.create_joint(middle(top));
                let omega_radials = omega_joints.map(|joint| {
                    fabric.create_interval(omega_midpoint, joint, Link::pull(1.0))
                });
                fabric.create_face(1.0, Left, omega_radials);
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
            BrickName::LeftOmniTwist | BrickName::RightOmniTwist => {
                let factor = PHI * ROOT3 / 2.0 / SQRT_2;
                let points @ [aaa, bbb, ccc, ddd] =
                    [(1.0, 1.0, 1.0), (1.0, -1.0, -1.0), (-1.0, -1.0, 1.0), (-1.0, 1.0, -1.0)]
                        .map(|(x, y, z)| Point3::new(x, y, z) * factor);
                let opposing = [[bbb, ddd, ccc], [aaa, ccc, ddd], [aaa, ddd, bbb], [bbb, ccc, aaa]]
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
                let [bdc, acd, adb, bca] = opposing.map(joint_at);
                for (alpha_index, omega_index) in pairs {
                    fabric.create_interval(alpha_index, omega_index, Link::push(PHI * ROOT3));
                }
                let face_facts = [
                    (a, [ab, ac, ad], Left),
                    (b, [ba, bc, bd], Left),
                    (c, [ca, cb, cd], Left),
                    (d, [da, db, dc], Left),
                    (bdc, [bd, dc, cb], Right),
                    (acd, [ac, cd, da], Right),
                    (adb, [ad, db, ba], Right),
                    (bca, [bc, ca, ab], Right),
                ];
                let faces = face_facts
                    .map(|(alpha_index, omega_indexes, spin)| {
                        let radials = omega_indexes.map(|omega_index| {
                            fabric.create_interval(alpha_index, omega_index, Link::pull(1.0))
                        });
                        fabric.create_face(1.0, spin, radials)
                    });
                match name {
                    BrickName::LeftOmniTwist => faces[0],
                    BrickName::RightOmniTwist => faces[4],
                    _ => unreachable!()
                }
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
