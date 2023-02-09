use std::f32::consts::PI;

use cgmath::{EuclideanSpace, Point3, SquareMatrix, Vector3};
use cgmath::num_traits::abs;

use crate::build::brick::{Baked, BrickName};
use crate::build::tenscript::{FaceName, Spin};
use crate::build::tenscript::Spin::{Left, Right};
use crate::fabric::{Fabric, Link, UniqueId};
use crate::fabric::interval::Interval;
use crate::fabric::joint::Joint;

impl Baked {
    pub const TARGET_FACE_STRAIN: f32 = 0.1;

    pub fn into_code(self) -> String {
        let mut lines = Vec::<String>::new();
        lines.push("Brick {".to_string());

        lines.push("    joints: vec![".to_string());
        lines.extend(self.joints
            .into_iter()
            .map(|Point3 { x, y, z }|
                format!("        point3({x:.4}, {y:.4}, {z:.4}),")));
        lines.push("    ],".to_string());

        lines.push("    intervals: vec![".to_string());
        lines.extend(self.intervals
            .into_iter()
            .map(|(alpha, omega, role, strain)|
                format!("        ({alpha}, {omega}, {role:?}, {strain:.4}),")));
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

impl TryFrom<(Fabric, UniqueId)> for Baked {
    type Error = String;

    fn try_from((fabric, face_id): (Fabric, UniqueId)) -> Result<Self, String> {
        let mut fabric = fabric;
        let face = fabric.face(face_id);
        fabric.apply_matrix4(face.vector_space(&fabric, false).invert().unwrap());
        let joint_incident = fabric.joint_incident();
        let target_face_strain = Baked::TARGET_FACE_STRAIN;
        for face in fabric.faces.values() {
            let strain = face.strain(&fabric);
            if abs(strain - target_face_strain) > 0.0001 {
                return Err(format!("Face interval strain too far from {target_face_strain} {strain:.5}"));
            }
        }
        Ok(Self {
            joints: fabric.joints
                .iter()
                .map(|Joint { location, .. }| *location)
                .collect(),
            intervals: fabric.interval_values()
                .filter_map(|Interval { alpha_index, omega_index, material, strain, .. }|
                    joint_incident[*alpha_index].push
                        .map(|_| (*alpha_index, *omega_index, fabric.materials[*material].role, *strain)))
                .collect(),
            faces: fabric.faces
                .values()
                .map(|face| (face.radial_joints(&fabric), face.face_name, face.spin))
                .collect(),
        })
    }
}

impl Baked {
    pub fn prototype(name: BrickName) -> (Fabric, UniqueId) {
        match name {
            BrickName::LeftTwist | BrickName::RightTwist => {
                let mut fabric = Fabric::default();
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
                    fabric.create_interval(alpha_index, omega_index, Link::push(3.205));
                }
                let bot_middle = bot.into_iter().map(|p| p.to_vec()).sum::<Vector3<f32>>() / 3.0;
                let alpha_midpoint = fabric.create_joint(Point3::from_vec(bot_middle));
                let alpha_radials = alpha_joints.map(|joint| {
                    fabric.create_interval(alpha_midpoint, joint, Link::pull(1.0))
                });
                let mut alpha_radials_reversed = alpha_radials;
                alpha_radials_reversed.reverse();
                let alpha_face = fabric.create_face(FaceName(0), 1.0, spin, alpha_radials_reversed);
                let top_middle = top.into_iter().map(|p| p.to_vec()).sum::<Vector3<f32>>() / 3.0;
                let omega_midpoint = fabric.create_joint(Point3::from_vec(top_middle));
                let omega_radials = omega_joints.map(|joint| {
                    fabric.create_interval(omega_midpoint, joint, Link::pull(1.0))
                });
                fabric.create_face(FaceName(1), 1.0, spin, omega_radials);
                let advanced_omega = omega_joints.iter().cycle().skip(to_skip).take(3);
                for (&alpha_index, &omega_index) in alpha_joints.iter().zip(advanced_omega) {
                    fabric.create_interval(alpha_index, omega_index, Link::pull(2.0));
                }
                (fabric, alpha_face)
            }
            BrickName::LeftOmniTwist | BrickName::RightOmniTwist => {
                let mut fabric = Fabric::default();
                let points @ [aaa, bbb, ccc, ddd] =
                    [(1.0, 1.0, 1.0), (1.0, -1.0, -1.0), (-1.0, -1.0, 1.0), (-1.0, 1.0, -1.0)]
                        .map(|(x, y, z)| Point3::new(x, y, z));
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
                    fabric.create_interval(alpha_index, omega_index, Link::push(3.271));
                }
                let spin = match name {
                    BrickName::LeftOmniTwist => Left,
                    BrickName::RightOmniTwist => Right,
                    _ => unreachable!(),
                };
                let face_facts = [
                    (a, [ab, ac, ad], spin.opposite(), FaceName(0)),
                    (b, [ba, bd, bc], spin.opposite(), FaceName(2)),
                    (c, [ca, cb, cd], spin.opposite(), FaceName(4)),
                    (d, [da, dc, db], spin.opposite(), FaceName(6)),
                    (bdc, if spin == Right { [bd, dc, cb] } else { [db, cd, bc] }, spin, FaceName(1)),
                    (acd, if spin == Right { [ac, cd, da] } else { [ca, dc, ad] }, spin, FaceName(3)),
                    (adb, if spin == Right { [ad, db, ba] } else { [da, bd, ab] }, spin, FaceName(5)),
                    (bca, if spin == Right { [bc, ca, ab] } else { [cb, ac, ba] }, spin, FaceName(7)),
                ];
                let faces = face_facts
                    .map(|(alpha_index, omega_indexes, spin, face_name)| {
                        let radials = omega_indexes.map(|omega_index| {
                            fabric.create_interval(alpha_index, omega_index, Link::pull(1.0))
                        });
                        fabric.create_face(face_name, 1.0, spin, radials)
                    });
                (fabric, faces[0])
            }
            BrickName::RightMitosis | BrickName::LeftMitosis => {
                let mut p = Prototype::default();
                let normal_push_length = 3.467;
                let [
                (left_front, left_back),
                (middle_front, middle_back),
                (right_front, right_back),
                ] = p.x(normal_push_length);
                let [
                (front_left_bottom, front_left_top),
                (front_right_bottom, front_right_top),
                (back_left_bottom, back_left_top),
                (back_right_bottom, back_right_top),
                ] = p.y(normal_push_length);
                let [
                (top_left, top_right),
                (bottom_left, bottom_right),
                ] = p.z(normal_push_length * 2.0);
                p.pull(2.5, &[
                    (middle_front, front_left_bottom),
                    (middle_front, front_left_top),
                    (middle_front, front_right_bottom),
                    (middle_front, front_right_top),
                    (middle_back, back_left_bottom),
                    (middle_back, back_left_top),
                    (middle_back, back_right_bottom),
                    (middle_back, back_right_top),
                ]);
                let left = name == BrickName::LeftMitosis;
                p.left(&[
                    ([top_left, left_back, back_left_top],
                     if left { FaceName(0) } else { FaceName(7) }),
                    ([bottom_left, left_front, front_left_bottom],
                     if left { FaceName(2) } else { FaceName(5) }),
                    ([top_right, right_front, front_right_top],
                     if left { FaceName(4) } else { FaceName(3) }),
                    ([bottom_right, right_back, back_right_bottom],
                     if left { FaceName(6) } else { FaceName(1) }),
                ]);
                p.right(&[
                    ([top_left, front_left_top, left_front],
                     if left { FaceName(7) } else { FaceName(0) }),
                    ([bottom_left, back_left_bottom, left_back],
                     if left { FaceName(5) } else { FaceName(2) }),
                    ([top_right, back_right_top, right_back],
                     if left { FaceName(3) } else { FaceName(4) }),
                    ([bottom_right, front_right_bottom, right_front],
                     if left { FaceName(1) } else { FaceName(6) }),
                ]);
                p.into()
            }
        }
    }
}

