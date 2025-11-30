use crate::build::dsl::brick::BrickPrototype;
use crate::build::dsl::brick_dsl::*;
use crate::build::dsl::Spin;

/// Build the Single-right brick prototype
pub fn single_right(params: &SingleParams) -> BrickPrototype {
    use BrickName::*;
    use BrickRole::*;
    use FaceName::*;
    use JointName::*;

    proto(SingleTwistRight, [Seed(1), OnSpinRight])
        .pushes_x(params.push_lengths.x, [(AlphaX, OmegaX)])
        .pushes_y(params.push_lengths.y, [(AlphaY, OmegaY)])
        .pushes_z(params.push_lengths.z, [(AlphaZ, OmegaZ)])
        .pulls(params.pull_length, [(AlphaX, OmegaZ), (AlphaY, OmegaX), (AlphaZ, OmegaY)])
        .face(
            Spin::Right,
            [AlphaZ, AlphaY, AlphaX],
            [
                OnSpinRight.calls_it(Attach(Spin::Right)),
                Seed(1).calls_it(SingleBot),
                Seed(1).downwards(),
            ],
            [],
        )
        .face(
            Spin::Right,
            [OmegaX, OmegaY, OmegaZ],
            [
                OnSpinRight.calls_it(SingleTop),
                OnSpinRight.calls_it(AttachNext),
                Seed(1).calls_it(SingleTop),
            ],
            [],
        )
        .build()
}
