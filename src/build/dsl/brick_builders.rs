use crate::build::dsl::brick::BrickDefinition;
/// Brick definitions using the type-safe Rust DSL.
///
/// All supporting types and helpers are in the `brick_dsl` module.

use crate::build::dsl::brick_dsl::*;
use crate::build::dsl::brick_dsl::FaceName::{Single, OmiFaceDown, Four, TorqueOnTop, TorqueFourDown, TorqueTied};
use crate::build::dsl::Spin;

/// Build the Single-right brick
pub fn single_right() -> BrickDefinition {
    use JointName::*;
    use SingleFace::*;

    use BrickName::*;
    use FaceContext::*;

    proto(SingleBrick)
        .pushes(3.204, [
            (AlphaX, OmegaX),
            (AlphaY, OmegaY),
            (AlphaZ, OmegaZ),
        ])
        .pulls(2.0, [
            (AlphaX, OmegaZ),
            (AlphaY, OmegaX),
            (AlphaZ, OmegaY),
        ])
        .face(Spin::Right, [AlphaZ, AlphaY, AlphaX], [
            OnSpinRight.calls_it(&[Single(Base)]),
            SeedA.calls_it(&[Single(Base)]),
        ])
        .face(Spin::Right, [OmegaX, OmegaY, OmegaZ], [
            OnSpinRight.calls_it(&[Single(Top), Single(NextBase)]),
            SeedA.calls_it(&[Single(NextBase)]),
        ])
        .baked()
        .joints([
            (-1.4913, -0.3875,  0.0099),
            ( 1.4913, -0.0099,  0.3875),
            ( 0.0099, -1.4913, -0.3875),
            ( 0.3875,  1.4913, -0.0099),
            (-0.3875,  0.0099, -1.4913),
            (-0.0099,  0.3875,  1.4913),
        ])
        .pushes([
            (2, 3, -0.0531),
            (4, 5, -0.0531),
            (0, 1, -0.0531),
        ])
        .pulls([
            (2, 1,  0.1171),
            (0, 5,  0.1171),
            (4, 3,  0.1171),
        ])
        // Faces are derived from proto on-demand via BrickDefinition::baked_faces()
        .build()
}

/// Build the Single-left brick
pub fn single_left() -> BrickDefinition {
    use JointName::*;
    use SingleFace::*;

    use BrickName::*;
    use FaceContext::*;

    proto(SingleBrick)
        .pushes(3.204, [
            (AlphaX, OmegaX),
            (AlphaY, OmegaY),
            (AlphaZ, OmegaZ),
        ])
        .pulls(2.0, [
            (AlphaX, OmegaY),
            (AlphaY, OmegaZ),
            (AlphaZ, OmegaX),
        ])
        .face(Spin::Left, [AlphaX, AlphaY, AlphaZ], [
            OnSpinLeft.calls_it(&[Single(Base)]),
            SeedA.calls_it(&[Single(Base)]),
        ])
        .face(Spin::Left, [OmegaZ, OmegaY, OmegaX], [
            OnSpinLeft.calls_it(&[Single(Top), Single(NextBase)]),
            SeedA.calls_it(&[Single(NextBase)]),
        ])
        .baked()
        .joints([
            (-1.4913,  0.0099, -0.3875),
            ( 1.4913,  0.3875, -0.0099),
            (-0.3875, -1.4913,  0.0099),
            (-0.0099,  1.4913,  0.3875),
            ( 0.0099, -0.3875, -1.4913),
            ( 0.3875, -0.0099,  1.4913),
        ])
        .pushes([
            (0, 1, -0.0531),
            (4, 5, -0.0531),
            (2, 3, -0.0531),
        ])
        .pulls([
            (4, 1,  0.1171),
            (2, 5,  0.1171),
            (0, 3,  0.1171),
        ])
        // Faces are derived from proto on-demand via BrickDefinition::baked_faces()
        .build()
}

