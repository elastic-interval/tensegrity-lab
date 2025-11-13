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

    /// Spring constant at 100mm reference length
    /// These values were calibrated for typical interval lengths around 100mm
    fn spring_constant_at_100mm(&self) -> f32 {
        match self {
            Push => 54_000_000.0,  // stiff compression members
            Pull => 1_800_000.0,   // flexible tension cables
            Spring => 900_000.0,   // very flexible springs
        }
    }

    /// Spring constant for a given length in millimeters
    /// Spring constant scales as k ∝ 1/L (shorter intervals are stiffer)
    /// Reference length is 100mm
    pub fn spring_constant(&self, length_mm: f32) -> f32 {
        let k_ref = self.spring_constant_at_100mm();
        // k = k_ref × (100mm / length_mm)
        k_ref * 100.0 / length_mm.max(1.0)
    }

    pub fn default_role(&self) -> Role {
        match self {
            Push => Role::Pushing,
            Pull => Role::Pulling,
            Spring => Role::Springy,
        }
    }
}
