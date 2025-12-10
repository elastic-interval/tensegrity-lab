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

/// Length in meters
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Meters(pub f32);

/// Force in Newtons
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Newtons(pub f32);

/// Acceleration in meters per second squared
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct MetersPerSecondSquared(pub f32);

/// Acceleration in millimeters per second squared
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct MillimetersPerSecondSquared(pub f32);

impl Deref for MillimetersPerSecondSquared {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Acceleration in millimeters per microsecond squared (simulation units)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct MillimetersPerMicrosecondSquared(pub f32);

/// Force in nanoNewtons (nN)
/// In the simulation's unit system: 1 nN = 1 g⋅mm/µs²
/// Conversion: 1 Newton = 10^9 nanoNewtons
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct NanoNewtons(pub f32);

/// Time in seconds
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Seconds(pub f32);

/// Percentage value (0-100)
/// Provides type-safe conversion to factors (0.0-1.0)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Percent(pub f32);


// Common time constants
pub const IMMEDIATE: Seconds = Seconds(0.0);
pub const MOMENT: Seconds = Seconds(0.1);

/// Linear density (mass per unit length) in grams per millimeter
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct GramsPerMillimeter(pub f32);

/// Rigidity in Newtons per millimeter (force per unit extension)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct NewtonsPerMillimeter(pub f32);

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

impl Deref for Millimeters {
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

impl Deref for NewtonsPerMeter {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for MillimetersPerMicrosecondSquared {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for NanoNewtons {
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

// Physical constants

/// Standard Earth gravity: 9.81 m/s²
pub const EARTH_GRAVITY_M_S2: MetersPerSecondSquared = MetersPerSecondSquared(9.81);

/// Standard Earth gravity: 9810 mm/s²
pub const EARTH_GRAVITY_MM_S2: MillimetersPerSecondSquared = MillimetersPerSecondSquared(9810.0);

/// Standard Earth gravity in simulation units: 9.81e-9 mm/µs²
/// This is the value to use in physics calculations since the simulation
/// uses microseconds for time and millimeters for length.
/// Derivation: 9.81 m/s² = 9810 mm/s² = 9810 mm / (10^6 µs)² = 9.81e-9 mm/µs²
pub const EARTH_GRAVITY: MillimetersPerMicrosecondSquared = MillimetersPerMicrosecondSquared(9.81e-9);

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
    pub fn to_meters(self) -> Meters {
        Meters(self.0 / 1000.0)
    }

    /// Create from meters
    pub fn from_meters(m: f32) -> Self {
        Self(m * 1000.0)
    }
}

impl Meters {
    /// Convert meters to millimeters
    pub fn to_millimeters(self) -> Millimeters {
        Millimeters(self.0 * 1000.0)
    }

