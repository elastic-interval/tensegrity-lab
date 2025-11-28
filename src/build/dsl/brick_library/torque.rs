use crate::build::dsl::brick::Brick;
use crate::build::dsl::brick_dsl::*;
use crate::build::dsl::Spin;
use cgmath::Vector3;

/// Build the Torque brick
pub fn torque(push_lengths: Vector3<f32>, pull_length: f32) -> Brick {
    use BrickName::*;
    use BrickRole::*;
    use FaceName::*;
    use JointName::*;

    proto(TorqueBrick, [OnSpinLeft, OnSpinRight, Seed(4)])
        .pushes_x(
            push_lengths.x,
            [
                (LeftFront, LeftBack),
                (MiddleFront, MiddleBack),
                (RightFront, RightBack),
            ],
        )
        .pushes_y(
            push_lengths.y,
            [
                (FrontLeftBottom, FrontLeftTop),
                (FrontRightBottom, FrontRightTop),
                (BackLeftBottom, BackLeftTop),
                (BackRightBottom, BackRightTop),
            ],
        )
        .pushes_z(push_lengths.z, [(TopLeft, TopRight), (BottomLeft, BottomRight)])
        .pulls(
            pull_length,
            [
                (MiddleFront, FrontLeftBottom),
                (MiddleFront, FrontLeftTop),
                (MiddleFront, FrontRightBottom),
                (MiddleFront, FrontRightTop),
                (MiddleBack, BackLeftBottom),
                (MiddleBack, BackLeftTop),
                (MiddleBack, BackRightBottom),
                (MiddleBack, BackRightTop),
            ],
        )
        .face(
            Spin::Left,
            [BottomLeft, LeftFront, FrontLeftBottom],
            [
                OnSpinLeft.calls_it(Far),
                OnSpinRight.calls_it(Attach(Spin::Left)),
                Seed(4).calls_it(LeftFrontBottom),
                Seed(4).downwards(),
            ],
        )
        .face(
            Spin::Right,
            [BottomLeft, LeftBack, BackLeftBottom],
            [
                OnSpinLeft.calls_it(FarC),
                OnSpinRight.calls_it(NearC),
                Seed(4).calls_it(LeftBackBottom),
                Seed(4).downwards(),
            ],
        )
        .face(
            Spin::Left,
            [BottomRight, RightBack, BackRightBottom],
            [
                OnSpinLeft.calls_it(NearC),
                OnSpinRight.calls_it(FarC),
                Seed(4).calls_it(RightBackBottom),
                Seed(4).downwards(),
            ],
        )
        .face(
            Spin::Right,
            [BottomRight, RightFront, FrontRightBottom],
            [
                OnSpinRight.calls_it(FarA),
                Seed(4).calls_it(RightFrontBottom),
                Seed(4).downwards(),
            ],
        )
        .face(
            Spin::Left,
            [TopLeft, LeftBack, BackLeftTop],
            [
                OnSpinLeft.calls_it(NearA),
                OnSpinRight.calls_it(FarA),
                Seed(4).calls_it(LeftBackTop),
            ],
        )
        .face(
            Spin::Right,
            [TopLeft, LeftFront, FrontLeftTop],
            [
                OnSpinLeft.calls_it(NearB),
                OnSpinRight.calls_it(FarB),
                Seed(4).calls_it(LeftFrontTop),
            ],
        )
        .face(
            Spin::Left,
            [TopRight, RightFront, FrontRightTop],
            [
                OnSpinLeft.calls_it(FarC),
                OnSpinRight.calls_it(NearC),
                Seed(4).calls_it(RightFrontTop),
            ],
        )
        .face(
            Spin::Right,
            [TopRight, RightBack, BackRightTop],
            [
                OnSpinLeft.calls_it(Attach(Spin::Right)),
                OnSpinRight.calls_it(Far),
                Seed(4).calls_it(RightBackTop),
            ],
        )
        .baked()
        .joints([
            (-1.4884, 1.4770, -2.1768),
            (1.4884, 1.4770, -2.1768),
            (-1.4761, 1.4774, 0.0000),
            (1.4761, 1.4774, 0.0000),
            (-1.4884, 1.4770, 2.1768),
            (1.4884, 1.4770, 2.1768),
            (-1.0452, 0.0000, -1.3543),
            (-1.0454, 2.9545, -1.3547),
            (-1.0451, 0.0000, 1.3543),
            (-1.0454, 2.9545, 1.3547),
            (1.0452, 0.0000, -1.3543),
            (1.0454, 2.9545, -1.3547),
            (1.0451, 0.0000, 1.3543),
            (1.0454, 2.9545, 1.3547),
            (-0.0000, 2.2856, -2.9443),
            (-0.0000, 2.2856, 2.9443),
            (-0.0000, 0.6685, -2.9443),
            (-0.0000, 0.6685, 2.9443),
        ])
        .pushes([
            (0, 1, -0.0101),
            (2, 3, -0.0117),
            (4, 5, -0.0101),
            (6, 7, -0.0109),
            (8, 9, -0.0109),
            (10, 11, -0.0109),
            (12, 13, -0.0109),
            (14, 15, -0.0160),
            (16, 17, -0.0160),
        ])
        .pulls([
            (2, 6, 0.1054),
            (2, 7, 0.1054),
            (2, 8, 0.1054),
            (2, 9, 0.1054),
            (3, 10, 0.1054),
            (3, 11, 0.1054),
            (3, 12, 0.1054),
            (3, 13, 0.1054),
        ])
        .build()
}
