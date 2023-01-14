/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::physics::SurfaceCharacter::{Absent, Frozen};

pub enum Environment {
    Liquid,
    AirGravity,
}

#[derive(Debug, Clone, Copy)]
pub enum SurfaceCharacter {
    Absent,
    Frozen,
    Sticky,
    Bouncy,
}

pub struct Physics {
    pub surface_character: SurfaceCharacter,
    pub gravity: f32,
    pub antigravity: f32,
    pub viscosity: f32,
    pub stiffness: f32,
}

impl Physics {
    pub fn new(environment: Environment) -> Self {
        match environment {
            Environment::Liquid => Self {
                surface_character: Absent,
                gravity: 0.0,
                antigravity: 0.0,
                viscosity: 1e4,
                stiffness: 5e-5,
            },
            Environment::AirGravity => Self{
                surface_character: Frozen,
                gravity: 2e-8,
                antigravity: 1e-3,
                viscosity: 1e3,
                stiffness: 1e-2,
            },
        }
    }
}
