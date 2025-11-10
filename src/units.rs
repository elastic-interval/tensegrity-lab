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

/// Length in millimeters
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Millimeters(pub f32);

/// Force in Newtons
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Newtons(pub f32);

/// Acceleration in meters per second squared
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct MetersPerSecondSquared(pub f32);

/// Acceleration in millimeters per second squared
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct MillimetersPerSecondSquared(pub f32);

/// Time in seconds
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Seconds(pub f32);

// Common time constants
pub const IMMEDIATE: Seconds = Seconds(0.0);
pub const MOMENT: Seconds = Seconds(0.2);

/// Linear density (mass per unit length) in grams per millimeter
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct GramsPerMillimeter(pub f32);

/// Stiffness in Newtons per millimeter (force per unit extension)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct NewtonsPerMillimeter(pub f32);

// Deref implementations for ergonomic access to inner values

impl Deref for Grams {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for Millimeters {
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

impl Deref for GramsPerMillimeter {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for NewtonsPerMillimeter {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Physical constants

/// Standard Earth gravity: 9.81 m/s²
pub const EARTH_GRAVITY_M_S2: MetersPerSecondSquared = MetersPerSecondSquared(9.81);

/// Standard Earth gravity: 9810 mm/s²
pub const EARTH_GRAVITY_MM_S2: MillimetersPerSecondSquared = MillimetersPerSecondSquared(9810.0);

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

impl Millimeters {
    /// Convert millimeters to meters
    pub fn to_meters(self) -> f32 {
        self.0 / 1000.0
    }

    /// Create from meters
    pub fn from_meters(m: f32) -> Self {
        Self(m * 1000.0)
    }
}

impl MetersPerSecondSquared {
    /// Convert to millimeters per second squared
    pub fn to_mm_s2(self) -> MillimetersPerSecondSquared {
        MillimetersPerSecondSquared(self.0 * 1000.0)
    }
}

impl MillimetersPerSecondSquared {
    /// Convert to meters per second squared
    pub fn to_m_s2(self) -> MetersPerSecondSquared {
        MetersPerSecondSquared(self.0 / 1000.0)
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

// Arithmetic operations for dimensional analysis

impl Mul<Millimeters> for GramsPerMillimeter {
    type Output = Grams;
    
    fn mul(self, length: Millimeters) -> Grams {
        Grams(*self * *length)
    }
}

impl Mul<Seconds> for MillimetersPerSecondSquared {
    type Output = f32; // mm/s (velocity)
    
    fn mul(self, time: Seconds) -> f32 {
        self.0 * time.0
    }
}

impl Div<Grams> for Newtons {
    type Output = f32; // m/s² (acceleration)
    
    fn div(self, mass: Grams) -> f32 {
        // F = ma, so a = F/m
        // Newton = kg⋅m/s², Grams needs conversion to kg
        self.0 / mass.to_kg()
    }
}

impl Mul<f32> for NewtonsPerMillimeter {
    type Output = Newtons;
    
    fn mul(self, extension: f32) -> Newtons {
        Newtons(*self * extension)
    }
}

impl Mul<Millimeters> for NewtonsPerMillimeter {
    type Output = Newtons;
    
    fn mul(self, extension: Millimeters) -> Newtons {
        Newtons(*self * *extension)
    }
}

// Scalar multiplication for units
impl Mul<f32> for Millimeters {
    type Output = Millimeters;
    
    fn mul(self, scalar: f32) -> Millimeters {
        Millimeters(*self * scalar)
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

// Display implementations

impl std::fmt::Display for Grams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}g", self.0)
    }
}

impl std::fmt::Display for Millimeters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}mm", self.0)
    }
}

impl std::fmt::Display for Newtons {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.3}N", self.0)
    }
}

impl std::fmt::Display for GramsPerMillimeter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.4}g/mm", self.0)
    }
}

impl std::fmt::Display for NewtonsPerMillimeter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}N/mm", self.0)
    }
}

impl std::fmt::Display for Seconds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}s", self.0)
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
    fn test_length_conversions() {
        let length = Millimeters(1000.0);
        assert_eq!(length.to_meters(), 1.0);
        
        let length2 = Millimeters::from_meters(0.5);
        assert_eq!(length2.0, 500.0);
    }

    #[test]
    fn test_gravity_conversion() {
        let g = EARTH_GRAVITY_M_S2.to_mm_s2();
        assert_eq!(g.0, 9810.0);
    }

    #[test]
    fn test_linear_density() {
        let density = GramsPerMillimeter(0.01); // 10 mg/mm
        let length = Millimeters(100.0); // 100 mm
        let mass = density * length;
        assert_eq!(mass.0, 1.0); // 1 gram
    }

    #[test]
    fn test_time_conversion() {
        let dt = Seconds::from_microseconds(250.0);
        assert!((dt.0 - 0.00025).abs() < 1e-9);
        assert!((dt.to_microseconds() - 250.0).abs() < 1e-3);
    }
}