/// Build the Omni brick
pub fn omni() -> BrickDefinition {
    use JointName::*;
    use OmniFaceDown::*;
    use FourDown::{BackLeft, BackRight, BottomLeft, BottomRight, FrontLeft, FrontRight, TopLeft, TopRight};
    use SingleFace::Base;

    use BrickName::*;
    use FaceContext::*;
    
    proto(Omni)
        .pushes(3.271, [
            (BotAlphaX, BotOmegaX),
            (TopAlphaX, TopOmegaX),
            (BotAlphaY, BotOmegaY),
            (TopAlphaY, TopOmegaY),
            (BotAlphaZ, BotOmegaZ),
            (TopAlphaZ, TopOmegaZ),
        ])
        .face(Spin::Right, [TopOmegaX, TopOmegaY, TopOmegaZ], [
            OnSpinLeft.calls_it(&[OmiFaceDown(Top)]),
            OnSpinRight.calls_it(&[Single(Base)]),
            SeedA.calls_it(&[Four(TopRight)]),
            SeedB.calls_it(&[Single(Base), OmiFaceDown(Bot)]),
        ])
        .face(Spin::Left, [TopOmegaX, TopAlphaY, BotOmegaZ], [
            OnSpinLeft.calls_it(&[OmiFaceDown(TopX)]),
            OnSpinRight.calls_it(&[OmiFaceDown(BotX)]),
            SeedA.calls_it(&[Four(FrontRight)]),
            SeedB.calls_it(&[OmiFaceDown(BotX)]),
        ])
        .face(Spin::Left, [TopOmegaY, TopAlphaZ, BotOmegaX], [
            OnSpinLeft.calls_it(&[OmiFaceDown(TopY)]),
            OnSpinRight.calls_it(&[OmiFaceDown(BotY)]),
            SeedA.calls_it(&[Four(BackRight)]),
            SeedB.calls_it(&[OmiFaceDown(BotY)]),
        ])
        .face(Spin::Left, [TopOmegaZ, TopAlphaX, BotOmegaY], [
            OnSpinLeft.calls_it(&[OmiFaceDown(TopZ)]),
            OnSpinRight.calls_it(&[OmiFaceDown(BotZ)]),
            SeedA.calls_it(&[Four(TopLeft)]),
            SeedB.calls_it(&[OmiFaceDown(BotZ)]),
        ])
        .face(Spin::Right, [BotAlphaZ, BotOmegaX, TopAlphaY], [
            OnSpinLeft.calls_it(&[OmiFaceDown(BotZ)]),
            OnSpinRight.calls_it(&[OmiFaceDown(TopZ)]),
            SeedA.calls_it(&[Single(Base), Four(BottomRight)]),
            SeedB.calls_it(&[OmiFaceDown(TopZ)]),
        ])
        .face(Spin::Right, [BotAlphaY, BotOmegaZ, TopAlphaX], [
            OnSpinLeft.calls_it(&[OmiFaceDown(BotY)]),
            OnSpinRight.calls_it(&[OmiFaceDown(TopY)]),
            SeedA.calls_it(&[Four(FrontLeft)]),
            SeedB.calls_it(&[OmiFaceDown(TopY)]),
        ])
        .face(Spin::Right, [BotAlphaX, BotOmegaY, TopAlphaZ], [
            OnSpinLeft.calls_it(&[OmiFaceDown(BotX)]),
            OnSpinRight.calls_it(&[OmiFaceDown(TopX)]),
            SeedA.calls_it(&[Four(BackLeft)]),
            SeedB.calls_it(&[OmiFaceDown(TopX)]),
        ])
        .face(Spin::Left, [BotAlphaX, BotAlphaY, BotAlphaZ], [
            OnSpinLeft.calls_it(&[Single(Base)]),
            OnSpinRight.calls_it(&[OmiFaceDown(Top)]),
            SeedA.calls_it(&[Single(Base), Four(BottomLeft)]),
            SeedB.calls_it(&[OmiFaceDown(Top)]),
        ])
        .baked()
        .joints([
            (-1.5556, -0.0000, -0.7722),
            ( 1.5556,  0.0000, -0.7722),
            (-1.5556,  0.0000,  0.7722),
            ( 1.5556, -0.0000,  0.7722),
            (-0.7722, -1.5556,  0.0000),
            (-0.7722,  1.5556, -0.0000),
            ( 0.7722, -1.5556, -0.0000),
            ( 0.7722,  1.5556, -0.0000),
            (-0.0000, -0.7722, -1.5556),
            (-0.0000, -0.7722,  1.5556),
            (-0.0000,  0.7722, -1.5556),
            (-0.0000,  0.7722,  1.5556),
        ])
        .pushes([
            (2, 3, -0.0473),
            (4, 5, -0.0473),
            (6, 7, -0.0473),
            (0, 1, -0.0473),
            (8, 9, -0.0473),
            (10, 11, -0.0473),
        ])
        .pulls([])
        .build()
}

