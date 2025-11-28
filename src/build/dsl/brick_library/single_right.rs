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
