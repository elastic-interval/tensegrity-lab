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
    .baked(0.9680)
        .joints([
            (-1.5604, 1.5679, -0.7309),
            (1.5576, 1.5631, -0.8268),
            (-1.5581, 1.5705, 0.8277),
            (1.5595, 1.5608, 0.7300),
            (-0.7755, 0.0102, 0.0212),
            (-0.7797, 3.1312, 0.0299),
            (0.7802, 0.0000, -0.0300),
            (0.7751, 3.1205, -0.0209),
            (-0.0234, 0.7869, -1.5659),
            (0.0278, 0.7865, 1.5552),
            (-0.0296, 2.3453, -1.5555),
            (0.0261, 2.3431, 1.5656),
        ])
        .pushes([
            (0, 1, -0.0164),
            (2, 3, -0.0165),
            (4, 5, -0.0158),
            (6, 7, -0.0160),
            (8, 9, -0.0157),
            (10, 11, -0.0157),
        ])
        .pulls([])
        .build()
}
