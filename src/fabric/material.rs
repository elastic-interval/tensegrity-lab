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

    /// Spring constant for physics simulation
    /// These are effective constants calibrated for the simulation, not physical N/mm values
    pub fn spring_constant(&self) -> f32 {
        match self {
            Push => 54_000_000.0,  // stiff compression members
            Pull => 1_800_000.0,   // flexible tension cables
            Spring => 900_000.0,   // very flexible springs
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
