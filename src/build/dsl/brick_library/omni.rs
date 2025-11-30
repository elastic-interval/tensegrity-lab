use crate::build::dsl::brick::BrickPrototype;
use crate::build::dsl::brick_dsl::*;
use crate::build::dsl::{ScaleMode, Spin};

/// Build the Omni brick prototype
pub fn omni(params: &OmniParams) -> BrickPrototype {
    use BrickName::*;
    use BrickRole::*;
    use FaceName::*;
    use JointName::*;
    use ScaleMode::*;

    proto_scaled(
        OmniBrick,
        [OnSpinLeft, OnSpinRight, Seed(4), Seed(1)],
        [Tetrahedral],
    )
    .pushes_x(
        params.push_lengths.x,
        [(BotAlphaX, BotOmegaX), (TopAlphaX, TopOmegaX)],
    )
    .pushes_y(
        params.push_lengths.y,
        [(BotAlphaY, BotOmegaY), (TopAlphaY, TopOmegaY)],
    )
    .pushes_z(
        params.push_lengths.z,
        [(BotAlphaZ, BotOmegaZ), (TopAlphaZ, TopOmegaZ)],
    )
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
    .with_scale(Tetrahedral, Tetrahedral.small())
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
    .with_scale(Tetrahedral, Tetrahedral.large())
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
    .with_scale(Tetrahedral, Tetrahedral.large())
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
    .with_scale(Tetrahedral, Tetrahedral.large())
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
    .with_scale(Tetrahedral, Tetrahedral.small())
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
    .with_scale(Tetrahedral, Tetrahedral.small())
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
    .with_scale(Tetrahedral, Tetrahedral.small())
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
    .with_scale(Tetrahedral, Tetrahedral.large())
    .build()
}
