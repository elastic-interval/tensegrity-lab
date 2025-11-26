use crate::build::dsl::brick::Brick;
use crate::build::dsl::brick_dsl::*;
use crate::build::dsl::Spin;

/// Build the Single-right brick
pub fn single_right() -> Brick {
    use BrickName::*;
    use BrickRole::*;
    use FaceName::*;
    use JointName::*;

    proto(SingleRightBrick, [Seed, OnSpinRight])
        .pushes(
            3.204,
            [(AlphaX, OmegaX), (AlphaY, OmegaY), (AlphaZ, OmegaZ)],
        )
        .pulls(2.0, [(AlphaX, OmegaZ), (AlphaY, OmegaX), (AlphaZ, OmegaY)])
        .face(
            Spin::Right,
            [AlphaZ, AlphaY, AlphaX],
            [
                OnSpinRight.calls_it(Attach(Spin::Right)),
                Seed.calls_it(SingleBot),
            ],
        )
        .face(
            Spin::Right,
            [OmegaX, OmegaY, OmegaZ],
            [OnSpinRight.calls_it(SingleTop), Seed.calls_it(SingleTop)],
        )
        .baked()
        .joints([
            (-1.4913, -0.3875, 0.0099),
            (1.4913, -0.0099, 0.3875),
            (0.0099, -1.4913, -0.3875),
            (0.3875, 1.4913, -0.0099),
            (-0.3875, 0.0099, -1.4913),
            (-0.0099, 0.3875, 1.4913),
        ])
        .pushes([(2, 3, -0.0531), (4, 5, -0.0531), (0, 1, -0.0531)])
        .pulls([(2, 1, 0.1171), (0, 5, 0.1171), (4, 3, 0.1171)])
        // Faces are derived from proto on-demand via BrickDefinition::baked_faces()
        .build()
}

/// Build the Single-left brick
pub fn single_left() -> Brick {
    use BrickName::*;
    use BrickRole::*;
    use FaceName::*;
    use JointName::*;

    proto(SingleLeftBrick, [Seed, OnSpinLeft])
        .pushes(
            3.204,
            [(AlphaX, OmegaX), (AlphaY, OmegaY), (AlphaZ, OmegaZ)],
        )
        .pulls(2.0, [(AlphaX, OmegaY), (AlphaY, OmegaZ), (AlphaZ, OmegaX)])
        .face(
            Spin::Left,
            [AlphaX, AlphaY, AlphaZ],
            [
                OnSpinLeft.calls_it(Attach(Spin::Left)),
                Seed.calls_it(SingleBot),
            ],
        )
        .face(
            Spin::Left,
            [OmegaZ, OmegaY, OmegaX],
            [OnSpinLeft.calls_it(SingleTop), Seed.calls_it(SingleTop)],
        )
        .baked()
        .joints([
            (-1.4913, 0.0099, -0.3875),
            (1.4913, 0.3875, -0.0099),
            (-0.3875, -1.4913, 0.0099),
            (-0.0099, 1.4913, 0.3875),
            (0.0099, -0.3875, -1.4913),
            (0.3875, -0.0099, 1.4913),
        ])
        .pushes([(0, 1, -0.0531), (4, 5, -0.0531), (2, 3, -0.0531)])
        .pulls([(4, 1, 0.1171), (2, 5, 0.1171), (0, 3, 0.1171)])
        // Faces are derived from proto on-demand via BrickDefinition::baked_faces()
        .build()
}

