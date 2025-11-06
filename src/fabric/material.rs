use crate::fabric::interval::Role;
use crate::fabric::material::Material::{FaceRadial, GuyLine, North, Pull, Push, South, Spring};

#[derive(Clone, Debug, PartialEq)]
pub struct MaterialProperties {
    pub label: &'static str,
    pub role: Role,
    pub stiffness: f32,
    pub mass: f32,
    pub support: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Material {
    Push,
    Pull,
    North,
    South,
    Spring,
    FaceRadial,
    GuyLine,
}

impl Material {
    pub fn properties(&self) -> MaterialProperties {
        match self {
            Push => MaterialProperties {
                label: "push",
                role: Role::Pushing,
                stiffness: 30.0,
                mass: 1.0,
                support: false,
            },
            Pull => MaterialProperties {
                label: "pull",
                role: Role::Pulling,
                stiffness: 1.0,
                mass: 0.1,
                support: false,
            },
            North => MaterialProperties {
                label: "north",
                role: Role::Pulling,
                stiffness: 1.0,
                mass: 0.01,
                support: true,
            },
            South => MaterialProperties {
                label: "south",
                role: Role::Pulling,
                stiffness: 1.0,
                mass: 0.01,
                support: true,
            },
            Spring => MaterialProperties {
                label: "spring",
                role: Role::Springy,
                stiffness: 0.5,
                mass: 0.01,
                support: false,
            },
            FaceRadial => MaterialProperties {
                label: "pull",
                role: Role::Pulling,
                stiffness: 1.0,
                mass: 0.1,
                support: false,
            },
            GuyLine => MaterialProperties {
                label: "pull",
                role: Role::Pulling,
                stiffness: 1.0,
                mass: 0.1,
                support: true,
            },
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        use Material::*;
        [Push, Pull, North, South, Spring, FaceRadial, GuyLine]
            .iter()
            .find(|&&material| material.properties().label == label)
            .copied()
    }
}