/// Build the Torque brick
pub fn torque() -> BrickDefinition {
    use JointName::*;

    use BrickName::*;
    use FaceContext::*;
    use TorqueFaceOnTop::*;
    use TorqueFaceFourDown::*;
    use SingleFace::Base;
    
    

    proto(Torque)
        .pushes(3.0, [
            (LeftFront, LeftBack),
            (MiddleFront, MiddleBack),
            (RightFront, RightBack),
        ])
        .pushes(3.0, [
            (FrontLeftBottom, FrontLeftTop),
            (FrontRightBottom, FrontRightTop),
            (BackLeftBottom, BackLeftTop),
            (BackRightBottom, BackRightTop),
        ])
        .pushes(6.0, [
            (TopLeft, TopRight),
            (BottomLeft, BottomRight),
        ])
        .pulls(1.86, [
            (MiddleFront, FrontLeftBottom),
            (MiddleFront, FrontLeftTop),
            (MiddleFront, FrontRightBottom),
            (MiddleFront, FrontRightTop),
            (MiddleBack, BackLeftBottom),
            (MiddleBack, BackLeftTop),
            (MiddleBack, BackRightBottom),
            (MiddleBack, BackRightTop),
        ])
        .face(Spin::Left, [BottomLeft, LeftFront, FrontLeftBottom], [
            OnSpinLeft.calls_it(&[Single(Base)]),
            OnSpinRight.calls_it(&[TorqueOnTop(FarSide)]),
            SeedA.calls_it(&[TorqueFourDown(LeftFrontBottom), Single(Base)]),
        ])
        .face(Spin::Right, [BottomLeft, LeftBack, BackLeftBottom], [
            OnSpinLeft.calls_it(&[TorqueOnTop(BaseBack)]),
            OnSpinRight.calls_it(&[TorqueOnTop(FarBack)]),
            SeedA.calls_it(&[TorqueFourDown(LeftBackBottom), Single(Base)]),
        ])
        .face(Spin::Left, [BottomRight, RightBack, BackRightBottom], [
            OnSpinLeft.calls_it(&[TorqueOnTop(FarBack)]),
            OnSpinRight.calls_it(&[TorqueOnTop(BaseBack)]),
            SeedA.calls_it(&[TorqueFourDown(RightBackBottom), Single(Base)]),
        ])
        .face(Spin::Right, [BottomRight, RightFront, FrontRightBottom], [
            OnSpinLeft.calls_it(&[TorqueOnTop(FarSide)]),
            OnSpinRight.calls_it(&[Single(Base)]),
            SeedA.calls_it(&[TorqueFourDown(RightFrontBottom), Single(Base)]),
        ])
        .face(Spin::Left, [TopLeft, LeftBack, BackLeftTop], [
            OnSpinLeft.calls_it(&[TorqueOnTop(BaseSide)]),
            OnSpinRight.calls_it(&[TorqueOnTop(FarBase)]),
            SeedA.calls_it(&[TorqueFourDown(LeftBackTop)]),
        ])
        .face(Spin::Right, [TopLeft, LeftFront, FrontLeftTop], [
            OnSpinLeft.calls_it(&[TorqueOnTop(BaseFront)]),
            OnSpinRight.calls_it(&[TorqueOnTop(FarFront)]),
            SeedA.calls_it(&[TorqueFourDown(LeftFrontTop)]),
        ])
        .face(Spin::Left, [TopRight, RightFront, FrontRightTop], [
            OnSpinLeft.calls_it(&[TorqueOnTop(FarFront)]),
            OnSpinRight.calls_it(&[TorqueOnTop(BaseFront)]),
            SeedA.calls_it(&[TorqueFourDown(RightFrontTop)]),
        ])
        .face(Spin::Right, [TopRight, RightBack, BackRightTop], [
            OnSpinLeft.calls_it(&[TorqueOnTop(FarBase)]),
            OnSpinRight.calls_it(&[TorqueOnTop(BaseSide)]),
            SeedA.calls_it(&[TorqueFourDown(RightBackTop)]),
        ])
        .baked()
        .joints([
            (-1.4967,  0.0000, -2.2107),
            ( 1.4967, -0.0000, -2.2106),
            (-1.4968,  0.0000,  0.0000),
            ( 1.4968,  0.0000,  0.0000),
            (-1.4967,  0.0000,  2.2107),
            ( 1.4967,  0.0000,  2.2106),
            (-1.0572, -1.4961, -1.3771),
            (-1.0572,  1.4961, -1.3771),
            (-1.0572, -1.4961,  1.3771),
            (-1.0572,  1.4961,  1.3771),
            ( 1.0572, -1.4961, -1.3771),
            ( 1.0572,  1.4961, -1.3771),
            ( 1.0572, -1.4961,  1.3771),
            ( 1.0572,  1.4961,  1.3771),
            ( 0.0000,  0.8226, -2.9920),
            ( 0.0000,  0.8226,  2.9920),
            (-0.0000, -0.8226, -2.9920),
            ( 0.0000, -0.8226,  2.9920),
        ])
        .pushes([
            (0, 1, -0.0011),
            (4, 5, -0.0011),
            (2, 3, -0.0010),
            (16, 17, -0.0016),
            (10, 11, -0.0015),
            (8, 9, -0.0015),
            (6, 7, -0.0015),
            (12, 13, -0.0015),
            (14, 15, -0.0016),
        ])
        .pulls([
            (2, 9, 0.1189),
            (3, 11, 0.1189),
            (3, 10, 0.1189),
            (2, 8, 0.1189),
            (2, 6, 0.1189),
            (2, 7, 0.1189),
            (3, 13, 0.1189),
            (3, 12, 0.1189),
        ])
        .build()
}

