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
        .baked(0.9136)
        .joints([
            (-1.1132, 0.0000, -0.0001),
            (0.9607, 1.9315, -0.5490),
            (0.5461, 0.0000, 0.9573),
            (-0.9479, 1.9448, -0.5568),
            (0.5450, 0.0000, -0.9516),
            (0.0040, 1.9504, 1.1055),
        ])
        .pushes([
            (0, 1, -0.0168),
            (2, 3, -0.0184),
            (4, 5, -0.0170),
        ])
        .pulls([
            (0, 3, 0.1081),
            (2, 5, 0.1082),
            (4, 1, 0.1007),
        ])
        .build()
}