#[derive(Default, Clone)]
struct Prototype {
    fabric: Fabric,
    face_id: Option<UniqueId>,
}

impl From<Prototype> for (Fabric, UniqueId) {
    fn from(value: Prototype) -> Self {
        (value.fabric, value.face_id.expect("no main face id"))
    }
}

impl Prototype {
    pub fn x<const N: usize>(&mut self, length: f32) -> [(usize, usize); N] {
        self.push(length, Vector3::unit_x())
    }

    pub fn y<const N: usize>(&mut self, length: f32) -> [(usize, usize); N] {
        self.push(length, Vector3::unit_y())
    }

    pub fn z<const N: usize>(&mut self, length: f32) -> [(usize, usize); N] {
        self.push(length, Vector3::unit_z())
    }

    pub fn push<const N: usize>(&mut self, length: f32, axis: Vector3<f32>) -> [(usize, usize); N] {
        [(); N].map(|()| {
            let [alpha, omega] = [-length / 2.0, length / 2.0]
                .map(|offset| self.fabric.create_joint(Point3::from_vec(axis * offset)));
            self.fabric.create_interval(alpha, omega, Link::push(length));
            (alpha, omega)
        })
    }

    pub fn pull(&mut self, length: f32, pairs: &[(usize, usize)]) {
        for &(alpha_index, omega_index) in pairs {
            self.fabric.create_interval(alpha_index, omega_index, Link::pull(length));
        }
    }

    pub fn left(&mut self, triples: &[([usize; 3], FaceName)]) {
        self.add_face(triples, Left);
    }

    pub fn right(&mut self, triples: &[([usize; 3], FaceName)]) {
        self.add_face(triples, Right);
    }

    fn add_face(&mut self, triples: &[([usize; 3], FaceName)], spin: Spin) {
        for &(indices, face_name) in triples {
            let middle_point = indices
                .into_iter()
                .map(|index| self.fabric.joints[index].location.to_vec())
                .sum::<Vector3<_>>() / 3.0;
            let alpha_index = self.fabric.create_joint(Point3::from_vec(middle_point));
            let radial_intervals = indices
                .map(|omega_index| self.fabric.create_interval(alpha_index, omega_index, Link::pull(1.0))); //??)
            let face_id = self.fabric.create_face(face_name, 1.0, spin, radial_intervals);
            if face_name == FaceName(0) {
                self.face_id = Some(face_id);
            }
        }
    }
}