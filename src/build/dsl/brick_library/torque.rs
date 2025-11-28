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
        .baked(1.0177)
        .joints([
            (-1.5062, 1.5013, -2.2217),
            (1.5062, 1.5013, -2.2217),
            (-1.4999, 1.5013, 0.0000),
            (1.4999, 1.5013, 0.0000),
            (-1.5062, 1.5013, 2.2217),
            (1.5062, 1.5013, 2.2217),
            (-1.0600, 0.0000, -1.4016),
            (-1.0600, 3.0027, -1.4017),
            (-1.0600, 0.0000, 1.4016),
            (-1.0600, 3.0027, 1.4017),
            (1.0600, 0.0000, -1.4016),
            (1.0600, 3.0027, -1.4017),
            (1.0600, 0.0000, 1.4016),
            (1.0600, 3.0027, 1.4017),
            (-0.0000, 2.3190, -2.9884),
            (-0.0000, 2.3189, 2.9884),
            (-0.0000, 0.6837, -2.9884),
            (-0.0000, 0.6837, 2.9883),
        ])
        .pushes([
            (0, 1, -0.0088),
            (2, 3, -0.0130),
            (4, 5, -0.0088),
            (6, 7, -0.0121),
            (8, 9, -0.0121),
            (10, 11, -0.0121),
            (12, 13, -0.0121),
            (14, 15, -0.0185),
            (16, 17, -0.0185),
        ])
        .pulls([
            (2, 6, 0.1121),
            (2, 7, 0.1121),
            (2, 8, 0.1121),
            (2, 9, 0.1121),
            (3, 10, 0.1121),
            (3, 11, 0.1121),
            (3, 12, 0.1121),
            (3, 13, 0.1121),
        ])
        .build()
}
