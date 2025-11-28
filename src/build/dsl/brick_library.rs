use crate::build::dsl::brick::{BakedBrick, Brick, Prototype};
use crate::build::dsl::brick_dsl::*;
use crate::build::dsl::Spin;
use cgmath::SquareMatrix;
use std::sync::OnceLock;

static SINGLE_RIGHT: OnceLock<Brick> = OnceLock::new();
static SINGLE_LEFT: OnceLock<Brick> = OnceLock::new();
static OMNI: OnceLock<Brick> = OnceLock::new();
static TORQUE: OnceLock<Brick> = OnceLock::new();

/// Build the Single-right brick
pub fn single_right() -> Brick {
    use BrickName::*;
    use BrickRole::*;
    use FaceName::*;
    use JointName::*;

    proto(SingleRightBrick, [Seed(1), OnSpinRight])
        .pushes_x(3.204, [(AlphaX, OmegaX)])
        .pushes_y(3.204, [(AlphaY, OmegaY)])
        .pushes_z(3.204, [(AlphaZ, OmegaZ)])
        .pulls(2.0, [(AlphaX, OmegaZ), (AlphaY, OmegaX), (AlphaZ, OmegaY)])
        .face(
            Spin::Right,
            [AlphaZ, AlphaY, AlphaX],
            [
                OnSpinRight.calls_it(Attach(Spin::Right)),
                Seed(1).calls_it(SingleBot),
                Seed(1).downwards(),
            ],
        )
        .face(
            Spin::Right,
            [OmegaX, OmegaY, OmegaZ],
            [
                OnSpinRight.calls_it(SingleTop),
                OnSpinRight.calls_it(AttachNext),
                Seed(1).calls_it(SingleTop),
            ],
        )
        .baked()
        .joints([
            (-0.9920, 0.0001, 0.5538),
            (1.1358, 2.2411, 0.0132),
            (0.9737, 0.0001, 0.5813),
            (-0.5536, 2.2415, -0.9915),
            (0.0164, 0.0000, -1.1337),
            (-0.5810, 2.2408, 0.9737),
        ])
        .pushes([
            (0, 1, -0.0224),
            (2, 3, -0.0229),
            (4, 5, -0.0235),
        ])
        .pulls([
            (0, 5, 0.1548),
            (2, 1, 0.1554),
            (4, 3, 0.1556),
        ])
        .build()
}

