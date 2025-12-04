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

/// Core physics environment with base values
#[derive(Debug, Clone)]
pub struct Physics {
    pub surface_character: SurfaceCharacter,
    pub pretenst: Percent,
    pub drag: f32,
    pub viscosity: f32,
    pub time_scale: f32,
    pub tweak: Tweak,
}

/// Multipliers applied on top of base physics values
#[derive(Debug, Clone)]
pub struct Tweak {
    pub drag_multiplier: f32,
    pub viscosity_multiplier: f32,
    pub time_scale_multiplier: f32,
    pub mass_multiplier: f32,
    pub rigidity_multiplier: f32,
}

impl Default for Tweak {
    fn default() -> Self {
        Self {
            drag_multiplier: 1.0,
            viscosity_multiplier: 1.0,
            time_scale_multiplier: 1.0,
            mass_multiplier: 1.0,
            rigidity_multiplier: 1.0,
        }
    }
}

impl Tweak {
    pub fn none() -> Self {
        Self::default()
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

        self.broadcast(radio);

        let tweak_params = [
            MassScale.parameter(self.tweak.mass_multiplier),
            RigidityScale.parameter(self.tweak.rigidity_multiplier),
            TimeScale.parameter(self.tweak.time_scale_multiplier),
        ];
        for p in tweak_params {
            StateChange::SetTweakParameter(p).send(radio);
        }
    }

    pub fn accept_tweak(&mut self, parameter: TweakParameter) {
        use TweakFeature::*;
        let TweakParameter { feature, value } = parameter;

        match feature {
            MassScale => self.tweak.mass_multiplier = value,
            RigidityScale => self.tweak.rigidity_multiplier = value,
            TimeScale => self.tweak.time_scale_multiplier = value,
        }
    }

    pub fn mass_multiplier(&self) -> f32 {
        self.tweak.mass_multiplier
    }

    pub fn rigidity_multiplier(&self) -> f32 {
        self.tweak.rigidity_multiplier
    }

    pub fn time_scale(&self) -> f32 {
        self.time_scale * self.tweak.time_scale_multiplier
    }

    pub fn drag(&self) -> f32 {
        self.drag * self.tweak.drag_multiplier
    }

    pub fn viscosity(&self) -> f32 {
        self.viscosity * self.tweak.viscosity_multiplier
    }

    pub fn update_settling_multipliers(&mut self, progress: f32) {
        let damping_mult = 1.0 + progress.powi(3) * 50.0;
        self.tweak.drag_multiplier = damping_mult;
        self.tweak.viscosity_multiplier = damping_mult;
    }
}


pub mod presets {
    use crate::fabric::physics::{Physics, Tweak};
    use crate::fabric::physics::SurfaceCharacter::{Absent, Frozen};
    use crate::units::Percent;

    const NO_TWEAK: Tweak = Tweak {
        drag_multiplier: 1.0,
        viscosity_multiplier: 1.0,
        time_scale_multiplier: 1.0,
        mass_multiplier: 1.0,
        rigidity_multiplier: 1.0,
    };

    pub const VIEWING: Physics = Physics {
        surface_character: Frozen,
        pretenst: Percent(1.0),
        drag: 0.5,
        viscosity: 0.0,
        time_scale: 1.0,
        tweak: NO_TWEAK,
    };

    pub const CONSTRUCTION: Physics = Physics {
        surface_character: Absent,
        pretenst: Percent(20.0),
        drag: 0.0125,
        viscosity: 40.0,
        time_scale: 2.0,
        tweak: NO_TWEAK,
    };

    pub const PRETENSING: Physics = Physics {
        surface_character: Absent,
        pretenst: Percent(1.0),
        drag: 25.0,
        viscosity: 4.0,
        time_scale: 2.0,
        tweak: NO_TWEAK,
    };

    pub const BAKING: Physics = Physics {
        surface_character: Absent,
        pretenst: Percent(5.0),
        drag: 500.0,
        viscosity: 1000.0,
        time_scale: 1.0,
        tweak: NO_TWEAK,
    };

    pub const ANIMATING: Physics = Physics {
        surface_character: Frozen,
        pretenst: Percent(1.0),
        drag: 0.5,
        viscosity: 0.0,
        time_scale: 1.0,
        tweak: NO_TWEAK,
    };

    pub const FALLING: Physics = Physics {
        surface_character: Frozen,
        pretenst: Percent(1.0),
        drag: 0.5,
        viscosity: 0.0,
        time_scale: 1.0,
        tweak: NO_TWEAK,
    };

    pub const SETTLING: Physics = Physics {
        surface_character: Frozen,
        pretenst: Percent(1.0),
        drag: 0.01,
        viscosity: 0.5,
        time_scale: 5.0,
        tweak: NO_TWEAK,
    };
}

#[cfg(test)]
mod tests {
    use super::presets::*;

    #[test]
    fn test_values_with_multipliers() {
        let mut physics = VIEWING.clone();
        assert_eq!(physics.time_scale(), 1.0);
        assert_eq!(physics.drag(), 0.5);
        assert_eq!(physics.viscosity(), 0.0);

        physics.tweak.time_scale_multiplier = 2.0;
        physics.tweak.drag_multiplier = 3.0;
        assert_eq!(physics.time_scale(), 2.0);
        assert_eq!(physics.drag(), 1.5);
    }

    #[test]
    fn test_settling_multipliers() {
        let mut physics = SETTLING.clone();
        assert_eq!(physics.drag(), 0.01);
        assert_eq!(physics.viscosity(), 0.5);

        physics.update_settling_multipliers(1.0);
        assert_eq!(physics.tweak.drag_multiplier, 51.0);
        assert_eq!(physics.tweak.viscosity_multiplier, 51.0);
        assert_eq!(physics.drag(), 0.51);
        assert_eq!(physics.viscosity(), 25.5);
    }
}
