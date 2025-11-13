/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */
use crate::{PhysicsFeature, PhysicsParameter, Radio, StateChange};

/// Number of physics iterations per frame (constant across all physics presets)
pub const ITERATIONS_PER_FRAME: usize = 100;

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

    pub fn force_of_gravity(&self, mass: f32) -> f32 {
        match self {
            SurfaceCharacter::Absent => 0.0,
            _ => {
                let gravity_per_gram = 5e-7;
                mass * gravity_per_gram
            }
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
    pub cycle_ticks: f32,
    pub pretenst: f32,
    pub stiffness_factor: f32,
    pub mass_factor: f32,
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
            CycleTicks => self.cycle_ticks = value,
            Pretenst => self.pretenst = value,
            Stiffness => self.stiffness_factor = value,
            MassFactor => self.mass_factor = value,
            StrainLimit => self.strain_limit = value,
            Viscosity => self.viscosity = value,
        }
    }

    pub fn broadcast(&self, radio: &Radio) {
        use PhysicsFeature::*;
        let parameters = [
            Drag.parameter(self.drag),
            CycleTicks.parameter(self.cycle_ticks),
            Stiffness.parameter(self.stiffness_factor),
            MassFactor.parameter(self.mass_factor),
            Pretenst.parameter(self.pretenst),
            StrainLimit.parameter(self.strain_limit),
            Viscosity.parameter(self.viscosity),
        ];
        for p in parameters {
            StateChange::SetPhysicsParameter(p).send(radio);
        }
    }

    pub fn iterations(&self) -> std::ops::Range<usize> {
        0..ITERATIONS_PER_FRAME
    }
}

pub mod presets {
    use crate::fabric::physics::Physics;
    use crate::fabric::physics::SurfaceCharacter::{Absent, Frozen};

    pub const LIQUID: Physics = Physics {
        drag: 5e-6,
        cycle_ticks: 1000.0,
        stiffness_factor: 1e-2,
        mass_factor: 1.0,
        pretenst: 20.0, // not used
        strain_limit: 1_000.0,
        surface_character: Absent,
        viscosity: 1e5,
    };

    pub const PROTOTYPE_FORMATION: Physics = Physics {
        drag: 1e-3,
        cycle_ticks: 1000.0,
        stiffness_factor: 1e-2,
        mass_factor: 1.0,
        pretenst: 1.0,
        strain_limit: 1_000.0,
        surface_character: Absent,
        viscosity: 2e4,
    };

    pub const AIR_GRAVITY: Physics = Physics {
        drag: 1e-5,
        cycle_ticks: 1000.0,
        stiffness_factor: 1.0,
        mass_factor: 1.0,
        pretenst: 2.0,
        strain_limit: 0.02,
        surface_character: Frozen,
        viscosity: 1e2,
    };

    pub const PRETENSING: Physics = Physics {
        drag: 1e-1,
        surface_character: Absent,
        viscosity: 1e5,
        ..AIR_GRAVITY
    };
}
