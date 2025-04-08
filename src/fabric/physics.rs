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
    pub fn antigravity(&self) -> f32 {
        match self {
            SurfaceCharacter::Absent => 0.0,
            _ => 1e-3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Physics {
    pub surface_character: SurfaceCharacter,
    pub iterations_per_frame: f32,
    pub gravity: f32,
    pub mass: f32,
    pub viscosity: f32,
    pub drag: f32,
    pub stiffness: f32,
    pub muscle_increment: f32,
}

impl Physics {
    pub fn accept(&mut self, parameter: PhysicsParameter) {
        use PhysicsFeature::*;
        let PhysicsParameter { feature, value } = parameter;
        match feature {
            Gravity => self.gravity = value,
            Mass => self.mass = value,
            Stiffness => self.stiffness = value,
            IterationsPerFrame => self.iterations_per_frame = value,
            MuscleIncrement => self.muscle_increment = value,
            Viscosity => self.viscosity = value,
            Drag => self.drag = value,
            Pretense => {}
        }
    }

    pub fn broadcast(&self, radio: &Radio) {
        use PhysicsFeature::*;
        use StateChange::SetPhysicsParameter;
        SetPhysicsParameter(Gravity.parameter(self.gravity)).send(radio);
        SetPhysicsParameter(Mass.parameter(self.mass)).send(radio);
        SetPhysicsParameter(Stiffness.parameter(self.stiffness)).send(radio);
        SetPhysicsParameter(IterationsPerFrame.parameter(self.iterations_per_frame)).send(radio);
        SetPhysicsParameter(MuscleIncrement.parameter(self.muscle_increment)).send(radio);
        SetPhysicsParameter(Viscosity.parameter(self.viscosity)).send(radio);
        SetPhysicsParameter(Drag.parameter(self.drag)).send(radio);
    }

    pub fn iterations(&self) -> std::ops::Range<usize> {
        0..self.iterations_per_frame as usize
    }
}

pub mod presets {
    use crate::fabric::physics::Physics;
    use crate::fabric::physics::SurfaceCharacter::{Absent, Frozen};

    pub const LIQUID: Physics = Physics {
        surface_character: Absent,
        iterations_per_frame: 1000.0,
        gravity: 0.0,
        mass: 1.0,
        viscosity: 1e4,
        drag: 1e-6,
        stiffness: 1e-3,
        muscle_increment: 0.0,
    };

    pub const PROTOTYPE_FORMATION: Physics = Physics {
        surface_character: Absent,
        iterations_per_frame: 100.0,
        gravity: 0.0,
        mass: 1.0,
        viscosity: 2e4,
        drag: 1e-3,
        stiffness: 1e-4,
        muscle_increment: 0.0,
    };

    pub const AIR_GRAVITY: Physics = Physics {
        surface_character: Frozen,
        iterations_per_frame: 100.0,
        gravity: 1e-7,
        mass: 1.0,
        viscosity: 1e2,
        drag: 1e-4,
        stiffness: 0.05,
        muscle_increment: 0.0,
    };
}