/// Build the Omni brick
pub fn omni() -> Brick {
    use BrickName::*;
    use BrickRole::*;
    use FaceName::*;
    use JointName::*;

    proto(
        OmniBrick,
        [OnSpinLeft, OnSpinRight, SeedFourDown, SeedFaceDown],
    )
    .pushes(
        3.271,
        [
            (BotAlphaX, BotOmegaX),
            (TopAlphaX, TopOmegaX),
            (BotAlphaY, BotOmegaY),
            (TopAlphaY, TopOmegaY),
            (BotAlphaZ, BotOmegaZ),
            (TopAlphaZ, TopOmegaZ),
        ],
    )
    .face(
        Spin::Right,
        [TopOmegaX, TopOmegaY, TopOmegaZ],
        [
            OnSpinLeft.calls_it(Attach(Spin::Right)),
            OnSpinRight.calls_it(OmniTop),
            SeedFourDown.calls_it(RightFrontTop),
            SeedFaceDown.calls_it(OmniTop),
        ],
    )
    .face(
        Spin::Left,
        [TopOmegaX, TopAlphaY, BotOmegaZ],
        [
            OnSpinLeft.calls_it(OmniTopX),
            OnSpinRight.calls_it(OmniBotX),
            SeedFourDown.calls_it(RightFrontBottom),
            SeedFaceDown.calls_it(OmniTopX),
        ],
    )
    .face(
        Spin::Left,
        [TopOmegaY, TopAlphaZ, BotOmegaX],
        [
            OnSpinLeft.calls_it(OmniTopY),
            OnSpinRight.calls_it(OmniBotY),
            SeedFourDown.calls_it(RightBackTop),
            SeedFaceDown.calls_it(OmniTopY),
        ],
    )
    .face(
        Spin::Left,
        [TopOmegaZ, TopAlphaX, BotOmegaY],
        [
            OnSpinLeft.calls_it(OmniTopZ),
            OnSpinRight.calls_it(OmniBotZ),
            SeedFourDown.calls_it(LeftFrontTop),
            SeedFaceDown.calls_it(OmniTopZ),
        ],
    )
    .face(
        Spin::Right,
        [BotAlphaZ, BotOmegaX, TopAlphaY],
        [
            OnSpinLeft.calls_it(OmniBotZ),
            OnSpinRight.calls_it(OmniTopZ),
            SeedFourDown.calls_it(RightBackBottom),
            SeedFaceDown.calls_it(OmniTopZ),
        ],
    )
    .face(
        Spin::Right,
        [BotAlphaY, BotOmegaZ, TopAlphaX],
        [
            OnSpinLeft.calls_it(OmniBotY),
            OnSpinRight.calls_it(OmniTopY),
            SeedFourDown.calls_it(LeftFrontBottom),
            SeedFaceDown.calls_it(OmniBotY),
        ],
    )
    .face(
        Spin::Right,
        [BotAlphaX, BotOmegaY, TopAlphaZ],
        [
            OnSpinLeft.calls_it(OmniBotX),
            OnSpinRight.calls_it(OmniTopX),
            SeedFourDown.calls_it(LeftBackTop),
            SeedFaceDown.calls_it(OmniBotX),
        ],
    )
    .face(
        Spin::Left,
        [BotAlphaX, BotAlphaY, BotAlphaZ],
        [
            OnSpinLeft.calls_it(OmniBot),
            OnSpinRight.calls_it(Attach(Spin::Left)),
            SeedFourDown.calls_it(LeftBackBottom),
            SeedFaceDown.calls_it(OmniBot),
        ],
    )
    .baked()
    .joints([
        (-1.5556, -0.0000, -0.7722),
        (1.5556, 0.0000, -0.7722),
        (-1.5556, 0.0000, 0.7722),
        (1.5556, -0.0000, 0.7722),
        (-0.7722, -1.5556, 0.0000),
        (-0.7722, 1.5556, -0.0000),
        (0.7722, -1.5556, -0.0000),
        (0.7722, 1.5556, -0.0000),
        (-0.0000, -0.7722, -1.5556),
        (-0.0000, -0.7722, 1.5556),
        (-0.0000, 0.7722, -1.5556),
        (-0.0000, 0.7722, 1.5556),
    ])
    .pushes([
        (2, 3, -0.0473),
        (4, 5, -0.0473),
        (6, 7, -0.0473),
        (0, 1, -0.0473),
        (8, 9, -0.0473),
        (10, 11, -0.0473),
    ])
    .pulls([])
    .build()
}