    /// Create from millimeters
    pub fn from_millimeters(mm: f32) -> Self {
        Self(mm / 1000.0)
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
    
    /// Convert to millimeters per microsecond squared (simulation units)
    pub fn to_mm_us2(self) -> MillimetersPerMicrosecondSquared {
        // 1 s = 10^6 µs, so 1 s² = 10^12 µs²
        // mm/s² → mm/µs² means dividing by 10^12
        MillimetersPerMicrosecondSquared(self.0 / 1e12)
    }
}

impl MillimetersPerMicrosecondSquared {
    /// Convert to millimeters per second squared
    pub fn to_mm_s2(self) -> MillimetersPerSecondSquared {
        // mm/µs² → mm/s² means multiplying by 10^12
        MillimetersPerSecondSquared(self.0 * 1e12)
    }
}

impl Newtons {
    /// Convert to nanoNewtons (simulation units)
    pub fn to_nano_newtons(self) -> NanoNewtons {
        // 1 N = 10^9 nN
        NanoNewtons(self.0 * 1e9)
    }
}

impl NanoNewtons {
    /// Convert to Newtons
    pub fn to_newtons(self) -> Newtons {
        // 1 nN = 10^-9 N
        Newtons(self.0 / 1e9)
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

impl NewtonsPerMeter {
    /// Convert to Newtons per millimeter
    /// N/m → N/mm: divide by 1000 (1 meter = 1000 mm)
    pub fn to_n_per_mm(self) -> NewtonsPerMillimeter {
        NewtonsPerMillimeter(self.0 / 1000.0)
    }
}

impl NewtonsPerMillimeter {
    /// Convert to Newtons per meter
    /// N/mm → N/m: multiply by 1000 (1 meter = 1000 mm)
    pub fn to_n_per_m(self) -> NewtonsPerMeter {
        NewtonsPerMeter(self.0 * 1000.0)
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

// F = ma: Multiply mass by acceleration to get force
// In simulation units: grams × (mm/µs²) → nanoNewtons
impl Mul<MillimetersPerMicrosecondSquared> for Grams {
    type Output = NanoNewtons;
    
    fn mul(self, acceleration: MillimetersPerMicrosecondSquared) -> NanoNewtons {
        // F = ma
        NanoNewtons(self.0 * acceleration.0)
    }
}

// Allow multiplication in either order
impl Mul<Grams> for MillimetersPerMicrosecondSquared {
    type Output = NanoNewtons;
    
    fn mul(self, mass: Grams) -> NanoNewtons {
        // F = ma
        NanoNewtons(mass.0 * self.0)
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

impl Mul<f32> for NanoNewtons {
    type Output = NanoNewtons;
    
    fn mul(self, scalar: f32) -> NanoNewtons {
        NanoNewtons(*self * scalar)
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

impl std::fmt::Display for NanoNewtons {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.3e}nN", self.0)
    }
}

impl std::fmt::Display for Percent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}%", self.0)
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
        assert_eq!(*length.to_meters(), 1.0);
        
        let length2 = Millimeters::from_meters(0.5);
        assert_eq!(length2.0, 500.0);
    }

    #[test]
    fn test_gravity_conversion() {
        let g = EARTH_GRAVITY_M_S2.to_mm_s2();
        assert_eq!(g.0, 9810.0);
        
        // Test simulation units conversion (use relative error for very small numbers)
        let g_sim = EARTH_GRAVITY_MM_S2.to_mm_us2();
        let relative_error = (g_sim.0 - 9.81e-9).abs() / 9.81e-9;
        assert!(relative_error < 1e-10, "relative error: {}", relative_error);
        
        // Test round-trip
        let g_back = EARTH_GRAVITY.to_mm_s2();
        assert!((g_back.0 - 9810.0).abs() < 1e-6);
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

    #[test]
    fn test_gravity_force() {
        // Test F = ma with Earth's gravity
        let mass = Grams(100.0); // 100 grams
        let gravity_force = mass * EARTH_GRAVITY;
        
        // Expected: 100 g × 9.81e-9 mm/µs² = 9.81e-7 g⋅mm/µs²
        assert!((*gravity_force - 9.81e-7).abs() < 1e-15);
        
        // Test multiplication in reverse order
        let gravity_force2 = EARTH_GRAVITY * mass;
        assert_eq!(gravity_force, gravity_force2);
        
        // Test scalar multiplication
        let doubled = gravity_force * 2.0;
        assert!((*doubled - 1.962e-6).abs() < 1e-15);
    }

    #[test]
    fn test_newton_conversions() {
        // Test Newton to NanoNewton conversion
        let force = Newtons(1.0);
        let nano_force = force.to_nano_newtons();
        assert_eq!(*nano_force, 1e9);
        
        // Test NanoNewton to Newton conversion
        let nano = NanoNewtons(1e9);
        let newtons = nano.to_newtons();
        assert_eq!(*newtons, 1.0);
        
        // Test round-trip
        let original = Newtons(5.5);
        let round_trip = original.to_nano_newtons().to_newtons();
        assert!((round_trip.0 - original.0).abs() < 1e-6);
    }
}
