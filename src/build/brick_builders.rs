/// Pure Rust brick definitions - replacing Tenscript parsing
///
/// This module contains only the brick definition functions.
/// All supporting types and helpers are in the `brick_dsl` module.

use crate::build::brick_dsl::*;
use crate::build::tenscript::brick::BrickDefinition;
use crate::build::tenscript::Spin;

/// Build the Single-right brick (prototype 0)
pub fn single_right() -> BrickDefinition {
    use SingleJoint::*;
    
    
    use BrickName::*;
    use FaceContext::*;
    use Face::*;
    
    proto(Single)
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
            (OnSpin(Spin::Right), &[Base]),
            (Initial, &[Base]),
        ])
        .face(Spin::Right, [OmegaX, OmegaY, OmegaZ], [
            (OnSpin(Spin::Right), &[Top, NextBase]),
            (Initial, &[NextBase]),
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

/// Build the Single-left brick (prototype 1)
pub fn single_left() -> BrickDefinition {
    use SingleJoint::*;
    
    
    use BrickName::*;
    use FaceContext::*;
    use Face::*;
    
    proto(Single)
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
            (OnSpin(Spin::Left), &[Base]),
            (Initial, &[Base]),
        ])
        .face(Spin::Left, [OmegaZ, OmegaY, OmegaX], [
            (OnSpin(Spin::Left), &[Top, NextBase]),
            (Initial, &[NextBase]),
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

/// Build the Omni brick (prototype 2)
pub fn omni() -> BrickDefinition {
    use OmniJoint::*;
    
    
    use BrickName::*;
    use FaceContext::*;
    use Face::*;
    
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
            (OnSpin(Spin::Left), &[Top]),
            (OnSpin(Spin::Right), &[Base]),
            (Initial, &[TopRight]),
            (Initial1, &[Base, Bot]),
        ])
        .face(Spin::Left, [TopOmegaX, TopAlphaY, BotOmegaZ], [
            (OnSpin(Spin::Left), &[TopX]),
            (OnSpin(Spin::Right), &[BotX]),
            (Initial, &[FrontRight]),
            (Initial1, &[BotX]),
        ])
        .face(Spin::Left, [TopOmegaY, TopAlphaZ, BotOmegaX], [
            (OnSpin(Spin::Left), &[TopY]),
            (OnSpin(Spin::Right), &[BotY]),
            (Initial, &[BackRight]),
            (Initial1, &[BotY]),
        ])
        .face(Spin::Left, [TopOmegaZ, TopAlphaX, BotOmegaY], [
            (OnSpin(Spin::Left), &[TopZ]),
            (OnSpin(Spin::Right), &[BotZ]),
            (Initial, &[TopLeft]),
            (Initial1, &[BotZ]),
        ])
        .face(Spin::Right, [BotAlphaZ, BotOmegaX, TopAlphaY], [
            (OnSpin(Spin::Left), &[BotZ]),
            (OnSpin(Spin::Right), &[TopZ]),
            (Initial, &[Base, BottomRight]),
            (Initial1, &[TopZ]),
        ])
        .face(Spin::Right, [BotAlphaY, BotOmegaZ, TopAlphaX], [
            (OnSpin(Spin::Left), &[BotY]),
            (OnSpin(Spin::Right), &[TopY]),
            (Initial, &[FrontLeft]),
            (Initial1, &[TopY]),
        ])
        .face(Spin::Right, [BotAlphaX, BotOmegaY, TopAlphaZ], [
            (OnSpin(Spin::Left), &[BotX]),
            (OnSpin(Spin::Right), &[TopX]),
            (Initial, &[BackLeft]),
            (Initial1, &[TopX]),
        ])
        .face(Spin::Left, [BotAlphaX, BotAlphaY, BotAlphaZ], [
            (OnSpin(Spin::Left), &[Base]),
            (OnSpin(Spin::Right), &[Top]),
            (Initial, &[Base, BottomLeft]),
            (Initial1, &[Top]),
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

/// Build the Torque brick (prototype 3)
pub fn torque() -> BrickDefinition {
    
    
    use BrickName::*;
    use FaceContext::*;
    use Face::*;

    proto(Torque)
        .pushes(3.0, [
            ("left_front", "left_back"),
            ("middle_front", "middle_back"),
            ("right_front", "right_back"),
        ])
        .pushes(3.0, [
            ("front_left_bottom", "front_left_top"),
            ("front_right_bottom", "front_right_top"),
            ("back_left_bottom", "back_left_top"),
            ("back_right_bottom", "back_right_top"),
        ])
        .pushes(6.0, [
            ("top_left", "top_right"),
            ("bottom_left", "bottom_right"),
        ])
        .pulls(1.86, [
            ("middle_front", "front_left_bottom"),
            ("middle_front", "front_left_top"),
            ("middle_front", "front_right_bottom"),
            ("middle_front", "front_right_top"),
            ("middle_back", "back_left_bottom"),
            ("middle_back", "back_left_top"),
            ("middle_back", "back_right_bottom"),
            ("middle_back", "back_right_top"),
        ])
        .face(Spin::Left, ["bottom_left", "left_front", "front_left_bottom"], [
            (OnSpin(Spin::Left), &[Base]),
            (OnSpin(Spin::Right), &[FarSide]),
            (Initial, &[LeftFrontBottom, Base]),
        ])
        .face(Spin::Right, ["bottom_left", "left_back", "back_left_bottom"], [
            (OnSpin(Spin::Left), &[BaseBack]),
            (OnSpin(Spin::Right), &[FarBack]),
            (Initial, &[LeftBackBottom, Base]),
        ])
        .face(Spin::Left, ["bottom_right", "right_back", "back_right_bottom"], [
            (OnSpin(Spin::Left), &[FarBack]),
            (OnSpin(Spin::Right), &[BaseBack]),
            (Initial, &[RightBackBottom, Base]),
        ])
        .face(Spin::Right, ["bottom_right", "right_front", "front_right_bottom"], [
            (OnSpin(Spin::Left), &[FarSide]),
            (OnSpin(Spin::Right), &[Base]),
            (Initial, &[RightFrontBottom, Base]),
        ])
        .face(Spin::Left, ["top_left", "left_back", "back_left_top"], [
            (OnSpin(Spin::Left), &[BaseSide]),
            (OnSpin(Spin::Right), &[FarBase]),
            (Initial, &[LeftBackTop]),
        ])
        .face(Spin::Right, ["top_left", "left_front", "front_left_top"], [
            (OnSpin(Spin::Left), &[BaseFront]),
            (OnSpin(Spin::Right), &[FarFront]),
            (Initial, &[LeftFrontTop]),
        ])
        .face(Spin::Left, ["top_right", "right_front", "front_right_top"], [
            (OnSpin(Spin::Left), &[FarFront]),
            (OnSpin(Spin::Right), &[BaseFront]),
            (Initial, &[RightFrontTop]),
        ])
        .face(Spin::Right, ["top_right", "right_back", "back_right_top"], [
            (OnSpin(Spin::Left), &[FarBase]),
            (OnSpin(Spin::Right), &[BaseSide]),
            (Initial, &[RightBackTop]),
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

/// Build the TorqueRight brick (prototype 4)
pub fn torque_right() -> BrickDefinition {
    
    
    use BrickName::*;
    use FaceContext::*;
    use Face::*;

    proto(TorqueRight)
        .joints(["middle_front", "middle_back"])
        .pushes(3.35, [
            ("left_front", "left_back"),
            ("right_front", "right_back"),
        ])
        .pushes(3.6, [
            ("front_left_bottom", "front_left_top"),
            ("front_right_bottom", "front_right_top"),
            ("back_left_bottom", "back_left_top"),
            ("back_right_bottom", "back_right_top"),
        ])
        .pushes(5.6, [
            ("top_left", "top_right"),
            ("bottom_left", "bottom_right"),
        ])
        .pulls(1.98, [
            ("middle_front", "front_left_bottom"),
            ("middle_front", "front_left_top"),
            ("middle_front", "front_right_bottom"),
            ("middle_front", "front_right_top"),
            ("middle_back", "back_left_bottom"),
            ("middle_back", "back_left_top"),
            ("middle_back", "back_right_bottom"),
            ("middle_back", "back_right_top"),
        ])
        .pulls(1.92, [
            ("middle_front", "back_left_bottom"),
            ("middle_front", "back_right_top"),
            ("middle_back", "front_right_bottom"),
            ("middle_back", "front_left_top"),
        ])
        .face(Spin::Left, ["bottom_left", "left_front", "front_left_bottom"], [
            (OnSpin(Spin::Left), &[Base]),
            (OnSpin(Spin::Right), &[OtherA]),
            (Initial, &[LeftFrontBottom, Base]),
        ])
        .face(Spin::Right, ["bottom_left", "left_back", "back_left_bottom"], [
            (OnSpin(Spin::Left), &[OtherA]),
            (OnSpin(Spin::Right), &[Base]),
            (Initial, &[LeftBackBottom, Base]),
        ])
        .face(Spin::Left, ["bottom_right", "right_back", "back_right_bottom"], [
            (OnSpin(Spin::Left), &[FarBase]),
            (OnSpin(Spin::Right), &[FarOtherA]),
            (Initial, &[RightBackBottom, Base]),
        ])
        .face(Spin::Right, ["bottom_right", "right_front", "front_right_bottom"], [
            (OnSpin(Spin::Left), &[FarOtherB]),
            (OnSpin(Spin::Right), &[FarBrother]),
            (Initial, &[RightFrontBottom, Base]),
        ])
        .face(Spin::Left, ["top_left", "left_back", "back_left_top"], [
            (OnSpin(Spin::Left), &[Brother]),
            (OnSpin(Spin::Right), &[OtherB]),
            (Initial, &[LeftBackTop]),
        ])
        .face(Spin::Right, ["top_left", "left_front", "front_left_top"], [
            (OnSpin(Spin::Left), &[OtherB]),
            (OnSpin(Spin::Right), &[Brother]),
            (Initial, &[LeftFrontTop]),
        ])
        .face(Spin::Left, ["top_right", "right_front", "front_right_top"], [
            (OnSpin(Spin::Left), &[FarBrother]),
            (OnSpin(Spin::Right), &[FarOtherB]),
            (Initial, &[RightFrontTop]),
        ])
        .face(Spin::Right, ["top_right", "right_back", "back_right_top"], [
            (OnSpin(Spin::Left), &[FarOtherA]),
            (OnSpin(Spin::Right), &[FarBase]),
            (Initial, &[RightBackTop]),
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

/// Build the TorqueLeft brick (prototype 5)
pub fn torque_left() -> BrickDefinition {
    
    
    use BrickName::*;
    use FaceContext::*;
    use Face::*;

    proto(TorqueLeft)
        .joints(["middle_front", "middle_back"])
        .pushes(3.35, [
            ("left_front", "left_back"),
            ("right_front", "right_back"),
        ])
        .pushes(3.6, [
            ("front_left_bottom", "front_left_top"),
            ("front_right_bottom", "front_right_top"),
            ("back_left_bottom", "back_left_top"),
            ("back_right_bottom", "back_right_top"),
        ])
        .pushes(5.6, [
            ("top_left", "top_right"),
            ("bottom_left", "bottom_right"),
        ])
        .pulls(1.98, [
            ("middle_front", "front_left_bottom"),
            ("middle_front", "front_left_top"),
            ("middle_front", "front_right_bottom"),
            ("middle_front", "front_right_top"),
            ("middle_back", "back_left_bottom"),
            ("middle_back", "back_left_top"),
            ("middle_back", "back_right_bottom"),
            ("middle_back", "back_right_top"),
        ])
        .pulls(1.92, [
            ("middle_back", "front_left_bottom"),
            ("middle_back", "front_right_top"),
            ("middle_front", "back_right_bottom"),
            ("middle_front", "back_left_top"),
        ])
        .face(Spin::Left, ["bottom_left", "left_front", "front_left_bottom"], [
            (OnSpin(Spin::Right), &[OtherA]),
            (OnSpin(Spin::Left), &[Base]),
            (Initial, &[LeftFrontBottom, Base]),
        ])
        .face(Spin::Right, ["bottom_left", "left_back", "back_left_bottom"], [
            (OnSpin(Spin::Right), &[Base]),
            (OnSpin(Spin::Left), &[OtherA]),
            (Initial, &[LeftBackBottom, Base]),
        ])
        .face(Spin::Left, ["bottom_right", "right_back", "back_right_bottom"], [
            (OnSpin(Spin::Right), &[FarOtherB]),
            (OnSpin(Spin::Left), &[FarBrother]),
            (Initial, &[RightBackBottom, Base]),
        ])
        .face(Spin::Right, ["bottom_right", "right_front", "front_right_bottom"], [
            (OnSpin(Spin::Right), &[FarBase]),
            (OnSpin(Spin::Left), &[FarOtherA]),
            (Initial, &[RightFrontBottom, Base]),
        ])
        .face(Spin::Left, ["top_left", "left_back", "back_left_top"], [
            (OnSpin(Spin::Right), &[OtherB]),
            (OnSpin(Spin::Left), &[Brother]),
            (Initial, &[LeftBackTop]),
        ])
        .face(Spin::Right, ["top_left", "left_front", "front_left_top"], [
            (OnSpin(Spin::Right), &[Brother]),
            (OnSpin(Spin::Left), &[OtherB]),
            (Initial, &[LeftFrontTop]),
        ])
        .face(Spin::Left, ["top_right", "right_front", "front_right_top"], [
            (OnSpin(Spin::Right), &[FarOtherA]),
            (OnSpin(Spin::Left), &[FarBase]),
            (Initial, &[RightFrontTop]),
        ])
        .face(Spin::Right, ["top_right", "right_back", "back_right_top"], [
            (OnSpin(Spin::Right), &[FarBrother]),
            (OnSpin(Spin::Left), &[FarOtherB]),
            (Initial, &[RightBackTop]),
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

/// Build the Equals brick (prototype 6)
/// Note: This brick doesn't have baked data in the original Tenscript
pub fn equals() -> BrickDefinition {
    
    
    use BrickName::*;
    use FaceContext::*;
    use Face::*;

    let proto = proto(Equals)
        .pushes(4.0, [
            ("left_front", "left_back"),
            ("middle_front", "middle_back"),
            ("right_front", "right_back"),
        ])
        .pushes(4.0, [
            ("front_left_bottom", "front_left_top"),
            ("front_right_bottom", "front_right_top"),
            ("back_left_bottom", "back_left_top"),
            ("back_right_bottom", "back_right_top"),
        ])
        .pushes(6.0, [
            ("top_left", "top_right"),
            ("bottom_left", "bottom_right"),
        ])
        .pulls(1.8, [
            ("middle_front", "front_left_bottom"),
            ("middle_front", "front_left_top"),
            ("middle_front", "front_right_bottom"),
            ("middle_front", "front_right_top"),
            ("middle_back", "back_left_bottom"),
            ("middle_back", "back_left_top"),
            ("middle_back", "back_right_bottom"),
            ("middle_back", "back_right_top"),
        ])
        .face(Spin::Left, ["bottom_left", "left_front", "front_left_bottom"], [
            (OnSpin(Spin::Left), &[Base]),
            (OnSpin(Spin::Right), &[FarSide]),
            (Initial, &[LeftFrontBottom, Base]),
        ])
        .face(Spin::Right, ["bottom_left", "left_back", "back_left_bottom"], [
            (OnSpin(Spin::Left), &[BaseBack]),
            (OnSpin(Spin::Right), &[FarBack]),
            (Initial, &[LeftBackBottom, Base]),
        ])
        .face(Spin::Left, ["bottom_right", "right_back", "back_right_bottom"], [
            (OnSpin(Spin::Left), &[FarBack]),
            (OnSpin(Spin::Right), &[BaseBack]),
            (Initial, &[RightBackBottom, Base]),
        ])
        .face(Spin::Right, ["bottom_right", "right_front", "front_right_bottom"], [
            (OnSpin(Spin::Left), &[FarSide]),
            (OnSpin(Spin::Right), &[Base]),
            (Initial, &[RightFrontBottom, Base]),
        ])
        .face(Spin::Left, ["top_left", "left_back", "back_left_top"], [
            (OnSpin(Spin::Left), &[BaseSide]),
            (OnSpin(Spin::Right), &[FarBase]),
            (Initial, &[LeftBackTop]),
        ])
        .face(Spin::Right, ["top_left", "left_front", "front_left_top"], [
            (OnSpin(Spin::Left), &[BaseFront]),
            (OnSpin(Spin::Right), &[FarFront]),
            (Initial, &[LeftFrontTop]),
        ])
        .face(Spin::Left, ["top_right", "right_front", "front_right_top"], [
            (OnSpin(Spin::Left), &[FarFront]),
            (OnSpin(Spin::Right), &[BaseFront]),
            (Initial, &[RightFrontTop]),
        ])
        .face(Spin::Right, ["top_right", "right_back", "back_right_top"], [
            (OnSpin(Spin::Left), &[FarBase]),
            (OnSpin(Spin::Right), &[BaseSide]),
            (Initial, &[RightBackTop]),
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
