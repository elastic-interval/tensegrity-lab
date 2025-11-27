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

    proto(SingleRightBrick, [Seed, OnSpinRight])
        .pushes_x(3.204, [(AlphaX, OmegaX)])
        .pushes_y(3.204, [(AlphaY, OmegaY)])
        .pushes_z(3.204, [(AlphaZ, OmegaZ)])
        .pulls(2.0, [(AlphaX, OmegaZ), (AlphaY, OmegaX), (AlphaZ, OmegaY)])
        .face(
            Spin::Right,
            [AlphaZ, AlphaY, AlphaX],
            [
                OnSpinRight.calls_it(Attach(Spin::Right)),
                Seed.calls_it(SingleBot),
                Seed.calls_it(Downwards),
            ],
        )
        .face(
            Spin::Right,
            [OmegaX, OmegaY, OmegaZ],
            [
                OnSpinRight.calls_it(SingleTop),
                OnSpinRight.calls_it(AttachNext),
                Seed.calls_it(SingleTop),
            ],
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
        .pushes_x(3.204, [(AlphaX, OmegaX)])
        .pushes_y(3.204, [(AlphaY, OmegaY)])
        .pushes_z(3.204, [(AlphaZ, OmegaZ)])
        .pulls(2.0, [(AlphaX, OmegaY), (AlphaY, OmegaZ), (AlphaZ, OmegaX)])
        .face(
            Spin::Left,
            [AlphaX, AlphaY, AlphaZ],
            [
                OnSpinLeft.calls_it(Attach(Spin::Left)),
                Seed.calls_it(SingleBot),
                Seed.calls_it(Downwards),
            ],
        )
        .face(
            Spin::Left,
            [OmegaZ, OmegaY, OmegaX],
            [
                OnSpinLeft.calls_it(SingleTop),
                OnSpinLeft.calls_it(AttachNext),
                Seed.calls_it(SingleTop),
            ],
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
    .pushes_x(3.271, [(BotAlphaX, BotOmegaX), (TopAlphaX, TopOmegaX)])
    .pushes_y(3.271, [(BotAlphaY, BotOmegaY), (TopAlphaY, TopOmegaY)])
    .pushes_z(3.271, [(BotAlphaZ, BotOmegaZ), (TopAlphaZ, TopOmegaZ)])
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
            SeedFaceDown.calls_it(OmniBotZ),
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
            SeedFaceDown.calls_it(Downwards),
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
        .pushes_x(3.0, [
            (LeftFront, LeftBack),
            (MiddleFront, MiddleBack),
            (RightFront, RightBack),
        ])
        .pushes_y(3.0, [
            (FrontLeftBottom, FrontLeftTop),
            (FrontRightBottom, FrontRightTop),
            (BackLeftBottom, BackLeftTop),
            (BackRightBottom, BackRightTop),
        ])
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
    let space = if brick_role.is_seed() {
        baked.down_rotation(brick_role)
    } else {
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
    };
    baked.apply_matrix(space);
    baked
}
