use crate::build::dsl::brick::BrickPrototype;
use crate::build::dsl::brick_dsl::*;
use crate::build::dsl::Spin;

/// Build the Torque brick prototype (left-handed).
/// The right-handed baked brick is derived via BakedBrick::mirror().
pub fn torque(params: &TorqueParams) -> BrickPrototype {
    use BrickName::*;
    use BrickRole::*;
    use FaceName::*;
    use JointName::*;

    proto(TorqueSymmetrical, [OnSpinLeft, Seed(4)])
        .pushes_x(
            params.push_lengths.x,
            [
                (LeftFront, LeftBack),
                (MiddleFront, MiddleBack),
                (RightFront, RightBack),
            ],
        )
        .pushes_y(
            params.push_lengths.y,
            [
                (FrontLeftBottom, FrontLeftTop),
                (FrontRightBottom, FrontRightTop),
                (BackLeftBottom, BackLeftTop),
                (BackRightBottom, BackRightTop),
            ],
        )
        .pushes_z(
            params.push_lengths.z,
            [(TopLeft, TopRight), (BottomLeft, BottomRight)],
        )
        .pulls(
            params.pull_length,
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
                Seed(4).calls_it(LeftFrontBottom),
                Seed(4).downwards(),
            ],
            [],
        )
        .face(
            Spin::Right,
            [BottomLeft, LeftBack, BackLeftBottom],
            [
                OnSpinLeft.calls_it(FarC),
                Seed(4).calls_it(LeftBackBottom),
                Seed(4).downwards(),
            ],
            [],
        )
        .face(
            Spin::Left,
            [BottomRight, RightBack, BackRightBottom],
            [
                OnSpinLeft.calls_it(NearC),
                Seed(4).calls_it(RightBackBottom),
                Seed(4).downwards(),
            ],
            [],
        )
        .face(
            Spin::Right,
            [BottomRight, RightFront, FrontRightBottom],
            [
                OnSpinLeft.calls_it(FarA),
                Seed(4).calls_it(RightFrontBottom),
                Seed(4).downwards(),
            ],
            [],
        )
        .face(
            Spin::Left,
            [TopLeft, LeftBack, BackLeftTop],
            [OnSpinLeft.calls_it(NearA), Seed(4).calls_it(LeftBackTop)],
            [],
        )
        .face(
            Spin::Right,
            [TopLeft, LeftFront, FrontLeftTop],
            [OnSpinLeft.calls_it(NearB), Seed(4).calls_it(LeftFrontTop)],
            [],
        )
        .face(
            Spin::Left,
            [TopRight, RightFront, FrontRightTop],
            [OnSpinLeft.calls_it(FarB), Seed(4).calls_it(RightFrontTop)],
            [],
        )
        .face(
            Spin::Right,
            [TopRight, RightBack, BackRightTop],
            [
                OnSpinLeft.calls_it(Attach(Spin::Right)),
                Seed(4).calls_it(RightBackTop),
            ],
            [],
        )
        .build()
}
