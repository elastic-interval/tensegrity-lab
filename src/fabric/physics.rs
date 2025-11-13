/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */
use crate::units::{MillimetersPerMicrosecondSquared, EARTH_GRAVITY};
use crate::{PhysicsFeature, PhysicsParameter, Radio, StateChange};

/// Number of physics iterations per frame (constant across all physics presets)
pub const ITERATIONS_PER_FRAME: usize = 100;

/// Time step for physics integration (normalized to 1.0 per iteration)
/// All physics constants are calibrated for DT=1.0
/// Adjusting this will affect the simulation speed and may require recalibration
pub const DT: f32 = 0.001;

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

    pub fn force_of_gravity(&self, mass: f32) -> MillimetersPerMicrosecondSquared {
        match self {
            SurfaceCharacter::Absent => MillimetersPerMicrosecondSquared(0.0),
            _ => MillimetersPerMicrosecondSquared(mass * *EARTH_GRAVITY),
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
    pub rigidity_factor: f32,
    pub mass_factor: f32,
    pub strain_limit: f32,
    pub surface_character: SurfaceCharacter,
    pub viscosity: f32,
}

impl Physics {
    /// Get rigidity factor compensated for DT to maintain stability
    /// Larger time steps need softer springs to avoid overshooting
    /// Scales by DT² because spring acceleration error grows quadratically
    pub fn effective_rigidity_factor(&self) -> f32 {
        self.rigidity_factor / DT / DT
    }

    /// Get viscosity compensated for DT to maintain damping
    /// Larger time steps need more damping to absorb energy
    pub fn effective_viscosity(&self) -> f32 {
        self.viscosity * DT
    }

    /// Get gravity scaling factor compensated for DT
    /// Gravity should scale inversely with DT to maintain same fall rate
    pub fn effective_gravity_factor(&self) -> f32 {
        1.0 / DT.powf(1.6)
    }

    /// Get drag scaling factor compensated for DT
    /// Drag is exponential decay per iteration, needs to scale with DT
    pub fn effective_drag_factor(&self) -> f32 {
        1.0 / DT
    }

    pub fn accept(&mut self, parameter: PhysicsParameter) {
        use PhysicsFeature::*;
        let PhysicsParameter { feature, value } = parameter;
        match feature {
            Drag => self.drag = value,
            CycleTicks => self.cycle_ticks = value,
            Pretenst => self.pretenst = value,
            Rigidity => self.rigidity_factor = value,
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
            Rigidity.parameter(self.rigidity_factor),
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
        rigidity_factor: 0.51,
        mass_factor: 51.0,
        pretenst: 20.0, // not used
        strain_limit: 1_000.0,
        surface_character: Absent,
        viscosity: 1e5,
    };

    pub const PROTOTYPE_FORMATION: Physics = Physics {
        drag: 1e-3,
        cycle_ticks: 1000.0,
        rigidity_factor: 0.51,
        mass_factor: 51.0,
        pretenst: 1.0,
        strain_limit: 1_000.0,
        surface_character: Absent,
        viscosity: 2e4,
    };

    pub const AIR_GRAVITY: Physics = Physics {
        drag: 1e-5,
        cycle_ticks: 1000.0,
        rigidity_factor: 51.0,
        mass_factor: 51.0,
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
