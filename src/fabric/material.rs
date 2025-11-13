use crate::fabric::interval::Role;
use crate::fabric::material::Material::{Pull, Push, Spring};
use crate::units::{GramsPerMillimeter, Millimeters, NewtonsPerMillimeter};

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
            Push => GramsPerMillimeter(0.001),
            Pull => GramsPerMillimeter(0.0001),
            Spring => GramsPerMillimeter(0.00001),
        }
    }

    /// Rigidity at 1-meter reference length (N/mm)
    /// Actual rigidity scales as k ∝ 1/L (shorter intervals are more rigid)
    pub fn rigidity_per_meter(&self) -> NewtonsPerMillimeter {
        match self {
            Push => NewtonsPerMillimeter(30_000.0),
            Pull => NewtonsPerMillimeter(1_000.0),
            Spring => NewtonsPerMillimeter(500.0),
        }
    }

    /// Rigidity adjusted for interval length (k ∝ 1/L)
    pub fn rigidity_at_length(&self, length: Millimeters) -> NewtonsPerMillimeter {
        let k_ref = self.rigidity_per_meter();
        NewtonsPerMillimeter(*k_ref * 1000.0 / length.max(0.1))
    }

    /// Material rigidity coefficient
    /// Relative rigidity between materials (Push=0.9, Pull=0.03, Spring=0.015)
    pub fn rigidity(&self) -> f32 {
        match self {
            Push => 0.9,
            Pull => 0.03,
            Spring => 0.015,
        }
    }

    /// Legacy mass multiplier
    pub fn mass(&self) -> f32 {
        match self {
            Push => 1.0,
            Pull => 0.1,
            Spring => 0.01,
        }
    }

    pub fn default_role(&self) -> Role {
        match self {
            Push => Role::Pushing,
            Pull => Role::Pulling,
            Spring => Role::Springy,
        }
    }
}
