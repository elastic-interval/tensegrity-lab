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
        GramsPerMillimeter(self.base_linear_density().0 * physics.mass_scale())
    }

    /// Spring constant at 1m (1000mm) reference length
    /// k × L = constant, so k(1000mm) = k(1mm) / 1000
    /// 
    /// Push: 7,500,000,000 N/m - comparable to a 50mm diameter aluminum tube with 2mm wall thickness
    /// (Young's modulus ~70 GPa, moment of inertia for thin-walled tube I ≈ π*r³*t ≈ 18,850 mm⁴,
    /// axial stiffness EA ≈ 22 MN for 1m length)
    /// 
    /// Pull: 600,000,000 N/m - comparable to 10mm diameter  Dyneema rope
    /// (Dyneema has modulus ~100-120 GPa, cross-section ~78.5 mm², axial stiffness EA ≈ 8-9 MN)
    fn spring_constant_at_1m(&self) -> f32 {
        match self {
            Push => 7_500_000_000.0,
            Pull => 600_000_000.0,
            Spring => 90_000_000.0,
        }
    }

    /// Spring constant for a given length in millimeters
    /// Spring constant scales as k ∝ 1/L (shorter intervals are stiffer)
    /// Also scaled by physics rigidity_scale parameter
    pub fn spring_constant(&self, length_mm: f32, physics: &Physics) -> f32 {
        let k_at_1m = self.spring_constant_at_1m();
        // k(L) = k(1m) × (1000mm / L) × rigidity_scale
        (k_at_1m * 1000.0 / length_mm.max(1.0)) * physics.rigidity_scale()
    }

    pub fn default_role(&self) -> Role {
        match self {
            Push => Role::Pushing,
            Pull => Role::Pulling,
            Spring => Role::Springy,
        }
    }
}
