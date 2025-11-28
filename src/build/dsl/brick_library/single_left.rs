use crate::build::dsl::brick::Brick;
use crate::build::dsl::brick_dsl::*;
use crate::build::dsl::Spin;
use cgmath::Vector3;

/// Build the Single-left brick
pub fn single_left(push_lengths: Vector3<f32>, pull_length: f32) -> Brick {
    use BrickName::*;
    use BrickRole::*;
    use FaceName::*;
    use JointName::*;

    proto(SingleLeftBrick, [Seed(1), OnSpinLeft])
        .pushes_x(push_lengths.x, [(AlphaX, OmegaX)])
        .pushes_y(push_lengths.y, [(AlphaY, OmegaY)])
        .pushes_z(push_lengths.z, [(AlphaZ, OmegaZ)])
        .pulls(pull_length, [(AlphaX, OmegaY), (AlphaY, OmegaZ), (AlphaZ, OmegaX)])
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
        .baked()
        .joints([
            (-1.1357, 0.0000, -0.0129),
            (0.9921, 2.2410, -0.5540),
            (0.5533, 0.0000, 0.9923),
            (-0.9736, 2.2407, -0.5821),
            (0.5815, 0.0003, -0.9730),
            (-0.0170, 2.2416, 1.1330),
        ])
        .pushes([
            (0, 1, -0.0223),
            (2, 3, -0.0229),
            (4, 5, -0.0236),
        ])
        .pulls([
            (0, 3, 0.1554),
            (2, 5, 0.1556),
            (4, 1, 0.1547),
        ])
        .build()
}
