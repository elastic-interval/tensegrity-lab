/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */
use crate::units::{MillimetersPerMicrosecondSquared, Percent, EARTH_GRAVITY};
use crate::{PhysicsFeature, PhysicsParameter, Radio, StateChange, TweakFeature, TweakParameter};


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

/// Core physics environment - what the world is like
#[derive(Debug, Clone)]
pub struct Physics {
    pub surface_character: SurfaceCharacter,
    pub pretenst: Percent,
    pub tweak: Tweak,
}

/// Optional modification layer on top of base physics
#[derive(Debug, Clone)]
pub enum Tweak {
    None,
    Scaling(ScalingTweak),
    Construction(ConstructionTweak),
    Convergence(ConvergenceTweak),
}

/// User-controlled scaling for experimentation
#[derive(Debug, Clone)]
pub struct ScalingTweak {
    pub mass_scale: f32,
    pub rigidity_scale: f32,
    pub time_scale: f32,
}

/// Fixed damping for construction phases
#[derive(Debug, Clone)]
pub struct ConstructionTweak {
    pub drag: f32,
    pub viscosity: f32,
}

impl ConstructionTweak {
    pub fn new(drag: f32, viscosity: f32) -> Self {
        Self { drag, viscosity }
    }
}

/// Temporary automated modifications to help structures settle
#[derive(Debug, Clone)]
pub struct ConvergenceTweak {
    pub enabled: bool,
    pub started: bool,
    pub base_physics: Box<Physics>,
    pub drag: f32,           // Computed convergence drag
    pub viscosity: f32,      // Computed convergence viscosity
    pub time_scale_multiplier: f32,
}

impl ConvergenceTweak {
    pub fn new(physics: &Physics) -> Self {
        // Clone physics but without tweak to avoid recursion
        let mut base = physics.clone();
        base.tweak = Tweak::None;

        Self {
            enabled: true,
            started: false,
            base_physics: Box::new(base),
            drag: 0.0,        // Start with no damping
            viscosity: 0.0,   // Start with no damping
            time_scale_multiplier: 1.0,
        }
    }
}

impl ScalingTweak {
    pub fn new(mass_scale: f32, rigidity_scale: f32, time_scale: f32) -> Self {
        Self {
            mass_scale,
            rigidity_scale,
            time_scale,
        }
    }
}

impl Physics {
    pub fn accept(&mut self, parameter: PhysicsParameter) {
        use PhysicsFeature::*;
        let PhysicsParameter { feature, value } = parameter;
        match feature {
            Pretenst => self.pretenst = Percent(value),
        }
    }

    pub fn broadcast(&self, radio: &Radio) {
        use PhysicsFeature::*;

        let physics_params = [
            Pretenst.parameter(*self.pretenst),
        ];
        for p in physics_params {
            StateChange::SetPhysicsParameter(p).send(radio);
        }
    }
    
    pub fn broadcast_with_tweaks(&self, radio: &Radio) {
        use TweakFeature::*;
        
        // Broadcast physics first
        self.broadcast(radio);
        
        // Then broadcast tweaks if present
        if let Tweak::Scaling(s) = &self.tweak {
            let tweak_params = [
                MassScale.parameter(s.mass_scale),
                RigidityScale.parameter(s.rigidity_scale),
                TimeScale.parameter(s.time_scale),
            ];
            for p in tweak_params {
                StateChange::SetTweakParameter(p).send(radio);
            }
        }
    }

    /// Accept a tweak parameter (mass/rigidity scaling)
    pub fn accept_tweak(&mut self, parameter: TweakParameter) {
        use TweakFeature::*;
        let TweakParameter { feature, value } = parameter;

        // Don't overwrite Construction or Convergence tweaks with Scaling tweaks
        // Those tweaks provide essential damping and should not be replaced
        if matches!(self.tweak, Tweak::Construction(_) | Tweak::Convergence(_)) {
            return;
        }

        // Get or create scaling tweak (only if not Construction/Convergence)
        let scaling = match &mut self.tweak {
            Tweak::Scaling(s) => s,
            _ => {
                self.tweak = Tweak::Scaling(ScalingTweak::new(1.0, 1.0, 1.0));
                if let Tweak::Scaling(s) = &mut self.tweak {
                    s
                } else {
                    unreachable!()
                }
            }
        };
        
        match feature {
            MassScale => scaling.mass_scale = value,
            RigidityScale => scaling.rigidity_scale = value,
            TimeScale => scaling.time_scale = value,
        }
    }
    