/// Build the Single-left brick
pub fn single_left() -> Brick {
    use BrickName::*;
    use BrickRole::*;
    use FaceName::*;
    use JointName::*;

    proto(SingleLeftBrick, [Seed(1), OnSpinLeft])
        .pushes_x(3.204, [(AlphaX, OmegaX)])
        .pushes_y(3.204, [(AlphaY, OmegaY)])
        .pushes_z(3.204, [(AlphaZ, OmegaZ)])
        .pulls(2.0, [(AlphaX, OmegaY), (AlphaY, OmegaZ), (AlphaZ, OmegaX)])
        .face(
            Spin::Left,
            [AlphaX, AlphaY, AlphaZ],
            [
                OnSpinLeft.calls_it(Attach(Spin::Left)),
                Seed(1).calls_it(SingleBot),
                Seed(1).downwards(),
            ],
        )
        .face(
            Spin::Left,
            [OmegaZ, OmegaY, OmegaX],
            [
                OnSpinLeft.calls_it(SingleTop),
                OnSpinLeft.calls_it(AttachNext),
                Seed(1).calls_it(SingleTop),
            ],
        )
        .baked()
        .joints([
            (-1.1357, 0.0000, -0.0129),
            (0.9921, 2.2410, -0.5540),
            (0.5533, 0.0000, 0.9923),
            (-0.9736, 2.2407, -0.5821),
            (0.5815, 0.0003, -0.9730),
            (-0.0170, 2.2416, 1.1330),
        ])
        .pushes([
            (0, 1, -0.0223),
            (2, 3, -0.0229),
            (4, 5, -0.0236),
        ])
        .pulls([
            (0, 3, 0.1554),
            (2, 5, 0.1556),
            (4, 1, 0.1547),
        ])
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
        [OnSpinLeft, OnSpinRight, Seed(4), Seed(1)],
    )
    .pushes_x(3.271, [(BotAlphaX, BotOmegaX), (TopAlphaX, TopOmegaX)])
    .pushes_y(3.271, [(BotAlphaY, BotOmegaY), (TopAlphaY, TopOmegaY)])
    .pushes_z(3.271, [(BotAlphaZ, BotOmegaZ), (TopAlphaZ, TopOmegaZ)])
    .face(
        Spin::Right,
        [TopOmegaX, TopOmegaY, TopOmegaZ],
        [
            OnSpinLeft.calls_it(Attach(Spin::Right)),
            OnSpinRight.calls_it(OmniTop),
            Seed(4).calls_it(RightFrontTop),
            Seed(1).calls_it(OmniTop),
        ],
    )
    .face(
        Spin::Left,
        [TopOmegaX, TopAlphaY, BotOmegaZ],
        [
            OnSpinLeft.calls_it(OmniTopX),
            OnSpinRight.calls_it(OmniBotX),
            Seed(4).calls_it(RightFrontBottom),
            Seed(4).downwards(),
            Seed(1).calls_it(OmniTopX),
        ],
    )
    .face(
        Spin::Left,
        [TopOmegaY, TopAlphaZ, BotOmegaX],
        [
            OnSpinLeft.calls_it(OmniTopY),
            OnSpinRight.calls_it(OmniBotY),
            Seed(4).calls_it(RightBackTop),
            Seed(1).calls_it(OmniTopY),
        ],
    )
    .face(
        Spin::Left,
        [TopOmegaZ, TopAlphaX, BotOmegaY],
        [
            OnSpinLeft.calls_it(OmniTopZ),
            OnSpinRight.calls_it(OmniBotZ),
            Seed(4).calls_it(LeftFrontTop),
            Seed(1).calls_it(OmniTopZ),
        ],
    )
    .face(
        Spin::Right,
        [BotAlphaZ, BotOmegaX, TopAlphaY],
        [
            OnSpinLeft.calls_it(OmniBotZ),
            OnSpinRight.calls_it(OmniTopZ),
            Seed(4).calls_it(RightBackBottom),
            Seed(4).downwards(),
            Seed(1).calls_it(OmniBotZ),
        ],
    )
    .face(
        Spin::Right,
        [BotAlphaY, BotOmegaZ, TopAlphaX],
        [
            OnSpinLeft.calls_it(OmniBotY),
            OnSpinRight.calls_it(OmniTopY),
            Seed(4).calls_it(LeftFrontBottom),
            Seed(4).downwards(),
            Seed(1).calls_it(OmniBotY),
        ],
    )
    .face(
        Spin::Right,
        [BotAlphaX, BotOmegaY, TopAlphaZ],
        [
            OnSpinLeft.calls_it(OmniBotX),
            OnSpinRight.calls_it(OmniTopX),
            Seed(4).calls_it(LeftBackTop),
            Seed(1).calls_it(OmniBotX),
        ],
    )
    .face(
        Spin::Left,
        [BotAlphaX, BotAlphaY, BotAlphaZ],
        [
            OnSpinLeft.calls_it(OmniBot),
            OnSpinRight.calls_it(Attach(Spin::Left)),
            Seed(4).calls_it(LeftBackBottom),
            Seed(4).downwards(),
            Seed(1).calls_it(OmniBot),
            Seed(1).downwards(),
        ],
    )
    .baked()
        .joints([
            (-1.5972, 1.6101, -0.7979),
            (1.6012, 1.6274, -0.8032),
            (-1.6012, 1.6104, 0.8032),
            (1.5972, 1.6273, 0.7979),
            (-0.8141, 0.0000, -0.0007),
            (-0.7867, 3.1969, 0.0040),
            (0.7866, 0.0408, -0.0046),
            (0.8143, 3.2376, 0.0014),
            (-0.0083, 0.8195, -1.6032),
            (-0.0088, 0.8188, 1.5971),
            (0.0093, 2.4189, -1.5972),
            (0.0078, 2.4180, 1.6031),
        ])
        .pushes([
            (0, 1, -0.0174),
            (2, 3, -0.0175),
            (4, 5, -0.0178),
            (6, 7, -0.0179),
            (8, 9, -0.0169),
            (10, 11, -0.0169),
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

    proto(TorqueBrick, [OnSpinLeft, OnSpinRight, Seed(4)])
        .pushes_x(
            3.0,
            [
                (LeftFront, LeftBack),
                (MiddleFront, MiddleBack),
                (RightFront, RightBack),
            ],
        )
        .pushes_y(
            3.0,
            [
                (FrontLeftBottom, FrontLeftTop),
                (FrontRightBottom, FrontRightTop),
                (BackLeftBottom, BackLeftTop),
                (BackRightBottom, BackRightTop),
            ],
        )
        .pushes_z(6.0, [(TopLeft, TopRight), (BottomLeft, BottomRight)])
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

fn get_brick_definition(brick_name: BrickName) -> &'static Brick {
    match brick_name {
        BrickName::SingleLeftBrick => SINGLE_LEFT.get_or_init(single_left),
        BrickName::SingleRightBrick => SINGLE_RIGHT.get_or_init(single_right),
        BrickName::OmniBrick => OMNI.get_or_init(omni),
        BrickName::TorqueBrick => TORQUE.get_or_init(torque),
    }
}

pub fn get_prototype(brick_name: BrickName) -> Prototype {
    get_brick_definition(brick_name).prototype.clone()
}

pub fn get_brick(brick_name: BrickName, brick_role: BrickRole) -> BakedBrick {
    let brick = get_brick_definition(brick_name);
    let mut baked = brick.baked.clone();
    for face in &mut baked.faces {
        face.aliases.retain(|alias| alias.brick_role == brick_role);
    }
    let space = match brick_role {
        BrickRole::Seed(_) => baked.down_rotation(brick_role),
        BrickRole::OnSpinLeft | BrickRole::OnSpinRight => {
            let face = baked
                .faces
                .iter()
                .find(|face| {
                    face.aliases.iter().any(|alias| {
                        alias.brick_role == brick_role
                            && matches!(alias.face_name, FaceName::Attach(_))
                    })
                })
                .expect("Brick does not have any face aliases for this role");
            face.vector_space(&baked).invert().unwrap()
        }
    };
    baked.apply_matrix(space);
    baked
}
