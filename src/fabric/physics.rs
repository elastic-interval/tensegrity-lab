/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */
use crate::units::{MillimetersPerSecondSquared, Percent, EARTH_GRAVITY_MM_S2};
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

    pub fn acceleration_of_gravity(&self) -> MillimetersPerSecondSquared {
        match self {
            SurfaceCharacter::Absent => MillimetersPerSecondSquared(0.0),
            _ => EARTH_GRAVITY_MM_S2,
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
    pub drag: f32,
    pub tweak: Tweak,
}

/// Optional modification layer on top of base physics
#[derive(Debug, Clone)]
pub enum Tweak {
    None,
    Scaling(ScalingTweak),
    Construction(ConstructionTweak),
    Settling(SettlingTweak),
    Animation(AnimationTweak),
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
    pub time_contraction: f32,
}

#[derive(Debug, Clone)]
pub struct SettlingTweak {
    pub enabled: bool,
    pub started: bool,
    pub base_physics: Box<Physics>,
    pub drag: f32,
    pub viscosity: f32,
    pub time_scale_multiplier: f32,
}

impl SettlingTweak {
    pub fn new(physics: &Physics) -> Self {
        let mut base = physics.clone();
        base.tweak = Tweak::None;
        Self {
            enabled: true,
            started: false,
            base_physics: Box::new(base),
            drag: 0.0,
            viscosity: 0.0,
            time_scale_multiplier: 5.0,
        }
    }
}

/// Slow-motion physics for animation visualization
#[derive(Debug, Clone)]
pub struct AnimationTweak {
    pub time_contraction: f32,
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

        // Don't overwrite Construction or Settling tweaks with Scaling tweaks
        // Those tweaks provide essential damping and should not be replaced
        if matches!(self.tweak, Tweak::Construction(_) | Tweak::Settling(_)) {
            return;
        }

        // Get or create scaling tweak (only if not Construction/Settling)
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
            Tweak::Construction(c) => c.time_contraction,
            Tweak::Scaling(s) => s.time_scale,
            Tweak::Settling(s) => {
                // During settling, time scale increases geometrically to speed up settling
                s.base_physics.time_scale() * s.time_scale_multiplier
            }
            Tweak::Animation(a) => a.time_contraction,
            _ => 1.0,
        }
    }

    pub fn drag(&self) -> f32 {
        match &self.tweak {
            Tweak::Construction(c) => c.drag,
            Tweak::Settling(s) => s.drag,
            _ => self.drag,
        }
    }

    pub fn viscosity(&self) -> f32 {
        match &self.tweak {
            Tweak::Construction(c) => c.viscosity,
            Tweak::Settling(s) => s.viscosity,
            _ => 0.0,
        }
    }

    pub fn time_contraction(&self) -> f32 {
        match &self.tweak {
            Tweak::Construction(c) => c.time_contraction,
            Tweak::Settling(s) => s.base_physics.time_contraction(),
            Tweak::Animation(a) => a.time_contraction,
            _ => 1.0,
        }
    }

    pub fn update_settling_progress(&mut self, progress: f32) {
        if let Tweak::Settling(settling) = &mut self.tweak {
            if !settling.started {
                settling.started = true;
            }
            let damping_mult = 1.0 + progress.powi(3) * 50.0;
            const SETTLING_BASE_DRAG: f32 = 0.01;
            const SETTLING_BASE_VISCOSITY: f32 = 0.5;
            settling.drag = SETTLING_BASE_DRAG * damping_mult;
            settling.viscosity = SETTLING_BASE_VISCOSITY * damping_mult;
        }
    }

    pub fn enable_settling(&mut self) {
        self.tweak = Tweak::Settling(SettlingTweak::new(self));
    }

    pub fn disable_settling(&mut self) {
        if let Tweak::Settling(settling) = &self.tweak {
            if settling.enabled {
                let base = settling.base_physics.clone();
                *self = *base;
            }
        }
        self.tweak = Tweak::None;
    }
}


pub mod presets {
    use crate::fabric::physics::{AnimationTweak, ConstructionTweak, Physics, Tweak};
    use crate::fabric::physics::SurfaceCharacter::{Absent, Frozen};
    use crate::units::Percent;

    pub const CONSTRUCTION: Physics = Physics {
        surface_character: Absent,
        pretenst: Percent(20.0),
        drag: 1.0,
        tweak: Tweak::Construction(ConstructionTweak {
            drag: 0.0125,
            viscosity: 40.0,
            time_contraction: 2.0,
        }),
    };

    pub const PRETENSING: Physics = Physics {
        surface_character: Absent,
        pretenst: Percent(1.0),
        drag: 1.0,
        tweak: Tweak::Construction(ConstructionTweak {
            drag: 25.0,
            viscosity: 4.0,
            time_contraction: 2.0,
        }),
    };

    /// Physics for baking brick prototypes - extreme damping for settling under strain
    pub const BAKING: Physics = Physics {
        surface_character: Absent,
        pretenst: Percent(5.0),
        drag: 1.0,
        tweak: Tweak::Construction(ConstructionTweak {
            drag: 500.0,
            viscosity: 1000.0,
            time_contraction: 1.0,
        }),
    };

    /// Physics for animation - slow time for visible dynamics
    pub const ANIMATING: Physics = Physics {
        surface_character: Frozen,
        pretenst: Percent(1.0),
        drag: 0.5,
        tweak: Tweak::Animation(AnimationTweak {
            time_contraction: 1.0,
        }),
    };

    pub const BASE_PHYSICS: Physics = Physics {
        surface_character: Frozen,
        pretenst: Percent(1.0),
        drag: 0.5,
        tweak: Tweak::None,
    };
}

#[cfg(test)]
mod tests {
    use super::presets::*;

    #[test]
    fn test_time_scale_after_disable_settling() {
        let mut physics = CONSTRUCTION.clone();
        assert_eq!(physics.time_scale(), 5.0);
        physics.enable_settling();
        physics.disable_settling();
        assert_eq!(physics.time_scale(), 1.0);
    }
}
