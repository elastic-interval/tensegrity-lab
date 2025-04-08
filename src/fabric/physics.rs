/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */
use crate::messages::{PhysicsFeature, PhysicsParameter, Radio, StateChange};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum SurfaceCharacter {
    #[default]
    Absent,
    Frozen,
    Sticky,
    Bouncy,
}

impl SurfaceCharacter {
    pub fn has_gravity(&self) -> bool {
        !matches!(self, SurfaceCharacter::Absent)
    }

    pub fn gravity(&self) -> f32 {
        match self {
            SurfaceCharacter::Absent => 0.0,
            _ => 1e-7,
        }
    }

    pub fn antigravity(&self) -> f32 {
        match self {
            SurfaceCharacter::Absent => 0.0,
            _ => 1e-3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Physics {
    pub drag: f32,
    pub iterations_per_frame: f32,
    pub mass: f32,
    pub muscle_increment: f32,
    pub stiffness: f32,
    pub strain_limit: f32,
    pub surface_character: SurfaceCharacter,
    pub viscosity: f32,
}

impl Physics {
    pub fn accept(&mut self, parameter: PhysicsParameter) {
        use PhysicsFeature::*;
        let PhysicsParameter { feature, value } = parameter;
        match feature {
            Drag => self.drag = value,
            IterationsPerFrame => self.iterations_per_frame = value,
            Mass => self.mass = value,
            MuscleIncrement => self.muscle_increment = value,
            Pretense => {}
            Stiffness => self.stiffness = value,
            StrainLimit => self.strain_limit = value,
            Viscosity => self.viscosity = value,
        }
    }

    pub fn broadcast(&self, radio: &Radio) {
        use PhysicsFeature::*;
        let parameters = [
            Drag.parameter(self.drag),
            IterationsPerFrame.parameter(self.iterations_per_frame),
            Mass.parameter(self.mass),
            MuscleIncrement.parameter(self.muscle_increment),
            Stiffness.parameter(self.stiffness),
            StrainLimit.parameter(self.strain_limit),
            Viscosity.parameter(self.viscosity),
        ];
        for p in parameters {
            StateChange::SetPhysicsParameter(p).send(radio);
        }
    }

    pub fn iterations(&self) -> std::ops::Range<usize> {
        0..self.iterations_per_frame as usize
    }
}

pub mod presets {
    use crate::fabric::physics::Physics;
    use crate::fabric::physics::SurfaceCharacter::{Absent, Frozen};

    pub const LIQUID: Physics = Physics {
        drag: 1e-6,
        iterations_per_frame: 1000.0,
        mass: 1.0,
        muscle_increment: 0.0,
        stiffness: 1e-3,
        strain_limit: 1.0,
        surface_character: Absent,
        viscosity: 1e4,
    };

    pub const PROTOTYPE_FORMATION: Physics = Physics {
        drag: 1e-3,
        iterations_per_frame: 100.0,
        mass: 1.0,
        muscle_increment: 0.0,
        stiffness: 1e-4,
        strain_limit: 1.0,
        surface_character: Absent,
        viscosity: 2e4,
    };

    pub const AIR_GRAVITY: Physics = Physics {
        drag: 1e-4,
        iterations_per_frame: 100.0,
        mass: 1.0,
        muscle_increment: 0.0,
        stiffness: 0.05,
        strain_limit: 1.0,
        surface_character: Frozen,
        viscosity: 1e2,
    };
}
