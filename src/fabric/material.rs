use crate::fabric::interval::Role;
use crate::fabric::material::Material::{Pull, Push, Spring};
use crate::units::GramsPerMillimeter;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Material {
    Push,
    Pull,
    Spring,
}

impl Material {
    /// Linear density: mass per unit length (g/mm)
    pub fn linear_density(&self) -> GramsPerMillimeter {
        match self {
            Push => GramsPerMillimeter(0.01),
            Pull => GramsPerMillimeter(0.001),
            Spring => GramsPerMillimeter(0.0001),
        }
    }

    /// Spring constant at 1mm reference length
    /// k × L = constant, so k(1mm) = k(1000mm) × 1000
    fn spring_constant_at_1mm(&self) -> f32 {
        match self {
            Push => 5_400_000_000.0,   // stiff compression members
            Pull => 180_000_000.0,     // flexible tension cables
            Spring => 90_000_000.0,    // very flexible springs
        }
    }

    /// Spring constant for a given length in millimeters
    /// Spring constant scales as k ∝ 1/L (shorter intervals are stiffer)
    pub fn spring_constant(&self, length_mm: f32) -> f32 {
        let k_at_1mm = self.spring_constant_at_1mm();
        // k(L) = k(1mm) × (1mm / L)
        k_at_1mm / length_mm.max(1.0)
    }

    pub fn default_role(&self) -> Role {
        match self {
            Push => Role::Pushing,
            Pull => Role::Pulling,
            Spring => Role::Springy,
        }
    }
}