/// Build the TorqueRight brick
pub fn torque_right() -> BrickDefinition {
    use JointName::*;

    use BrickName::*;
    use FaceContext::*;
    use TorqueFaceOnTop::*;
    use TorqueFaceFourDown::*;
    use TorqueTiedFace::*;
    use SingleFace::Base;
    
    

    proto(TorqueRight)
        .joints([MiddleFront, MiddleBack])
        .pushes(3.35, [
            (LeftFront, LeftBack),
            (RightFront, RightBack),
        ])
        .pushes(3.6, [
            (FrontLeftBottom, FrontLeftTop),
            (FrontRightBottom, FrontRightTop),
            (BackLeftBottom, BackLeftTop),
            (BackRightBottom, BackRightTop),
        ])
        .pushes(5.6, [
            (TopLeft, TopRight),
            (BottomLeft, BottomRight),
        ])
        .pulls(1.98, [
            (MiddleFront, FrontLeftBottom),
            (MiddleFront, FrontLeftTop),
            (MiddleFront, FrontRightBottom),
            (MiddleFront, FrontRightTop),
            (MiddleBack, BackLeftBottom),
            (MiddleBack, BackLeftTop),
            (MiddleBack, BackRightBottom),
            (MiddleBack, BackRightTop),
        ])
        .pulls(1.92, [
            (MiddleFront, BackLeftBottom),
            (MiddleFront, BackRightTop),
            (MiddleBack, FrontRightBottom),
            (MiddleBack, FrontLeftTop),
        ])
        .face(Spin::Left, [BottomLeft, LeftFront, FrontLeftBottom], [
            OnSpinLeft.calls_it(&[Single(Base)]),
            OnSpinRight.calls_it(&[TorqueTied(OtherA)]),
            SeedA.calls_it(&[TorqueFourDown(LeftFrontBottom), Single(Base)]),
        ])
        .face(Spin::Right, [BottomLeft, LeftBack, BackLeftBottom], [
            OnSpinLeft.calls_it(&[TorqueTied(OtherA)]),
            OnSpinRight.calls_it(&[Single(Base)]),
            SeedA.calls_it(&[TorqueFourDown(LeftBackBottom), Single(Base)]),
        ])
        .face(Spin::Left, [BottomRight, RightBack, BackRightBottom], [
            OnSpinLeft.calls_it(&[TorqueOnTop(FarBase)]),
            OnSpinRight.calls_it(&[TorqueTied(FarOtherA)]),
            SeedA.calls_it(&[TorqueFourDown(RightBackBottom), Single(Base)]),
        ])
        .face(Spin::Right, [BottomRight, RightFront, FrontRightBottom], [
            OnSpinLeft.calls_it(&[TorqueTied(FarOtherB)]),
            OnSpinRight.calls_it(&[TorqueOnTop(FarBrother)]),
            SeedA.calls_it(&[TorqueFourDown(RightFrontBottom), Single(Base)]),
        ])
        .face(Spin::Left, [TopLeft, LeftBack, BackLeftTop], [
            OnSpinLeft.calls_it(&[TorqueTied(Brother)]),
            OnSpinRight.calls_it(&[TorqueTied(OtherB)]),
            SeedA.calls_it(&[TorqueFourDown(LeftBackTop)]),
        ])
        .face(Spin::Right, [TopLeft, LeftFront, FrontLeftTop], [
            OnSpinLeft.calls_it(&[TorqueTied(OtherB)]),
            OnSpinRight.calls_it(&[TorqueTied(Brother)]),
            SeedA.calls_it(&[TorqueFourDown(LeftFrontTop)]),
        ])
        .face(Spin::Left, [TopRight, RightFront, FrontRightTop], [
            OnSpinLeft.calls_it(&[TorqueOnTop(FarBrother)]),
            OnSpinRight.calls_it(&[TorqueTied(FarOtherB)]),
            SeedA.calls_it(&[TorqueFourDown(RightFrontTop)]),
        ])
        .face(Spin::Right, [TopRight, RightBack, BackRightTop], [
            OnSpinLeft.calls_it(&[TorqueTied(FarOtherA)]),
            OnSpinRight.calls_it(&[TorqueOnTop(FarBase)]),
            SeedA.calls_it(&[TorqueFourDown(RightBackTop)]),
        ])
        .baked()
        .joints([
            (-0.7039,  2.4632, -0.0000),
            ( 0.7039,  2.4632,  0.0000),
            (-1.3573,  3.2825, -1.9516),
            ( 1.3573,  1.6439, -1.9516),
            (-1.3573,  1.6439,  1.9516),
            ( 1.3573,  3.2825,  1.9516),
            (-1.9315,  1.4217, -1.4279),
            ( 0.3778,  3.9264, -1.1956),
            ( 0.3778,  1.0000,  1.1956),
            (-1.9315,  3.5047,  1.4279),
            (-0.3778,  1.0000, -1.1956),
            ( 1.9315,  3.5047, -1.4279),
            ( 1.9315,  1.4217,  1.4279),
            (-0.3778,  3.9264,  1.1956),
            ( 0.5942,  3.1285, -2.6228),
            (-0.5942,  3.1285,  2.6228),
            (-0.5942,  1.7979, -2.6228),
            ( 0.5942,  1.7979,  2.6228),
        ])
        .pushes([
            (2, 3, -0.0519),
            (4, 5, -0.0519),
            (16, 17, -0.0384),
            (10, 11, -0.0504),
            (6, 7, -0.0504),
            (12, 13, -0.0504),
            (14, 15, -0.0384),
            (8, 9, -0.0504),
        ])
        .pulls([
            (0, 6, 0.0868),
            (0, 8, 0.0996),
            (0, 13, 0.0000),
            (1, 7, 0.0000),
            (0, 9, 0.0868),
            (0, 10, 0.0000),
            (1, 10, 0.0996),
            (1, 13, 0.0996),
            (1, 11, 0.0868),
            (0, 7, 0.0996),
            (1, 8, 0.0000),
            (1, 12, 0.0868),
        ])
        .build()
}

