use crate::build::dsl::brick::{Axis, BrickPrototype};
use crate::build::dsl::brick_dsl::*;
use crate::build::dsl::Spin;

/// Build the Single brick prototype (left-handed).
/// The right-handed baked brick is derived via BakedBrick::mirror().
pub fn single_left(params: &SingleParams) -> BrickPrototype {
    use BrickName::*;
    use BrickRole::*;
    use FaceName::*;
    use JointName::*;

    proto(SingleTwistLeft, [Seed(1), OnSpin(Spin::Left)])
        .pushes_x(params.push_lengths.x, [(AlphaX, OmegaX)])
        .pushes_y(params.push_lengths.y, [(AlphaY, OmegaY)])
        .pushes_z(params.push_lengths.z, [(AlphaZ, OmegaZ)])
        .pulls(
            params.pull_length,
            [(AlphaX, OmegaY), (AlphaY, OmegaZ), (AlphaZ, OmegaX)],
        )
        .face(
            Spin::Left,
            [AlphaX, AlphaY, AlphaZ],
            [
                OnSpin(Spin::Left).calls_it(Attach(Spin::Left)),
                Seed(1).calls_it(SingleBot),
                Seed(1).downwards(),
            ],
            [],
        )
        .face(
            Spin::Left,
            [OmegaZ, OmegaY, OmegaX],
            [
                OnSpin(Spin::Left).calls_it(SingleTop),
                OnSpin(Spin::Left).calls_it(AttachNext),
                Seed(1).calls_it(SingleTop),
            ],
            [],
        )
        // Under Seed(1) the brick has 3-fold cyclic symmetry over its
        // three push axes (X → Y → Z) — the three pulls (AlphaX:OmegaY,
        // AlphaY:OmegaZ, AlphaZ:OmegaX) cycle accordingly. The Oven's
        // `symmetrize_brick_3fold` uses this declaration to know it can
        // orbit-average the joint positions.
        .cyclic_axes_for(Seed(1), [Axis::X, Axis::Y, Axis::Z])
        .build()
}