/// Build the Torque brick
pub fn torque() -> Brick {
    use BrickName::*;
    use BrickRole::*;
    use FaceName::*;
    use JointName::*;

    proto(TorqueBrick, [OnSpinLeft, OnSpinRight, SeedFourDown])
        .pushes(
            3.0,
            [
                (LeftFront, LeftBack),
                (MiddleFront, MiddleBack),
                (RightFront, RightBack),
            ],
        )
        .pushes(
            3.0,
            [
                (FrontLeftBottom, FrontLeftTop),
                (FrontRightBottom, FrontRightTop),
                (BackLeftBottom, BackLeftTop),
                (BackRightBottom, BackRightTop),
            ],
        )
        .pushes(6.0, [(TopLeft, TopRight), (BottomLeft, BottomRight)])
        .pulls(
            1.86,
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
                SeedFourDown.calls_it(LeftFrontBottom),
            ],
        )
        .face(
            Spin::Right,
            [BottomLeft, LeftBack, BackLeftBottom],
            [
                OnSpinLeft.calls_it(FarC),
                OnSpinRight.calls_it(NearC),
                SeedFourDown.calls_it(LeftBackBottom),
            ],
        )
        .face(
            Spin::Left,
            [BottomRight, RightBack, BackRightBottom],
            [
                OnSpinLeft.calls_it(NearC),
                OnSpinRight.calls_it(FarC),
                SeedFourDown.calls_it(RightBackBottom),
            ],
        )
        .face(
            Spin::Right,
            [BottomRight, RightFront, FrontRightBottom],
            [
                OnSpinRight.calls_it(FarA),
                SeedFourDown.calls_it(RightFrontBottom),
            ],
        )
        .face(
            Spin::Left,
            [TopLeft, LeftBack, BackLeftTop],
            [
                OnSpinLeft.calls_it(NearA),
                OnSpinRight.calls_it(FarA),
                SeedFourDown.calls_it(LeftBackTop),
            ],
        )
        .face(
            Spin::Right,
            [TopLeft, LeftFront, FrontLeftTop],
            [
                OnSpinLeft.calls_it(NearB),
                OnSpinRight.calls_it(FarB),
                SeedFourDown.calls_it(LeftFrontTop),
            ],
        )
        .face(
            Spin::Left,
            [TopRight, RightFront, FrontRightTop],
            [
                OnSpinLeft.calls_it(FarC),
                OnSpinRight.calls_it(NearC),
                SeedFourDown.calls_it(RightFrontTop),
            ],
        )
        .face(
            Spin::Right,
            [TopRight, RightBack, BackRightTop],
            [
                OnSpinLeft.calls_it(Attach(Spin::Right)),
                OnSpinRight.calls_it(Far),
                SeedFourDown.calls_it(RightBackTop),
            ],
        )
        .baked()
        .joints([
            (-1.4967, 0.0000, -2.2107),
            (1.4967, -0.0000, -2.2106),
            (-1.4968, 0.0000, 0.0000),
            (1.4968, 0.0000, 0.0000),
            (-1.4967, 0.0000, 2.2107),
            (1.4967, 0.0000, 2.2106),
            (-1.0572, -1.4961, -1.3771),
            (-1.0572, 1.4961, -1.3771),
            (-1.0572, -1.4961, 1.3771),
            (-1.0572, 1.4961, 1.3771),
            (1.0572, -1.4961, -1.3771),
            (1.0572, 1.4961, -1.3771),
            (1.0572, -1.4961, 1.3771),
            (1.0572, 1.4961, 1.3771),
            (0.0000, 0.8226, -2.9920),
            (0.0000, 0.8226, 2.9920),
            (-0.0000, -0.8226, -2.9920),
            (0.0000, -0.8226, 2.9920),
        ])
        .pushes([
            (0, 1, -0.0011),
            (4, 5, -0.0011),
            (2, 3, -0.0010),
            (16, 17, -0.0016),
            (10, 11, -0.0015),
            (8, 9, -0.0015),
            (6, 7, -0.0015),
            (12, 13, -0.0015),
            (14, 15, -0.0016),
        ])
        .pulls([
            (2, 9, 0.1189),
            (3, 11, 0.1189),
            (3, 10, 0.1189),
            (2, 8, 0.1189),
            (2, 6, 0.1189),
            (2, 7, 0.1189),
            (3, 13, 0.1189),
            (3, 12, 0.1189),
        ])
        .build()
}

/// Build the complete brick library from Rust code
pub fn build_brick_library() -> Vec<Brick> {
    vec![single_right(), single_left(), omni(), torque()]
}
