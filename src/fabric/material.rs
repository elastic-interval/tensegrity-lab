use crate::fabric::interval::Role;
use crate::fabric::interval::Role::{Pull, Push, Spring};
use crate::fabric::material::Material::{
    BowTieMaterial, FaceRadialMaterial, GuyLineMaterial, NorthMaterial, PullMaterial, PushMaterial,
    SouthMaterial, SpringMaterial,
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
    PushMaterial = 0,
    PullMaterial = 1,
    BowTieMaterial = 2,
    NorthMaterial = 3,
    SouthMaterial = 4,
    SpringMaterial = 5,
    FaceRadialMaterial = 6,
    GuyLineMaterial = 7,
}

impl Material {
    pub fn properties(&self) -> MaterialProperties {
        match self {
            PushMaterial => MaterialProperties {
                label: ":push",
                role: Push,
                stiffness: 30.0,
                mass: 1.0,
                support: false,
            },
            PullMaterial => MaterialProperties {
                label: ":pull",
                role: Pull,
                stiffness: 1.0,
                mass: 0.1,
                support: false,
            },
            BowTieMaterial => MaterialProperties {
                label: ":bow-tie",
                role: Pull,
                stiffness: 1.0,
                mass: 0.1,
                support: false,
            },
            NorthMaterial => MaterialProperties {
                label: ":north",
                role: Pull,
                stiffness: 1.0,
                mass: 0.01,
                support: true,
            },
            SouthMaterial => MaterialProperties {
                label: ":south",
                role: Pull,
                stiffness: 1.0,
                mass: 0.01,
                support: true,
            },
            SpringMaterial => MaterialProperties {
                label: ":spring",
                role: Spring,
                stiffness: 0.5,
                mass: 0.01,
                support: false,
            },
            FaceRadialMaterial => MaterialProperties {
                label: ":pull",
                role: Pull,
                stiffness: 1.0,
                mass: 0.1,
                support: false,
            },
            GuyLineMaterial => MaterialProperties {
                label: ":pull",
                role: Pull,
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
            PushMaterial,
            PullMaterial,
            BowTieMaterial,
            NorthMaterial,
            SouthMaterial,
            SpringMaterial,
            FaceRadialMaterial,
            GuyLineMaterial,
        ];

        ALL_MATERIALS
            .iter()
            .find(|&&material| material.properties().label == label)
            .copied()
    }
}
