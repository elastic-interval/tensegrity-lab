/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */
use crate::units::Percent;
use crate::{PhysicsFeature, PhysicsParameter, Radio, StateChange, TweakFeature, TweakParameter};


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SurfaceCharacter {
    Frozen,
    Sticky,
    Bouncy,
    Slippery,
}

use crate::fabric::Velocity;
use crate::units::EARTH_GRAVITY;
use cgmath::num_traits::zero;

/// Parameters for surface interaction calculation
pub struct SurfaceInteraction {
    pub altitude: f32,
    pub velocity: Velocity,
    pub force_velocity: Velocity,
    pub drag: f32,
    pub viscosity: f32,
    pub mass: f32,
    pub dt: f32,
}

/// Result of surface interaction
pub struct SurfaceResult {
    pub velocity: Velocity,
    pub clamp_y: Option<f32>,
}

const SURFACE_TOLERANCE: f32 = 0.01;
const STICKY_DOWN_DRAG_FACTOR: f32 = 0.8;

impl SurfaceCharacter {
    /// Apply surface physics and return the resulting velocity and optional y-position clamp
    /// All coordinates are now in meters directly
    pub fn interact(&self, s: SurfaceInteraction) -> SurfaceResult {
        let gravity = *EARTH_GRAVITY;
        let mut velocity = s.velocity;
        let mut clamp_y = None;

        if s.altitude > SURFACE_TOLERANCE {
            // Above surface - apply gravity and standard physics
            // gravity is m/s², dt is seconds, result is m/s velocity change
            velocity.y -= gravity * s.dt;
            let speed_squared = velocity.magnitude2();
            velocity += s.force_velocity - velocity * speed_squared * s.viscosity * s.dt;
            velocity *= 1.0 - s.drag * s.dt;
        } else {
            // On or below surface
            let depth = -s.altitude;
            let degree_submerged: f32 = depth.min(1.0);

            velocity += s.force_velocity;

            match self {
                SurfaceCharacter::Frozen => {
                    velocity = zero();
                    clamp_y = Some(0.0);
                }
                SurfaceCharacter::Sticky => {
                    let friction = if velocity.y < 0.0 {
                        STICKY_DOWN_DRAG_FACTOR
                    } else {
                        1.0 - s.drag * s.dt
                    };
                    velocity.x *= friction;
                    velocity.z *= friction;

                    let antigravity = gravity * s.mass * degree_submerged * 50.0;
                    velocity.y += antigravity * s.dt;

                    if velocity.y < 0.0 {
                        velocity.y *= 0.5;
                    }

                    if depth > 0.1 {
                        clamp_y = Some(-0.1);
                        velocity.y = 0.0;
                    }
                }
                SurfaceCharacter::Bouncy => {
                    if velocity.y < 0.0 {
                        velocity.y *= -0.5;
                    }

                    velocity.x *= 0.6;
                    velocity.z *= 0.6;

                    let antigravity = gravity * s.mass * degree_submerged * 5.0;
                    velocity.y += antigravity * s.dt;
                }
                SurfaceCharacter::Slippery => {
                    clamp_y = Some(0.0);
                    velocity.y = 0.0;

                    let speed_horizontal = (velocity.x * velocity.x + velocity.z * velocity.z).sqrt();

                    const SURFACE_DAMPING: f32 = 50.0;
                    let linear_friction = 1.0
                        - ((SURFACE_DAMPING + s.drag) * s.dt
                            + SURFACE_DAMPING * s.viscosity * speed_horizontal * s.dt);
                    let quadratic_damping = 1.0 - (2.0 * speed_horizontal * speed_horizontal * s.dt);
                    let total_friction = (linear_friction * quadratic_damping.max(0.0)).max(0.0);

                    velocity.x *= total_friction;
                    velocity.z *= total_friction;
                }
            }
        }

        SurfaceResult { velocity, clamp_y }
    }
}

use cgmath::InnerSpace;

/// Core physics environment with base values
#[derive(Debug, Clone)]
pub struct Physics {
    pub surface: Option<SurfaceCharacter>,
    pub pretenst: Percent,
    pub drag: f32,
    pub viscosity: f32,
    pub tweak: Tweak,
}

/// Multipliers applied on top of base physics values
#[derive(Debug, Clone)]
pub struct Tweak {
    pub drag_multiplier: f32,
    pub viscosity_multiplier: f32,
    pub mass_multiplier: f32,
    pub rigidity_multiplier: f32,
}

impl Default for Tweak {
    fn default() -> Self {
        Self {
            drag_multiplier: 1.0,
            viscosity_multiplier: 1.0,
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
        }
    }

    pub fn mass_multiplier(&self) -> f32 {
        self.tweak.mass_multiplier
    }

    pub fn rigidity_multiplier(&self) -> f32 {
        self.tweak.rigidity_multiplier
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
    use crate::units::Percent;

    const NO_TWEAK: Tweak = Tweak {
        drag_multiplier: 1.0,
        viscosity_multiplier: 1.0,
        mass_multiplier: 1.0,
        rigidity_multiplier: 1.0,
    };

    pub const VIEWING: Physics = Physics {
        surface: None,
        pretenst: Percent(1.0),
        drag: 0.5,
        viscosity: 0.0,
        tweak: NO_TWEAK,
    };

    pub const CONSTRUCTION: Physics = Physics {
        surface: None,
        pretenst: Percent(20.0),
        drag: 0.0125,
        viscosity: 40.0,
        tweak: NO_TWEAK,
    };

    pub const PRETENSING: Physics = Physics {
        surface: None,
        pretenst: Percent(1.0),
        drag: 25.0,
        viscosity: 4.0,
        tweak: NO_TWEAK,
    };

    pub const BAKING: Physics = Physics {
        surface: None,
        pretenst: Percent(5.0),
        drag: 500.0,
        viscosity: 1000.0,
        tweak: NO_TWEAK,
    };

    pub const ANIMATING: Physics = Physics {
        surface: None,
        pretenst: Percent(1.0),
        drag: 0.5,
        viscosity: 0.0,
        tweak: NO_TWEAK,
    };

    pub const FALLING: Physics = Physics {
        surface: None,
        pretenst: Percent(1.0),
        drag: 0.5,
        viscosity: 0.0,
        tweak: NO_TWEAK,
    };

    pub const SETTLING: Physics = Physics {
        surface: None,
        pretenst: Percent(1.0),
        drag: 0.01,
        viscosity: 0.5,
        tweak: NO_TWEAK,
    };
}

#[cfg(test)]
mod tests {
    use super::presets::*;

    #[test]
    fn test_values_with_multipliers() {
        let mut physics = VIEWING.clone();
        assert_eq!(physics.drag(), 0.5);
        assert_eq!(physics.viscosity(), 0.0);

        physics.tweak.drag_multiplier = 3.0;
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
