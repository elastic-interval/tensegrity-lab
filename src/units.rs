/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

//! Physical units for tensegrity simulation
//!
//! This module provides type-safe wrappers for physical quantities,
//! making the physics more intuitive and preventing unit errors.

use std::ops::{Add, AddAssign, Div, Mul};

/// Trait for unit types that wrap f32 values
pub trait Unit {
    fn f32(self) -> f32;
}

macro_rules! unit {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        pub struct $name(pub f32);

        impl Unit for $name {
            fn f32(self) -> f32 {
                self.0
            }
        }
    };
    ($name:ident, Default) => {
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
        pub struct $name(pub f32);

        impl Unit for $name {
            fn f32(self) -> f32 {
                self.0
            }
        }
    };
}

unit!(Grams);
unit!(Meters);
unit!(Newtons);
unit!(MetersPerSecondSquared);
unit!(Seconds);
unit!(Percent, Default);
unit!(Degrees, Default);
unit!(GramsPerMeter);
unit!(NewtonsPerMeter);

pub const IMMEDIATE: Seconds = Seconds(0.0);
pub const MOMENT: Seconds = Seconds(0.1);

// Physical constants

/// Standard Earth gravity: 9.81 m/s²
pub const EARTH_GRAVITY: MetersPerSecondSquared = MetersPerSecondSquared(9.81);

pub const MM_PER_METER: f32 = 1000.0;
pub const GRAMS_PER_KG: f32 = 1000.0;
pub const MICROSECONDS_PER_SECOND: f32 = 1_000_000.0;

// Conversion implementations

impl Grams {
    pub fn to_kg(self) -> f32 {
        self.0 / GRAMS_PER_KG
    }

    pub fn from_kg(kg: f32) -> Self {
        Self(kg * GRAMS_PER_KG)
    }
}

impl Meters {
    pub fn to_mm(self) -> f32 {
        self.0 * MM_PER_METER
    }
}

impl Seconds {
    pub fn to_microseconds(self) -> f32 {
        self.0 * MICROSECONDS_PER_SECOND
    }

    pub fn from_microseconds(us: f32) -> Self {
        Self(us / MICROSECONDS_PER_SECOND)
    }
}

impl Percent {
    /// Convert percentage to a factor (0.0-1.0)
    /// Example: 50% → 0.5, 100% → 1.0
    pub fn as_factor(self) -> f32 {
        self.0 / 100.0
    }

    /// Create from a factor (0.0-1.0)
    /// Example: 0.5 → 50%, 1.0 → 100%
    pub fn from_factor(factor: f32) -> Self {
        Self(factor * 100.0)
    }
}

// Arithmetic operations for dimensional analysis

impl Div<Grams> for Newtons {
    type Output = f32; // m/s² (acceleration)

    fn div(self, mass: Grams) -> f32 {
        // F = ma, so a = F/m
        // Newton = kg⋅m/s², Grams needs conversion to kg
        self.0 / mass.to_kg()
    }
}

impl Mul<f32> for Grams {
    type Output = Grams;

    fn mul(self, scalar: f32) -> Grams {
        Grams(self.f32() * scalar)
    }
}

impl Div<f32> for Grams {
    type Output = Grams;

    fn div(self, scalar: f32) -> Grams {
        Grams(self.f32() / scalar)
    }
}

impl Div<f32> for Newtons {
    type Output = Newtons;

    fn div(self, scalar: f32) -> Newtons {
        Newtons(self.f32() / scalar)
    }
}

impl Add for Grams {
    type Output = Grams;

    fn add(self, other: Grams) -> Grams {
        Grams(self.f32() + other.f32())
    }
}

impl AddAssign for Grams {
    fn add_assign(&mut self, other: Grams) {
        self.0 += other.f32();
    }
}

impl Add for Meters {
    type Output = Meters;

    fn add(self, other: Meters) -> Meters {
        Meters(self.f32() + other.f32())
    }
}

impl Mul<f32> for Meters {
    type Output = Meters;

    fn mul(self, scalar: f32) -> Meters {
        Meters(self.f32() * scalar)
    }
}

impl Mul<Meters> for f32 {
    type Output = Meters;

    fn mul(self, meters: Meters) -> Meters {
        Meters(self * meters.f32())
    }
}

impl Div<f32> for Meters {
    type Output = Meters;

    fn div(self, scalar: f32) -> Meters {
        Meters(self.f32() / scalar)
    }
}

// Meters / Meters = f32 (dimensionless ratio)
impl Div<Meters> for Meters {
    type Output = f32;

    fn div(self, other: Meters) -> f32 {
        self.f32() / other.f32()
    }
}

// NewtonsPerMeter * Meters = Newtons (F = k * x)
impl Mul<Meters> for NewtonsPerMeter {
    type Output = Newtons;

    fn mul(self, extension: Meters) -> Newtons {
        Newtons(self.f32() * extension.f32())
    }
}

// GramsPerMeter * Meters = Grams (mass = density * length)
impl Mul<Meters> for GramsPerMeter {
    type Output = Grams;

    fn mul(self, length: Meters) -> Grams {
        Grams(self.f32() * length.f32())
    }
}

// Display implementations

impl std::fmt::Display for Grams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}g", self.0)
    }
}

impl std::fmt::Display for Meters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.3}m", self.0)
    }
}

impl std::fmt::Display for Newtons {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.3}N", self.0)
    }
}

impl std::fmt::Display for NewtonsPerMeter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2e}N/m", self.0)
    }
}

impl std::fmt::Display for Seconds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}s", self.0)
    }
}

impl std::fmt::Display for Percent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}%", self.0)
    }
}

impl std::fmt::Display for Degrees {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}°", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mass_conversions() {
        let mass = Grams(1000.0);
        assert_eq!(mass.to_kg(), 1.0);

        let mass2 = Grams::from_kg(2.5);
        assert_eq!(mass2.0, 2500.0);
    }

    #[test]
    fn test_gravity_constant() {
        assert_eq!(EARTH_GRAVITY.f32(), 9.81);
    }

    #[test]
    fn test_linear_density() {
        let density = GramsPerMeter(10.0);
        let length = Meters(0.1);
        let mass = density * length;
        assert_eq!(mass.f32(), 1.0);
    }

    #[test]
    fn test_time_conversion() {
        let dt = Seconds::from_microseconds(250.0);
        assert!((dt.f32() - 0.00025).abs() < 1e-9);
        assert!((dt.to_microseconds() - 250.0).abs() < 1e-3);
    }

    #[test]
    fn test_spring_force() {
        let spring_constant = NewtonsPerMeter(100.0);
        let extension = Meters(0.1);
        let force = spring_constant * extension;
        assert_eq!(force.f32(), 10.0);
    }

    #[test]
    fn test_force_division() {
        let force = Newtons(9.81);
        let mass = Grams(1000.0);
        let acceleration = force / mass;
        assert!((acceleration - 9.81).abs() < 1e-6);
    }
}
