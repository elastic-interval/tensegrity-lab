use cgmath::{Point3, point3};
use clap::ValueEnum;

use crate::build::tenscript::{FaceName, Spin};
use crate::build::tenscript::FaceName::{*};
use crate::build::tenscript::Spin::{Left, Right};
use crate::fabric::interval::Role;
use crate::fabric::interval::Role::{Pull, Push};

#[derive(Debug, Clone)]
pub struct Brick {
    pub joints: Vec<Point3<f32>>,
    pub intervals: Vec<(usize, usize, Role, f32)>,
    pub faces: Vec<([usize; 3], FaceName, Spin)>,
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

impl Brick {
    pub fn new(name: BrickName) -> Brick {
        match name {
            BrickName::LeftTwist => Brick {
                joints: vec![
                    point3(-0.5000, 0.0000, 0.8660),
                    point3(-0.5000, 0.0000, -0.8660),
                    point3(1.0000, 0.0000, 0.0000),
                    point3(-0.0067, 1.9663, -1.0000),
                    point3(0.8694, 1.9663, 0.4941),
                    point3(-0.8626, 1.9663, 0.5058),
                    point3(0.0000, -0.0001, -0.0000),
                    point3(0.0000, 1.9664, -0.0000),
                ],
                intervals: vec![
                    (0, 3, Push, -0.0531),
                    (1, 3, Pull, 0.1176),
                    (2, 4, Pull, 0.1176),
                    (1, 4, Push, -0.0531),
                    (2, 5, Push, -0.0531),
                    (0, 5, Pull, 0.1176),
                ],
                faces: vec![
                    ([3, 4, 5], Apos, Left),
                    ([2, 1, 0], Aneg, Left),
                ],
            },
            BrickName::RightTwist => Brick {
                joints: vec![
                    point3(-0.5000, -0.0000, 0.8660),
                    point3(-0.5000, 0.0000, -0.8660),
                    point3(1.0000, -0.0000, -0.0000),
                    point3(0.8694, 1.9663, -0.4942),
                    point3(-0.0067, 1.9663, 1.0000),
                    point3(-0.8627, 1.9663, -0.5058),
                    point3(-0.0000, -0.0001, -0.0000),
                    point3(-0.0000, 1.9664, -0.0000),
                ],
                intervals: vec![
                    (0, 4, Pull, 0.1176),
                    (2, 5, Push, -0.0531),
                    (2, 3, Pull, 0.1176),
                    (1, 5, Pull, 0.1176),
                    (1, 4, Push, -0.0531),
                    (0, 3, Push, -0.0531),
                ],
                faces: vec![
                    ([2, 1, 0], Aneg, Right),
                    ([3, 4, 5], Apos, Right),
                ],
            },
            BrickName::LeftOmniTwist => Brick {
                joints: vec![
                    point3(-0.0000, 0.0001, 0.0000),
                    point3(1.0000, 0.0000, 0.0000),
                    point3(-0.5000, 0.0000, -0.8660),
                    point3(-0.5000, 0.0000, 0.8660),
                    point3(-0.0048, 1.6290, -1.1518),
                    point3(-1.0048, 1.6330, -1.1464),
                    point3(0.5000, 2.4436, -0.8660),
                    point3(0.4904, 0.8106, -1.4433),
                    point3(-0.9951, 1.6290, 0.5800),
                    point3(-0.4904, 1.6330, 1.4433),
                    point3(-1.4952, 0.8106, 0.2970),
                    point3(-1.0000, 2.4436, 0.0000),
                    point3(0.9999, 1.6290, 0.5717),
                    point3(1.4952, 1.6330, -0.2970),
                    point3(0.5000, 2.4436, 0.8660),
                    point3(1.0048, 0.8106, 1.1464),
                    point3(0.0000, 2.4434, 0.0000),
                    point3(0.0048, 0.8146, 1.1518),
                    point3(0.9951, 0.8146, -0.5800),
                    point3(-0.9999, 0.8146, -0.5717),
                ],
                intervals: vec![
                    (3, 13, Push, -0.0473),
                    (7, 14, Push, -0.0473),
                    (1, 5, Push, -0.0473),
                    (11, 15, Push, -0.0473),
                    (6, 10, Push, -0.0473),
                    (2, 9, Push, -0.0473),
                ],
                faces: vec![
                    ([13, 15, 14], Dneg, Right),
                    ([5, 7, 6], Bneg, Right),
                    ([13, 7, 1], Cpos, Left),
                    ([9, 10, 11], Cneg, Right),
                    ([1, 2, 3], Aneg, Right),
                    ([9, 15, 3], Bpos, Left),
                    ([10, 2, 5], Dpos, Left),
                    ([14, 11, 6], Apos, Left),
                ],
            },
            BrickName::RightOmniTwist => Brick {
                joints: vec![
                    point3(0.0000, 0.0002, 0.0000),
                    point3(1.0000, 0.0000, -0.0000),
                    point3(-0.5000, 0.0000, -0.8660),
                    point3(-0.5000, 0.0000, 0.8660),
                    point3(-0.0048, 1.6290, 1.1518),
                    point3(-1.0048, 1.6330, 1.1464),
                    point3(0.4904, 0.8106, 1.4433),
                    point3(0.5000, 2.4436, 0.8660),
                    point3(0.9999, 1.6290, -0.5717),
                    point3(1.4952, 1.6330, 0.2970),
                    point3(0.5000, 2.4436, -0.8660),
                    point3(1.0048, 0.8106, -1.1464),
                    point3(-0.9951, 1.6290, -0.5800),
                    point3(-0.4904, 1.6330, -1.4433),
                    point3(-1.4952, 0.8106, -0.2970),
                    point3(-1.0000, 2.4436, -0.0000),
                    point3(0.0000, 2.4434, -0.0000),
                    point3(0.0048, 0.8146, -1.1518),
                    point3(-0.9999, 0.8146, 0.5717),
                    point3(0.9951, 0.8146, 0.5800),
                ],
                intervals: vec![
                    (1, 5, Push, -0.0473),
                    (3, 13, Push, -0.0473),
                    (2, 9, Push, -0.0473),
                    (11, 15, Push, -0.0473),
                    (7, 14, Push, -0.0473),
                    (6, 10, Push, -0.0473),
                ],
                faces: vec![
                    ([1, 2, 3], Aneg, Left),
                    ([5, 7, 6], Bneg, Left),
                    ([6, 9, 1], Dpos, Right),
                    ([3, 14, 5], Cpos, Right),
                    ([13, 15, 14], Dneg, Left),
                    ([9, 10, 11], Cneg, Left),
                    ([2, 11, 13], Bpos, Right),
                    ([7, 15, 10], Apos, Right),
                ],
            },
            BrickName::LeftMitosis => unimplemented!(),
            BrickName::RightMitosis => unimplemented!(),
        }
    }
}
