use cgmath::Point3;
use clap::ValueEnum;

use crate::build::tenscript::{FaceName, Spin};
use crate::build::tenscript::FaceName::{*};
use crate::build::tenscript::Spin::{Left, Right};
use crate::fabric::interval::Role;
use crate::fabric::interval::Role::{Pull, Push};

#[derive(Debug, Clone)]
pub struct Brick {
    pub joints: Vec<Point3<f32>>,
    pub intervals: Vec<(usize, usize, Role)>,
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
            BrickName::RightTwist => Brick {
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
            BrickName::LeftOmniTwist => Brick {
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
}
