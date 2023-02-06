use std::f32::consts::{PI, SQRT_2};

use cgmath::{EuclideanSpace, Point3, SquareMatrix, Vector3};
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
        fabric.apply_matrix4(face.space(&fabric, false).invert().unwrap());
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
                    Point3::new(-0.500000, -0.000000, 0.866028),
                    Point3::new(-0.500000, 0.000000, -0.866028),
                    Point3::new(1.000000, 0.000000, 0.000000),
                    Point3::new(0.004655, 1.652047, -0.999992),
                    Point3::new(0.863694, 1.652043, 0.504021),
                    Point3::new(-0.868339, 1.652046, 0.495967),
                    Point3::new(-0.000004, 0.000068, 0.000004),
                    Point3::new(0.000003, 1.651977, -0.000007),
                ],
                intervals: vec![
                    (1, 3, Pull),
                    (0, 3, Push),
                    (1, 4, Push),
                    (2, 4, Pull),
                    (2, 5, Push),
                    (0, 5, Pull),
                ],
                faces: vec![
                    ([2, 1, 0], Aneg, Left),
                    ([3, 4, 5], Apos, Left),
                ],
            },
            BrickName::RightTwist =>Brick {
                joints: vec![
                    Point3::new(-0.500001, 0.000000, 0.866024),
                    Point3::new(-0.499998, 0.000000, -0.866024),
                    Point3::new(1.000000, 0.000000, 0.000000),
                    Point3::new(0.863693, 1.652043, -0.504027),
                    Point3::new(0.004651, 1.652045, 0.999985),
                    Point3::new(-0.868336, 1.652040, -0.495971),
                    Point3::new(-0.000001, 0.000069, 0.000005),
                    Point3::new(0.000009, 1.651973, -0.000005),
                ],
                intervals: vec![
                    (0, 3, Push),
                    (2, 3, Pull),
                    (2, 5, Push),
                    (1, 5, Pull),
                    (0, 4, Pull),
                    (1, 4, Push),
                ],
                faces: vec![
                    ([2, 1, 0], Aneg, Right),
                    ([3, 4, 5], Apos, Right),
                ],
            },
            BrickName::RightOmniTwist => Brick {
                joints: vec![
                    Point3::new(0.008714, 0.056616, 0.000579),
                    Point3::new(1.000000, -0.000000, 0.000000),
                    Point3::new(-0.498965, -0.000000, -0.866282),
                    Point3::new(-0.501035, -0.000000, 0.866282),
                    Point3::new(-0.059065, 1.559766, 1.047565),
                    Point3::new(-1.061005, 1.634545, 1.040761),
                    Point3::new(0.371996, 0.743354, 1.437684),
                    Point3::new(0.496198, 2.371572, 0.865391),
                    Point3::new(0.938741, 1.558709, -0.472263),
                    Point3::new(1.430259, 1.636447, 0.394230),
                    Point3::new(0.496308, 2.371334, -0.867739),
                    Point3::new(1.057067, 0.740329, -1.043843),
                    Point3::new(-0.869593, 1.547232, -0.568764),
                    Point3::new(-0.378333, 1.631422, -1.440080),
                    Point3::new(-1.432399, 0.737172, -0.396862),
                    Point3::new(-1.005423, 2.368254, -0.002648),
                    Point3::new(-0.003867, 2.332181, -0.001919),
                    Point3::new(0.053163, 0.806793, -1.062118),
                    Point3::new(-0.962387, 0.805705, 0.490552),
                    Point3::new(0.883026, 0.813121, 0.577837),
                ],
                intervals: vec![
                    (3, 13, Push),
                    (2, 9, Push),
                    (11, 15, Push),
                    (1, 5, Push),
                    (7, 14, Push),
                    (6, 10, Push),
                ],
                faces: vec![
                    ([2, 11, 13], Bpos, Right),
                    ([13, 15, 14], Dneg, Left),
                    ([7, 15, 10], Apos, Right),
                    ([3, 14, 5], Cpos, Right),
                    ([6, 9, 1], Dpos, Right),
                    ([5, 7, 6], Bneg, Left),
                    ([1, 2, 3], Aneg, Left),
                    ([9, 10, 11], Cneg, Left),
                ],
            },
            BrickName::LeftOmniTwist =>Brick {
                joints: vec![
                    Point3::new(-0.001052, 0.065878, -0.005261),
                    Point3::new(1.000000, -0.000000, -0.000000),
                    Point3::new(-0.498019, -0.000000, -0.862215),
                    Point3::new(-0.501982, 0.000000, 0.862215),
                    Point3::new(-0.068790, 1.552298, -1.043123),
                    Point3::new(-1.060286, 1.637359, -1.042465),
                    Point3::new(0.501305, 2.360666, -0.871356),
                    Point3::new(0.368141, 0.735169, -1.432105),
                    Point3::new(-0.886729, 1.554949, 0.579371),
                    Point3::new(-0.369899, 1.636865, 1.432374),
                    Point3::new(-1.425979, 0.738043, 0.399946),
                    Point3::new(-1.000678, 2.362999, -0.000470),
                    Point3::new(0.928972, 1.552220, 0.460547),
                    Point3::new(1.427644, 1.629460, -0.400411),
                    Point3::new(0.503571, 2.365928, 0.866142),
                    Point3::new(1.061593, 0.737880, 1.033950),
                    Point3::new(0.002486, 2.323556, -0.000375),
                    Point3::new(0.060304, 0.808495, 1.062984),
                    Point3::new(0.902238, 0.804924, -0.588535),
                    Point3::new(-0.962040, 0.807676, -0.485882),
                ],
                intervals: vec![
                    (11, 15, Push),
                    (3, 13, Push),
                    (7, 14, Push),
                    (2, 9, Push),
                    (6, 10, Push),
                    (1, 5, Push),
                ],
                faces: vec![
                    ([10, 2, 5], Dpos, Left),
                    ([13, 15, 14], Dneg, Right),
                    ([1, 2, 3], Aneg, Right),
                    ([5, 7, 6], Bneg, Right),
                    ([14, 11, 6], Apos, Left),
                    ([13, 7, 1], Cpos, Left),
                    ([9, 15, 3], Bpos, Left),
                    ([9, 10, 11], Cneg, Right),
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

fn middle(points: [Point3<f32>; 3]) -> Point3<f32> {
    (points[0] + points[1].to_vec() + points[2].to_vec()) / 3f32
}

#[test]
fn test_brick() {
    let (mut fabric, face_id) = Brick::prototype(BrickName::LeftOmniTwist);
    for _ in 1..50000 {
        fabric.iterate(&crate::fabric::physics::presets::LIQUID);
    }
    fabric.set_altitude(10.0);
    let brick = Brick::from((fabric.clone(), face_id));
    println!("{}", brick.clone().into_code());
    dbg!(a_neg_face(brick));
}

#[test]
fn test_bases() {
    dbg!(a_neg_face(Brick::new(BrickName::LeftTwist)));
    dbg!(a_neg_face(Brick::new(BrickName::RightTwist)));
    // dbg!(a_neg_face(Brick::new(BrickName::LeftOmniTwist)));
    dbg!(a_neg_face(Brick::new(BrickName::RightOmniTwist)));
}

fn a_neg_face(brick: Brick) -> [Point3<f32>; 3] {
    let (joints, _, _) = brick.faces.into_iter().find(|&(_, face_name, _)| face_name == Aneg).unwrap();
    let face_locations: [Point3<f32>; 3] = joints
        .into_iter()
        .map(|index| brick.joints[index])
        .next_chunk().unwrap();
    face_locations
}

