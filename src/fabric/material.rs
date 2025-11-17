use crate::fabric::interval::Role;
use crate::fabric::material::Material::{Pull, Push, Spring};
use crate::fabric::physics::Physics;
use crate::units::GramsPerMillimeter;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Material {
    Push,
    Pull,
    Spring,
}

impl Material {
    /// Base linear density: mass per unit length (g/mm)
    fn base_linear_density(&self) -> GramsPerMillimeter {
        GramsPerMillimeter(match self {
            Push => 1.0,
            Pull => 0.05,
            Spring => 1.0,
        })
    }
    
    /// Linear density scaled by physics parameters
    pub fn linear_density(&self, physics: &Physics) -> GramsPerMillimeter {
        GramsPerMillimeter(self.base_linear_density().0 * physics.mass_scale)
    }

    /// Spring constant at 1mm reference length
    /// k × L = constant, so k(1mm) = k(1000mm) × 1000
    fn spring_constant_at_1mm(&self) -> f32 {
        match self {
            Push => 7_500_000_000_000.0,
            Pull => 600_000_000_000.0,
            Spring => 90_000_000_000.0,
        }
    }

    /// Spring constant for a given length in millimeters
    /// Spring constant scales as k ∝ 1/L (shorter intervals are stiffer)
    /// Also scaled by physics rigidity_scale parameter
    pub fn spring_constant(&self, length_mm: f32, physics: &Physics) -> f32 {
        let k_at_1mm = self.spring_constant_at_1mm();
        // k(L) = k(1mm) × (1mm / L) × rigidity_scale
        (k_at_1mm / length_mm.max(1.0)) * physics.rigidity_scale
    }

    pub fn default_role(&self) -> Role {
        match self {
            Push => Role::Pushing,
            Pull => Role::Pulling,
            Spring => Role::Springy,
        }
    }
}
