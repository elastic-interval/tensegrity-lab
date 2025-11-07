use crate::fabric::interval::Role;
use crate::fabric::material::Material::{Pull, Push, Spring};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Material {
    Push,
    Pull,
    Spring,
}

impl Material {
    pub fn stiffness(&self) -> f32 {
        match self {
            Push => 30.0,
            Pull => 1.0,
            Spring => 0.5,
        }
    }

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
