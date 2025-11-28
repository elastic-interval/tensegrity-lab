use crate::build::dsl::brick::Brick;
use crate::build::dsl::brick_dsl::*;
use crate::build::dsl::Spin;
use cgmath::Vector3;

/// Build the Single-right brick
pub fn single_right(push_lengths: Vector3<f32>, pull_length: f32) -> Brick {
    use BrickName::*;
    use BrickRole::*;
    use FaceName::*;
    use JointName::*;

    proto(SingleRightBrick, [Seed(1), OnSpinRight])
        .pushes_x(push_lengths.x, [(AlphaX, OmegaX)])
        .pushes_y(push_lengths.y, [(AlphaY, OmegaY)])
        .pushes_z(push_lengths.z, [(AlphaZ, OmegaZ)])
        .pulls(pull_length, [(AlphaX, OmegaZ), (AlphaY, OmegaX), (AlphaZ, OmegaY)])
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
        .baked(0.9134)
        .joints([
            (-0.9531, 0.0001, 0.5560),
            (1.1035, 1.9434, -0.0064),
            (0.9581, 0.0000, 0.5468),
            (-0.5569, 1.9432, -0.9526),
            (-0.0055, 0.0002, -1.1043),
            (-0.5464, 1.9430, 0.9591),
        ])
        .pushes([
            (0, 1, -0.0172),
            (2, 3, -0.0174),
            (4, 5, -0.0171),
        ])
        .pulls([
            (0, 5, 0.1060),
            (2, 1, 0.1061),
            (4, 3, 0.1062),
        ])
        .build()
}
