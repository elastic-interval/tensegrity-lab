use std::f32::consts::{PI, SQRT_2};

use cgmath::{EuclideanSpace, Point3, SquareMatrix, Vector3};
use crate::build::brick::{Brick, BrickName};

use crate::build::tenscript::FaceName::{*};
use crate::build::tenscript::Spin::{Left, Right};
use crate::fabric::{Fabric, Link, UniqueId};
use crate::fabric::interval::Interval;
use crate::fabric::joint::Joint;

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
                format!("        ({joints:?}, {face_name:?}, {spin:?}),")));
        lines.push("    ],".to_string());

        lines.push("}".to_string());
        lines.join("\n")
    }
}

impl From<(Fabric, UniqueId)> for Brick {
    fn from((fabric, face_id): (Fabric, UniqueId)) -> Self {
        let mut fabric = fabric;
        let face = fabric.face(face_id);
        fabric.apply_matrix4(face.vector_space(&fabric, false).invert().unwrap());
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

const ROOT3: f32 = 1.732_050_8;
const ROOT6: f32 = 2.449_489_8;
const ROOT5: f32 = 2.236_068;
const PHI: f32 = (1f32 + ROOT5) / 2f32;

impl Brick {
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
                let bot_middle = bot.into_iter().map(|p| p.to_vec()).sum::<Vector3<f32>>() / 3.0;
                let alpha_midpoint = fabric.create_joint(Point3::from_vec(bot_middle));
                let alpha_radials = alpha_joints.map(|joint| {
                    fabric.create_interval(alpha_midpoint, joint, Link::pull(1.0))
                });
                let mut alpha_radials_reversed = alpha_radials;
                alpha_radials_reversed.reverse();
                let alpha_face = fabric.create_face(Aneg, 1.0, spin, alpha_radials_reversed);
                let top_middle = top.into_iter().map(|p| p.to_vec()).sum::<Vector3<f32>>() / 3.0;
                let omega_midpoint = fabric.create_joint(Point3::from_vec(top_middle));
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
                let spin = match name {
                    BrickName::LeftOmniTwist => Left,
                    BrickName::RightOmniTwist => Right,
                    _ => unreachable!(),
                };
                let face_facts = [
                    (a, [ab, ac, ad], spin.opposite(), Aneg),
                    (b, [ba, bd, bc], spin.opposite(), Bneg),
                    (c, [ca, cb, cd], spin.opposite(), Cneg),
                    (d, [da, dc, db], spin.opposite(), Dneg),
                    (bdc, if spin == Right { [bd, dc, cb] } else { [db, cd, bc] }, spin, Apos),
                    (acd, if spin == Right { [ac, cd, da] } else { [ca, dc, ad] }, spin, Bpos),
                    (adb, if spin == Right { [ad, db, ba] } else { [da, bd, ab] }, spin, Cpos),
                    (bca, if spin == Right { [bc, ca, ab] } else { [cb, ac, ba] }, spin, Dpos),
                ];
                let faces = face_facts
                    .map(|(alpha_index, omega_indexes, spin, face_name)| {
                        let radials = omega_indexes.map(|omega_index| {
                            fabric.create_interval(alpha_index, omega_index, Link::pull(1.0))
                        });
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
