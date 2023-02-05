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
                format!("        ({joints:?}, {face_name:?}, {spin:?}, ),")));
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
                    Point3::new(1.000000, 0.000000, -0.000000),
                    Point3::new(-0.500001, 0.000006, 0.866025),
                    Point3::new(-0.499999, -0.000006, -0.866025),
                    Point3::new(-0.868349, 1.652046, 0.495955),
                    Point3::new(0.004644, 1.652035, -0.999997),
                    Point3::new(0.863685, 1.652050, 0.504012),
                    Point3::new(0.000004, 0.000069, 0.000001),
                    Point3::new(-0.000010, 1.651975, -0.000005),
                ],
                intervals: vec![
                    (0, 3, Push),
                    (0, 5, Pull),
                    (1, 3, Pull),
                    (2, 4, Pull),
                    (1, 4, Push),
                    (2, 5, Push),
                ],
                faces: vec![
                    ([3, 4, 5], Apos, Left, ),
                    ([0, 1, 2], Aneg, Left, ),
                ],
            },
            BrickName::RightTwist => Brick {
                joints: vec![
                    Point3::new(1.000000, 0.000000, -0.000000),
                    Point3::new(-0.499998, -0.000005, 0.866024),
                    Point3::new(-0.500002, 0.000005, -0.866024),
                    Point3::new(-0.868347, 1.652043, -0.495956),
                    Point3::new(0.863685, 1.652047, -0.504014),
                    Point3::new(0.004646, 1.652036, 0.999995),
                    Point3::new(0.000004, 0.000069, -0.000001),
                    Point3::new(-0.000009, 1.651971, 0.000004),
                ],
                intervals: vec![
                    (0, 4, Pull),
                    (2, 3, Pull),
                    (2, 5, Push),
                    (0, 3, Push),
                    (1, 4, Push),
                    (1, 5, Pull),
                ],
                faces: vec![
                    ([0, 1, 2], Aneg, Right, ),
                    ([3, 4, 5], Apos, Right, ),
                ],
            },
            BrickName::LeftOmniTwist => Brick {
                joints: vec![
                    Point3::new(0.042248, -0.047202, 0.035933),
                    Point3::new(0.789643, 0.576105, -0.211108),
                    Point3::new(-0.508176, 0.137923, 0.850524),
                    Point3::new(-0.281467, -0.714028, -0.639417),
                    Point3::new(1.024276, -1.566879, -0.163855),
                    Point3::new(0.279277, -2.181409, 0.090764),
                    Point3::new(0.920942, -0.975422, -0.968180),
                    Point3::new(1.927835, -1.680073, 0.247834),
                    Point3::new(1.615774, -0.239910, 0.929035),
                    Point3::new(2.157902, -0.433107, 0.116740),
                    Point3::new(1.701578, -0.818945, 1.737427),
                    Point3::new(1.143387, 0.641706, 1.010987),
                    Point3::new(0.163913, -1.228979, 1.410461),
                    Point3::new(0.497353, -0.566970, 2.069039),
                    Point3::new(-0.732260, -1.104025, 0.987302),
                    Point3::new(0.625561, -2.112558, 1.312718),
                    Point3::new(1.390541, -1.508260, 1.076798),
                    Point3::new(0.388223, 0.042557, 1.282897),
                    Point3::new(-0.205827, -1.308363, 0.166420),
                    Point3::new(1.263362, -0.302174, -0.306450),
                ],
                intervals: vec![
                    (11, 15, Push),
                    (2, 9, Push),
                    (7, 14, Push),
                    (1, 5, Push),
                    (6, 10, Push),
                    (3, 13, Push),
                ],
                faces: vec![
                    ([2, 11, 13], Bpos, Right, ),
                    ([13, 14, 15], Dneg, Left, ),
                    ([1, 2, 3], Aneg, Left, ),
                    ([6, 9, 1], Dpos, Right, ),
                    ([9, 10, 11], Cneg, Left, ),
                    ([7, 15, 10], Apos, Right, ),
                    ([3, 14, 5], Cpos, Right, ),
                    ([5, 6, 7], Bneg, Left, ),
                ],
            },
            BrickName::RightOmniTwist => Brick {
                joints: vec![
                    Point3::new(0.038414, -0.043832, 0.030870),
                    Point3::new(0.789857, 0.575780, -0.211196),
                    Point3::new(-0.506606, 0.138893, 0.852319),
                    Point3::new(-0.283251, -0.714673, -0.641123),
                    Point3::new(1.031779, -1.567751, -0.168480),
                    Point3::new(0.284101, -2.187320, 0.082170),
                    Point3::new(0.923014, -0.975889, -0.971714),
                    Point3::new(1.926762, -1.681584, 0.245755),
                    Point3::new(1.608890, -0.243373, 0.930839),
                    Point3::new(2.159838, -0.435039, 0.113142),
                    Point3::new(1.703202, -0.829149, 1.743191),
                    Point3::new(1.137294, 0.640867, 1.015335),
                    Point3::new(0.166095, -1.241341, 1.401855),
                    Point3::new(0.496459, -0.567303, 2.067696),
                    Point3::new(-0.739823, -1.110423, 0.985562),
                    Point3::new(0.630226, -2.120438, 1.310938),
                    Point3::new(1.398309, -1.514541, 1.083463),
                    Point3::new(0.395474, 0.032509, 1.278563),
                    Point3::new(-0.194008, -1.306332, 0.167338),
                    Point3::new(1.267940, -0.296118, -0.318080),
                ],
                intervals: vec![
                    (2, 9, Push),
                    (7, 14, Push),
                    (11, 15, Push),
                    (3, 13, Push),
                    (6, 10, Push),
                    (1, 5, Push),
                ],
                faces: vec![
                    ([1, 2, 3], Aneg, Right, ),
                    ([7, 15, 10], Apos, Left, ),
                    ([3, 14, 5], Cpos, Left, ),
                    ([6, 9, 1], Dpos, Left, ),
                    ([13, 14, 15], Dneg, Right, ),
                    ([9, 10, 11], Cneg, Right, ),
                    ([5, 6, 7], Bneg, Right, ),
                    ([2, 11, 13], Bpos, Left, ),
                ],
            },
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
                let alpha_face = fabric.create_face(FaceName::Aneg, 1.0, spin, alpha_radials);
                let omega_midpoint = fabric.create_joint(middle(top));
                let omega_radials = omega_joints.map(|joint| {
                    fabric.create_interval(omega_midpoint, joint, Link::pull(1.0))
                });
                fabric.create_face(FaceName::Apos, 1.0, spin, omega_radials);
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
                    (a, [ab, ac, ad], false, FaceName::Aneg),
                    (b, [ba, bc, bd], false, FaceName::Bneg),
                    (c, [ca, cb, cd], false, FaceName::Cneg),
                    (d, [da, db, dc], false, FaceName::Dneg),
                    (bdc, [bd, dc, cb], true, FaceName::Apos),
                    (acd, [ac, cd, da], true, FaceName::Bpos),
                    (adb, [ad, db, ba], true, FaceName::Cpos),
                    (bca, [bc, ca, ab], true, FaceName::Dpos),
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
