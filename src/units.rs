/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

//! Physical units for tensegrity simulation
//!
//! This module provides type-safe wrappers for physical quantities,
//! making the physics more intuitive and preventing unit errors.

use std::ops::{Add, AddAssign, Deref, Div, Mul};

/// Mass in grams
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Grams(pub f32);

/// Length in meters
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Meters(pub f32);

/// Force in Newtons
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Newtons(pub f32);

/// Acceleration in meters per second squared
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct MetersPerSecondSquared(pub f32);

impl Deref for MetersPerSecondSquared {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Time in seconds
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Seconds(pub f32);

/// Percentage value (0-100)
/// Provides type-safe conversion to factors (0.0-1.0)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Percent(pub f32);

/// Angle in degrees
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Degrees(pub f32);

// Common time constants
pub const IMMEDIATE: Seconds = Seconds(0.0);
pub const MOMENT: Seconds = Seconds(0.1);

/// Linear density (mass per unit length) in grams per meter
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct GramsPerMeter(pub f32);

/// Spring constant in Newtons per meter (N/m)
/// Standard SI unit for spring stiffness: k in F = kx
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct NewtonsPerMeter(pub f32);

// Deref implementations for ergonomic access to inner values

impl Deref for Grams {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for Meters {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for Newtons {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for NewtonsPerMeter {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for GramsPerMeter {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for Percent {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for Degrees {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Physical constants

/// Standard Earth gravity: 9.81 m/s²
pub const EARTH_GRAVITY: MetersPerSecondSquared = MetersPerSecondSquared(9.81);

/// Millimeters per meter - for converting meter coordinates to millimeters
pub const MM_PER_METER: f32 = 1000.0;

// Conversion implementations

impl Grams {
    /// Convert grams to kilograms
    pub fn to_kg(self) -> f32 {
        self.0 / 1000.0
    }

    /// Create from kilograms
    pub fn from_kg(kg: f32) -> Self {
        Self(kg * 1000.0)
    }
}

impl Meters {
    /// Convert meters to millimeters (for display purposes)
    pub fn to_mm(self) -> f32 {
        self.0 * 1000.0
    }
}

impl Seconds {
    /// Convert seconds to microseconds
    pub fn to_microseconds(self) -> f32 {
        self.0 * 1_000_000.0
    }

    /// Create from microseconds
    pub fn from_microseconds(us: f32) -> Self {
        Self(us / 1_000_000.0)
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
        Grams(*self * scalar)
    }
}

// Division for units
impl Div<f32> for Grams {
    type Output = Grams;

    fn div(self, scalar: f32) -> Grams {
        Grams(*self / scalar)
    }
}

impl Div<f32> for Newtons {
    type Output = Newtons;

    fn div(self, scalar: f32) -> Newtons {
        Newtons(*self / scalar)
    }
}

// Addition for units
impl Add for Grams {
    type Output = Grams;

    fn add(self, other: Grams) -> Grams {
        Grams(*self + *other)
    }
}

impl AddAssign for Grams {
    fn add_assign(&mut self, other: Grams) {
        self.0 += *other;
    }
}

// Meters arithmetic
impl Add for Meters {
    type Output = Meters;

    fn add(self, other: Meters) -> Meters {
        Meters(*self + *other)
    }
}

impl Mul<f32> for Meters {
    type Output = Meters;

    fn mul(self, scalar: f32) -> Meters {
        Meters(*self * scalar)
    }
}

// NewtonsPerMeter * Meters = Newtons (F = k * x)
impl Mul<Meters> for NewtonsPerMeter {
    type Output = Newtons;

    fn mul(self, extension: Meters) -> Newtons {
        Newtons(*self * *extension)
    }
}

// GramsPerMeter * Meters = Grams (mass = density * length)
impl Mul<Meters> for GramsPerMeter {
    type Output = Grams;

    fn mul(self, length: Meters) -> Grams {
        // g/m * m = g
        Grams(self.0 * *length)
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
        // Standard Earth gravity: 9.81 m/s²
        assert_eq!(*EARTH_GRAVITY, 9.81);
    }

    #[test]
    fn test_linear_density() {
        // Test GramsPerMeter * Meters = Grams
        let density = GramsPerMeter(10.0); // 10 g/m
        let length = Meters(0.1); // 100 mm
        let mass = density * length;
        assert_eq!(mass.0, 1.0); // 1 gram (10 g/m * 0.1 m)
    }

    #[test]
    fn test_time_conversion() {
        let dt = Seconds::from_microseconds(250.0);
        assert!((dt.0 - 0.00025).abs() < 1e-9);
        assert!((dt.to_microseconds() - 250.0).abs() < 1e-3);
    }

    #[test]
    fn test_spring_force() {
        // Test F = kx with NewtonsPerMeter
        let spring_constant = NewtonsPerMeter(100.0); // 100 N/m
        let extension = Meters(0.1); // 10 cm
        let force = spring_constant * extension;
        assert_eq!(*force, 10.0); // 10 Newtons
    }

    #[test]
    fn test_force_division() {
        // Test F / m = a (returns m/s²)
        let force = Newtons(9.81);
        let mass = Grams(1000.0); // 1 kg
        let acceleration = force / mass;
        assert!((acceleration - 9.81).abs() < 1e-6); // 9.81 m/s²
    }
}
