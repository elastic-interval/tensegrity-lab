use crate::build::dsl::brick::{Axis, BrickPrototype};
use crate::build::dsl::brick_dsl::*;
use crate::build::dsl::{ScaleMode, Spin};

/// Build the Omni brick prototype (left-handed).
/// The right-handed baked brick is derived via BakedBrick::mirror().
pub fn omni(params: &OmniParams) -> BrickPrototype {
    use BrickName::*;
    use BrickRole::*;
    use FaceName::*;
    use JointName::*;
    use ScaleMode::*;

    proto_scaled(
        OmniSymmetrical,
        [OnSpin(Spin::Right), Seed(4), Seed(2), Seed(1)],
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
            OnSpin(Spin::Right).calls_it(Attach(Spin::Right)),
            Seed(4).calls_it(RightFrontTop),
            Seed(2).calls_it(UpperRight),
            Seed(1).calls_it(OmniTop),
        ],
        [Tetrahedral.large()],
    )
    .face(
        Spin::Left,
        [TopOmegaX, TopAlphaY, BotOmegaZ],
        [
            OnSpin(Spin::Right).calls_it(OmniBotX),
            Seed(4).calls_it(RightFrontBottom),
            Seed(4).downwards(),
            Seed(2).calls_it(ForeRight),
            Seed(1).calls_it(OmniTopX),
        ],
        [Tetrahedral.small()],
    )
    .face(
        Spin::Left,
        [TopOmegaY, TopAlphaZ, BotOmegaX],
        [
            OnSpin(Spin::Right).calls_it(OmniBotY),
            Seed(4).calls_it(RightBackTop),
            Seed(2).calls_it(AftRight),
            Seed(1).calls_it(OmniTopY),
        ],
        [Tetrahedral.small()],
    )
    .face(
        Spin::Left,
        [TopOmegaZ, TopAlphaX, BotOmegaY],
        [
            OnSpin(Spin::Right).calls_it(OmniBotZ),
            Seed(4).calls_it(LeftFrontTop),
            Seed(2).calls_it(UpperLeft),
            Seed(1).calls_it(OmniTopZ),
        ],
        [Tetrahedral.small()],
    )
    .face(
        Spin::Right,
        [BotAlphaZ, BotOmegaX, TopAlphaY],
        [
            OnSpin(Spin::Right).calls_it(OmniTopZ),
            Seed(4).calls_it(RightBackBottom),
            Seed(4).downwards(),
            Seed(2).calls_it(LowerRight),
            Seed(2).downwards(),
            Seed(1).calls_it(OmniBotZ),
        ],
        [Tetrahedral.large()],
    )
    .face(
        Spin::Right,
        [BotAlphaY, BotOmegaZ, TopAlphaX],
        [
            OnSpin(Spin::Right).calls_it(OmniTopY),
            Seed(4).calls_it(LeftFrontBottom),
            Seed(4).downwards(),
            Seed(2).calls_it(ForeLeft),
            Seed(1).calls_it(OmniBotY),
        ],
        [Tetrahedral.large()],
    )
    .face(
        Spin::Right,
        [BotAlphaX, BotOmegaY, TopAlphaZ],
        [
            OnSpin(Spin::Right).calls_it(OmniTopX),
            Seed(4).calls_it(LeftBackTop),
            Seed(2).calls_it(AftLeft),
            Seed(1).calls_it(OmniBotX),
        ],
        [Tetrahedral.large()],
    )
    .face(
        Spin::Left,
        [BotAlphaX, BotAlphaY, BotAlphaZ],
        [
            OnSpin(Spin::Right).calls_it(OmniBot),
            Seed(4).calls_it(LeftBackBottom),
            Seed(4).downwards(),
            Seed(2).calls_it(LowerLeft),
            Seed(2).downwards(),
            Seed(1).calls_it(OmniBot),
            Seed(1).downwards(),
        ],
        [Tetrahedral.small()],
    )
    // Under Seed(1) the brick exhibits 3-fold cyclic symmetry over its three
    // axes in the order X → Y → Z. Seed joints with axis X, Y, Z get symbolic
    // letters A, B, C respectively in their display names.
    .cyclic_axes_for(Seed(1), [Axis::X, Axis::Y, Axis::Z])
    // Per-face twist (0/1/2 of a 3-fold cycle) declares the rotational
    // offset of bricks attached off each labeled "right" face relative
    // to its mirror-paired "left" face. Lets the labeller produce
    // mirror-partner joint labels that share their numeric suffix.
    // Empirically determined to make HeadlessHug's mirror partners line
    // up; left faces stay at twist 0 by default.
    .face_twist(LowerRight, 2)
    .face_twist(UpperRight, 1)
    .face_twist(ForeRight, 1)
    .face_twist(AftRight, 2)
    .build()
}
