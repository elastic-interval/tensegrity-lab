/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */
use crate::messages::{ParameterType, PhysicsFeature, PhysicsParameter};

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
    pub muscle_nuance_increment: f32,
}

impl Physics {
    pub fn accept(&mut self, parameter: PhysicsParameter) {
        use PhysicsFeature::*;
        let PhysicsParameter {
            value,
            parameter_type,
            feature,
        } = parameter;
        match feature {
            Gravity => match parameter_type {
                ParameterType::Report => {}
                ParameterType::Set => self.gravity = value,
                ParameterType::Adjust => self.gravity *= value,
            },
            Pretense => {
                unimplemented!()
            }
            Stiffness => match parameter_type {
                ParameterType::Report => {}
                ParameterType::Set => self.stiffness = value,
                ParameterType::Adjust => self.stiffness *= value,
            },
            IterationsPerFrame => {
                unimplemented!()
            }
            MuscleIncrement => match parameter_type {
                ParameterType::Report => {}
                ParameterType::Set => self.muscle_nuance_increment = value,
                ParameterType::Adjust => self.muscle_nuance_increment *= value,
            },
            Viscosity => match parameter_type {
                ParameterType::Report => {}
                ParameterType::Set => self.viscosity = value,
                ParameterType::Adjust => self.viscosity *= value,
            },
            Drag => match parameter_type {
                ParameterType::Report => {}
                ParameterType::Set => self.drag = value,
                ParameterType::Adjust => self.drag *= value,
            },
        }
    }
}

pub mod presets {
    use crate::fabric::physics::Physics;
    use crate::fabric::physics::SurfaceCharacter::{Absent, Frozen};

    pub const LIQUID: Physics = Physics {
        surface_character: Absent,
        gravity: 0.0,
        antigravity: 0.0,
        viscosity: 1e4,
        drag: 1e-6,
        stiffness: 1e-3,
        muscle_nuance_increment: 0.0,
    };

    pub const PROTOTYPE_FORMATION: Physics = Physics {
        surface_character: Absent,
        gravity: 0.0,
        antigravity: 0.0,
        viscosity: 2e4,
        drag: 1e-3,
        stiffness: 1e-4,
        muscle_nuance_increment: 0.0,
    };

    pub const AIR_GRAVITY: Physics = Physics {
        surface_character: Frozen,
        gravity: 1e-7,
        antigravity: 1e-3,
        viscosity: 1e2,
        drag: 1e-4,
        stiffness: 0.05,
        muscle_nuance_increment: 0.0,
    };
}
