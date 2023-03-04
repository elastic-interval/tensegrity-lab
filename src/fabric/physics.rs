/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum SurfaceCharacter {
    #[default]
    Absent,
    Frozen,
    Sticky,
    Bouncy,
}

#[derive(Debug, Clone)]
pub struct Physics {
    pub surface_character: SurfaceCharacter,
    pub gravity: f32,
    pub antigravity: f32,
    pub viscosity: f32,
    pub drag: f32,
    pub stiffness: f32,
}

pub mod presets {
    use crate::fabric::physics::Physics;
    use crate::fabric::physics::SurfaceCharacter::{Absent, Frozen};

    pub const LIQUID: Physics = Physics {
        surface_character: Absent,
        gravity: 0.0,
        antigravity: 0.0,
        viscosity: 1e3,
        drag: 1.0 - 1e-6,
        stiffness: 1e-3,
    };

    pub const PROTOTYPE_FORMATION: Physics = Physics {
        surface_character: Absent,
        gravity: 0.0,
        antigravity: 0.0,
        viscosity: 1e4,
        drag: 1.0 - 1e-2,
        stiffness: 5e-5,
    };

    pub const AIR_GRAVITY: Physics = Physics {
        surface_character: Frozen,
        gravity: 1e-7,
        antigravity: 1e-3,
        viscosity: 1e3,
        drag: 1.0 - 1e-5,
        stiffness: 1e-2,
    };
}