/// Build the TorqueLeft brick
pub fn torque_left() -> BrickDefinition {
    use JointName::*;

    use BrickName::*;
    use FaceContext::*;
    use TorqueFaceOnTop::*;
    use TorqueFaceFourDown::*;
    use TorqueTiedFace::*;
    use SingleFace::Base;
    
    

    proto(TorqueLeft)
        .joints([MiddleFront, MiddleBack])
        .pushes(3.35, [
            (LeftFront, LeftBack),
            (RightFront, RightBack),
        ])
        .pushes(3.6, [
            (FrontLeftBottom, FrontLeftTop),
            (FrontRightBottom, FrontRightTop),
            (BackLeftBottom, BackLeftTop),
            (BackRightBottom, BackRightTop),
        ])
        .pushes(5.6, [
            (TopLeft, TopRight),
            (BottomLeft, BottomRight),
        ])
        .pulls(1.98, [
            (MiddleFront, FrontLeftBottom),
            (MiddleFront, FrontLeftTop),
            (MiddleFront, FrontRightBottom),
            (MiddleFront, FrontRightTop),
            (MiddleBack, BackLeftBottom),
            (MiddleBack, BackLeftTop),
            (MiddleBack, BackRightBottom),
            (MiddleBack, BackRightTop),
        ])
        .pulls(1.92, [
            (MiddleBack, FrontLeftBottom),
            (MiddleBack, FrontRightTop),
            (MiddleFront, BackRightBottom),
            (MiddleFront, BackLeftTop),
        ])
        .face(Spin::Left, [BottomLeft, LeftFront, FrontLeftBottom], [
            OnSpinRight.calls_it(&[TorqueTied(OtherA)]),
            OnSpinLeft.calls_it(&[Single(Base)]),
            SeedA.calls_it(&[TorqueFourDown(LeftFrontBottom), Single(Base)]),
        ])
        .face(Spin::Right, [BottomLeft, LeftBack, BackLeftBottom], [
            OnSpinRight.calls_it(&[Single(Base)]),
            OnSpinLeft.calls_it(&[TorqueTied(OtherA)]),
            SeedA.calls_it(&[TorqueFourDown(LeftBackBottom), Single(Base)]),
        ])
        .face(Spin::Left, [BottomRight, RightBack, BackRightBottom], [
            OnSpinRight.calls_it(&[TorqueTied(FarOtherB)]),
            OnSpinLeft.calls_it(&[TorqueOnTop(FarBrother)]),
            SeedA.calls_it(&[TorqueFourDown(RightBackBottom), Single(Base)]),
        ])
        .face(Spin::Right, [BottomRight, RightFront, FrontRightBottom], [
            OnSpinRight.calls_it(&[TorqueOnTop(FarBase)]),
            OnSpinLeft.calls_it(&[TorqueTied(FarOtherA)]),
            SeedA.calls_it(&[TorqueFourDown(RightFrontBottom), Single(Base)]),
        ])
        .face(Spin::Left, [TopLeft, LeftBack, BackLeftTop], [
            OnSpinRight.calls_it(&[TorqueTied(OtherB)]),
            OnSpinLeft.calls_it(&[TorqueTied(Brother)]),
            SeedA.calls_it(&[TorqueFourDown(LeftBackTop)]),
        ])
        .face(Spin::Right, [TopLeft, LeftFront, FrontLeftTop], [
            OnSpinRight.calls_it(&[TorqueTied(Brother)]),
            OnSpinLeft.calls_it(&[TorqueTied(OtherB)]),
            SeedA.calls_it(&[TorqueFourDown(LeftFrontTop)]),
        ])
        .face(Spin::Left, [TopRight, RightFront, FrontRightTop], [
            OnSpinRight.calls_it(&[TorqueTied(FarOtherA)]),
            OnSpinLeft.calls_it(&[TorqueOnTop(FarBase)]),
            SeedA.calls_it(&[TorqueFourDown(RightFrontTop)]),
        ])
        .face(Spin::Right, [TopRight, RightBack, BackRightTop], [
            OnSpinRight.calls_it(&[TorqueOnTop(FarBrother)]),
            OnSpinLeft.calls_it(&[TorqueTied(FarOtherB)]),
            SeedA.calls_it(&[TorqueFourDown(RightBackTop)]),
        ])
        .baked()
        .joints([
            (-0.7039,  2.4632,  0.0000),
            ( 0.7039,  2.4632, -0.0000),
            (-1.3573,  1.6439, -1.9516),
            ( 1.3573,  3.2825, -1.9516),
            (-1.3573,  3.2825,  1.9516),
            ( 1.3573,  1.6439,  1.9516),
            ( 0.3778,  1.0000, -1.1956),
            (-1.9315,  3.5047, -1.4279),
            (-1.9315,  1.4217,  1.4279),
            ( 0.3778,  3.9264,  1.1956),
            ( 1.9315,  1.4217, -1.4279),
            (-0.3778,  3.9264, -1.1956),
            (-0.3778,  1.0000,  1.1956),
            ( 1.9315,  3.5047,  1.4279),
            (-0.5942,  3.1285, -2.6228),
            ( 0.5942,  3.1285,  2.6228),
            ( 0.5942,  1.7979, -2.6228),
            (-0.5942,  1.7979,  2.6228),
        ])
        .pushes([
            (2, 3, -0.0519),
            (4, 5, -0.0519),
            (16, 17, -0.0384),
            (10, 11, -0.0504),
            (6, 7, -0.0504),
            (8, 9, -0.0504),
            (14, 15, -0.0384),
            (12, 13, -0.0504),
        ])
        .pulls([
            (0, 9, 0.0996),
            (0, 6, 0.0996),
            (1, 12, 0.0996),
            (1, 11, 0.0996),
            (1, 9, 0.0000),
            (0, 7, 0.0868),
            (0, 8, 0.0868),
            (1, 10, 0.0868),
            (1, 13, 0.0868),
            (0, 11, 0.0000),
            (1, 6, 0.0000),
            (0, 12, 0.0000),
        ])
        .build()
}