    /// Get mass scale (from tweak or default 1.0)
    pub fn mass_scale(&self) -> f32 {
        match &self.tweak {
            Tweak::Scaling(s) => s.mass_scale,
            _ => 1.0,
        }
    }
    
    /// Get rigidity scale (from tweak or default 1.0)
    pub fn rigidity_scale(&self) -> f32 {
        match &self.tweak {
            Tweak::Scaling(s) => s.rigidity_scale,
            _ => 1.0,
        }
    }
    
    /// Get time scale (from tweak or default 1.0)
    pub fn time_scale(&self) -> f32 {
        match &self.tweak {
            Tweak::Scaling(s) => s.time_scale,
            Tweak::Convergence(c) => {
                // During convergence, time scale increases geometrically to speed up settling
                c.base_physics.time_scale() * c.time_scale_multiplier
            }
            _ => 1.0,
        }
    }

    /// Get drag coefficient (0.0 normally, construction/convergence value when tweaked)
    pub fn drag(&self) -> f32 {
        match &self.tweak {
            Tweak::Construction(c) => c.drag,
            Tweak::Convergence(c) => c.drag,
            _ => 0.0,
        }
    }

    /// Get viscosity coefficient (0.0 normally, construction/convergence value when tweaked)
    pub fn viscosity(&self) -> f32 {
        match &self.tweak {
            Tweak::Construction(c) => c.viscosity,
            Tweak::Convergence(c) => c.viscosity,
            _ => 0.0,
        }
    }

    /// Update convergence based on time progress (0.0 to 1.0)
    /// Gradually increases damping to slow the system down over time
    /// Time scale remains constant at 1.0 to match UI timing
    pub fn update_convergence_progress(&mut self, progress: f32) {
        if let Tweak::Convergence(conv) = &mut self.tweak {
            if !conv.started {
                conv.started = true;
            }

            // Apply progressive damping during convergence
            // This gradually slows the system down
            let damping_mult = 1.0 + progress.powi(3) * 50.0;

            // Convergence-specific base damping values (independent of BASE_PHYSICS)
            // These are tuned for convergence behavior
            const CONVERGENCE_BASE_DRAG: f32 = 0.01;
            const CONVERGENCE_BASE_VISCOSITY: f32 = 0.5;

            // Keep time scale constant at 1.0 (no speedup)
            // This ensures fabric time matches what user sees in UI
            let time_scale_mult = 1.0;

            // Compute and store convergence damping values
            conv.drag = CONVERGENCE_BASE_DRAG * damping_mult;
            conv.viscosity = CONVERGENCE_BASE_VISCOSITY * damping_mult;
            conv.time_scale_multiplier = time_scale_mult;
        }
    }
    
    /// Enable convergence tracking
    pub fn enable_convergence(&mut self) {
        self.tweak = Tweak::Convergence(ConvergenceTweak::new(self));
    }
    
    /// Disable convergence tracking
    pub fn disable_convergence(&mut self) {
        if let Tweak::Convergence(conv) = &self.tweak {
            if conv.enabled {
                // Restore base physics
                let base = conv.base_physics.clone();
                *self = *base;
            }
        }
        self.tweak = Tweak::None;
    }
}


pub mod presets {
    use crate::fabric::physics::{ConstructionTweak, Physics, Tweak};
    use crate::fabric::physics::SurfaceCharacter::{Absent, Frozen};
    use crate::units::Percent;

    pub const CONSTRUCTION: Physics = Physics {
        surface_character: Absent,
        pretenst: Percent(20.0),
        tweak: Tweak::Construction(ConstructionTweak {
            drag: 0.0125,
            viscosity: 40.0,
        }),
    };

    pub const PRETENSING: Physics = Physics {
        surface_character: Absent,
        pretenst: Percent(1.0),
        tweak: Tweak::Construction(ConstructionTweak {
            drag: 25.0,
            viscosity: 4.0,
        }),
    };

    pub const BASE_PHYSICS: Physics = Physics {
        surface_character: Frozen,
        pretenst: Percent(1.0),
        tweak: Tweak::None,
    };
}
