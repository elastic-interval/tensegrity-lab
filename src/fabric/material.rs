use crate::fabric::interval::Role;
use crate::fabric::material::Material::{
    BowTie, FaceRadial, GuyLine, North, Pull, Push,
    South, Spring,
};

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
    Push = 0,
    Pull = 1,
    BowTie = 2,
    North = 3,
    South = 4,
    Spring = 5,
    FaceRadial = 6,
    GuyLine = 7,
}

impl Material {
    pub fn properties(&self) -> MaterialProperties {
        match self {
            Push => MaterialProperties {
                label: ":push",
                role: Role::Pushing,
                stiffness: 30.0,
                mass: 1.0,
                support: false,
            },
            Pull => MaterialProperties {
                label: ":pull",
                role: Role::Pulling,
                stiffness: 1.0,
                mass: 0.1,
                support: false,
            },
            BowTie => MaterialProperties {
                label: ":bow-tie",
                role: Role::Pulling,
                stiffness: 1.0,
                mass: 0.1,
                support: false,
            },
            North => MaterialProperties {
                label: ":north",
                role: Role::Pulling,
                stiffness: 1.0,
                mass: 0.01,
                support: true,
            },
            South => MaterialProperties {
                label: ":south",
                role: Role::Pulling,
                stiffness: 1.0,
                mass: 0.01,
                support: true,
            },
            Spring => MaterialProperties {
                label: ":spring",
                role: Role::Springy,
                stiffness: 0.5,
                mass: 0.01,
                support: false,
            },
            FaceRadial => MaterialProperties {
                label: ":pull",
                role: Role::Pulling,
                stiffness: 1.0,
                mass: 0.1,
                support: false,
            },
            GuyLine => MaterialProperties {
                label: ":pull",
                role: Role::Pulling,
                stiffness: 1.0,
                mass: 0.1,
                support: true,
            },
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        use Material::*;

        // This array ensures we search all materials
        const ALL_MATERIALS: [Material; 8] = [
            Push,
            Pull,
            BowTie,
            North,
            South,
            Spring,
            FaceRadial,
            GuyLine,
        ];

        ALL_MATERIALS
            .iter()
            .find(|&&material| material.properties().label == label)
            .copied()
    }
}
