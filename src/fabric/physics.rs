/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */
use crate::units::{MillimetersPerMicrosecondSquared, EARTH_GRAVITY};
use crate::{PhysicsFeature, PhysicsParameter, Radio, StateChange};

/// Base number of physics iterations per frame (can be reduced adaptively)
pub const BASE_ITERATIONS_PER_FRAME: usize = 1200;


#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum SurfaceCharacter {
    #[default]
    Absent,
    Frozen,
    Sticky,
    Bouncy,
    Slippery,
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
    // Environmental damping (current values, modified during convergence)
    pub drag: f32,
    pub viscosity: f32,
    pub surface_character: SurfaceCharacter,
    
    // Base damping values (stored at convergence start)
    pub base_drag: f32,
    pub base_viscosity: f32,
    pub base_dt_scale: f32,
    
    // Base material properties (user-controlled, persistent)
    pub base_rigidity_scale: f32,
    pub base_mass_scale: f32,
    
    // Effective material properties (base × convergence multipliers)
    pub rigidity_scale: f32,
    pub mass_scale: f32,
    
    // Temporal parameters
    pub dt_scale: f32,
    pub iterations_per_frame: usize,
    
    // Legacy/UI parameters
    pub pretenst: f32,
    pub strain_limit: f32,
    
    // Convergence tracking
    pub convergence: Option<ConvergenceState>,
}

#[derive(Debug, Clone)]
pub struct ConvergenceState {
    pub enabled: bool,
    pub started: bool,
}

impl ConvergenceState {
    pub fn new(enabled: bool) -> Self {
        Self { 
            enabled,
            started: false,
        }
    }
}

impl Physics {
    pub fn accept(&mut self, parameter: PhysicsParameter) {
        use PhysicsFeature::*;
        let PhysicsParameter { feature, value } = parameter;
        match feature {
            Drag => self.drag = value,
            Pretenst => self.pretenst = value,
            StrainLimit => self.strain_limit = value,
            Viscosity => self.viscosity = value,
            MassScale => {
                self.base_mass_scale = value;
                self.update_effective_scales();
            },
            RigidityScale => {
                self.base_rigidity_scale = value;
                self.update_effective_scales();
            }
        }
    }
    
    /// Update effective scales from base scales
    pub fn update_effective_scales(&mut self) {
        self.rigidity_scale = self.base_rigidity_scale;
        self.mass_scale = self.base_mass_scale;
    }

    pub fn broadcast(&self, radio: &Radio) {
        use PhysicsFeature::*;
        let parameters = [
            Drag.parameter(self.drag),
            Pretenst.parameter(self.pretenst),
            StrainLimit.parameter(self.strain_limit),
            Viscosity.parameter(self.viscosity),
            MassScale.parameter(self.base_mass_scale),
            RigidityScale.parameter(self.base_rigidity_scale),
        ];
        for p in parameters {
            StateChange::SetPhysicsParameter(p).send(radio);
        }
    }

    pub fn iterations(&self) -> std::ops::Range<usize> {
        0..self.iterations_per_frame
    }
    
    /// Update convergence based on time progress (0.0 to 1.0)
    /// Gradually increases damping to slow the system down over time
    pub fn update_convergence_progress(&mut self, progress: f32) {
        // Store base values on first call
        if let Some(conv) = &mut self.convergence {
            if !conv.started {
                self.base_drag = self.drag;
                self.base_viscosity = self.viscosity;
                self.base_dt_scale = self.dt_scale;
                conv.started = true;
            }
        }
        
        // Calculate damping multiplier based on progress
        // progress^3 gives a smooth ramp-up that accelerates near the end
        let damping_mult = 1.0 + progress.powi(3) * 50.0;
        
        // Set values based on base, don't multiply repeatedly
        self.drag = self.base_drag * damping_mult;
        self.viscosity = self.base_viscosity * damping_mult;
        self.dt_scale = self.base_dt_scale * (1.0 + progress * 0.5);
    }
    
    /// Enable convergence tracking with default settings
    pub fn enable_convergence(&mut self) {
        self.convergence = Some(ConvergenceState::new(true));
    }
    
    /// Disable convergence tracking
    pub fn disable_convergence(&mut self) {
        if let Some(conv) = &mut self.convergence {
            conv.enabled = false;
        }
    }
    
}

pub mod presets {
    use crate::fabric::physics::{Physics, BASE_ITERATIONS_PER_FRAME};
    use crate::fabric::physics::SurfaceCharacter::{Absent, Frozen};

    pub const LIQUID: Physics = Physics {
        drag: 0.0125,
        viscosity: 40.0,
        surface_character: Absent,
        base_drag: 0.0125,
        base_viscosity: 40.0,
        base_dt_scale: 1.0,
        base_rigidity_scale: 0.00001,
        base_mass_scale: 0.00001,
        rigidity_scale: 0.0001,
        mass_scale: 0.0001,
        dt_scale: 1.0,
        iterations_per_frame: BASE_ITERATIONS_PER_FRAME,
        pretenst: 20.0,
        strain_limit: 1_000.0,
        convergence: None,
    };

    pub const AIR_GRAVITY: Physics = Physics {
        drag: 0.01,
        viscosity: 0.5,
        surface_character: Frozen,
        base_drag: 0.01,
        base_viscosity: 0.5,
        base_dt_scale: 1.0,
        base_rigidity_scale: 1.0,
        base_mass_scale: 1.0,
        rigidity_scale: 1.0,
        mass_scale: 1.0,
        dt_scale: 1.0,
        iterations_per_frame: BASE_ITERATIONS_PER_FRAME,
        pretenst: 3.0,
        strain_limit: 0.02,
        convergence: None,
    };

    pub const PRETENSING: Physics = Physics {
        drag: 25.0,
        viscosity: 4.0,
        surface_character: Absent,
        base_drag: 25.0,
        base_viscosity: 4.0,
        base_dt_scale: 1.0,
        base_rigidity_scale: 1.0,
        base_mass_scale: 1.0,
        rigidity_scale: 1.0,
        mass_scale: 1.0,
        dt_scale: 1.0,
        iterations_per_frame: BASE_ITERATIONS_PER_FRAME,
        pretenst: 3.0,
        strain_limit: 0.02,
        convergence: None,
    };
}
