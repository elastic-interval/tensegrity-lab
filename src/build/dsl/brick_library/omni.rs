use crate::build::dsl::brick::Brick;
use crate::build::dsl::brick_dsl::*;
use crate::build::dsl::Spin;
use cgmath::Vector3;

/// Build the Omni brick
pub fn omni(push_lengths: Vector3<f32>) -> Brick {
    use BrickName::*;
    use BrickRole::*;
    use FaceName::*;
    use JointName::*;

    proto(
        OmniBrick,
        [OnSpinLeft, OnSpinRight, Seed(4), Seed(1)],
    )
    .pushes_x(push_lengths.x, [(BotAlphaX, BotOmegaX), (TopAlphaX, TopOmegaX)])
    .pushes_y(push_lengths.y, [(BotAlphaY, BotOmegaY), (TopAlphaY, TopOmegaY)])
    .pushes_z(push_lengths.z, [(BotAlphaZ, BotOmegaZ), (TopAlphaZ, TopOmegaZ)])
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
