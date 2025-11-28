use crate::build::dsl::brick::BrickPrototype;
use crate::build::dsl::brick_dsl::*;
use crate::build::dsl::Spin;

/// Build the Single-left brick prototype
pub fn single_left(params: &SingleParams) -> BrickPrototype {
    use BrickName::*;
    use BrickRole::*;
    use FaceName::*;
    use JointName::*;

    proto(SingleLeftBrick, [Seed(1), OnSpinLeft])
        .pushes_x(params.push_lengths.x, [(AlphaX, OmegaX)])
        .pushes_y(params.push_lengths.y, [(AlphaY, OmegaY)])
        .pushes_z(params.push_lengths.z, [(AlphaZ, OmegaZ)])
        .pulls(params.pull_length, [(AlphaX, OmegaY), (AlphaY, OmegaZ), (AlphaZ, OmegaX)])
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
        .build()
}