/// Build the Equals brick
pub fn equals() -> BrickDefinition {
    use JointName::*;

    use BrickName::*;
    use FaceContext::*;
    use TorqueFaceOnTop::*;
    use TorqueFaceFourDown::*;
    use SingleFace::Base;
    
    

    let proto = proto(Equals)
        .pushes(4.0, [
            (LeftFront, LeftBack),
            (MiddleFront, MiddleBack),
            (RightFront, RightBack),
        ])
        .pushes(4.0, [
            (FrontLeftBottom, FrontLeftTop),
            (FrontRightBottom, FrontRightTop),
            (BackLeftBottom, BackLeftTop),
            (BackRightBottom, BackRightTop),
        ])
        .pushes(6.0, [
            (TopLeft, TopRight),
            (BottomLeft, BottomRight),
        ])
        .pulls(1.8, [
            (MiddleFront, FrontLeftBottom),
            (MiddleFront, FrontLeftTop),
            (MiddleFront, FrontRightBottom),
            (MiddleFront, FrontRightTop),
            (MiddleBack, BackLeftBottom),
            (MiddleBack, BackLeftTop),
            (MiddleBack, BackRightBottom),
            (MiddleBack, BackRightTop),
        ])
        .face(Spin::Left, [BottomLeft, LeftFront, FrontLeftBottom], [
            OnSpinLeft.calls_it(&[Single(Base)]),
            OnSpinRight.calls_it(&[TorqueOnTop(FarSide)]),
            SeedA.calls_it(&[TorqueFourDown(LeftFrontBottom), Single(Base)]),
        ])
        .face(Spin::Right, [BottomLeft, LeftBack, BackLeftBottom], [
            OnSpinLeft.calls_it(&[TorqueOnTop(BaseBack)]),
            OnSpinRight.calls_it(&[TorqueOnTop(FarBack)]),
            SeedA.calls_it(&[TorqueFourDown(LeftBackBottom), Single(Base)]),
        ])
        .face(Spin::Left, [BottomRight, RightBack, BackRightBottom], [
            OnSpinLeft.calls_it(&[TorqueOnTop(FarBack)]),
            OnSpinRight.calls_it(&[TorqueOnTop(BaseBack)]),
            SeedA.calls_it(&[TorqueFourDown(RightBackBottom), Single(Base)]),
        ])
        .face(Spin::Right, [BottomRight, RightFront, FrontRightBottom], [
            OnSpinLeft.calls_it(&[TorqueOnTop(FarSide)]),
            OnSpinRight.calls_it(&[Single(Base)]),
            SeedA.calls_it(&[TorqueFourDown(RightFrontBottom), Single(Base)]),
        ])
        .face(Spin::Left, [TopLeft, LeftBack, BackLeftTop], [
            OnSpinLeft.calls_it(&[TorqueOnTop(BaseSide)]),
            OnSpinRight.calls_it(&[TorqueOnTop(FarBase)]),
            SeedA.calls_it(&[TorqueFourDown(LeftBackTop)]),
        ])
        .face(Spin::Right, [TopLeft, LeftFront, FrontLeftTop], [
            OnSpinLeft.calls_it(&[TorqueOnTop(BaseFront)]),
            OnSpinRight.calls_it(&[TorqueOnTop(FarFront)]),
            SeedA.calls_it(&[TorqueFourDown(LeftFrontTop)]),
        ])
        .face(Spin::Left, [TopRight, RightFront, FrontRightTop], [
            OnSpinLeft.calls_it(&[TorqueOnTop(FarFront)]),
            OnSpinRight.calls_it(&[TorqueOnTop(BaseFront)]),
            SeedA.calls_it(&[TorqueFourDown(RightFrontTop)]),
        ])
        .face(Spin::Right, [TopRight, RightBack, BackRightTop], [
            OnSpinLeft.calls_it(&[TorqueOnTop(FarBase)]),
            OnSpinRight.calls_it(&[TorqueOnTop(BaseSide)]),
            SeedA.calls_it(&[TorqueFourDown(RightBackTop)]),
        ])
        .build_proto();

    BrickDefinition {
        proto,
        baked: None,
    }
}

/// Build the complete brick library from Rust code
pub fn build_brick_library() -> Vec<BrickDefinition> {
    vec![
        single_right(),
        single_left(),
        omni(),
        torque(),
        torque_right(),
        torque_left(),
        equals(),
    ]
}
