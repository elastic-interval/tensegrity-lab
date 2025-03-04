use crate::fabric::interval::Role;
use crate::fabric::interval::Role::{Pull, Push, Spring};
use crate::fabric::material::Material::{BowTieMaterial, FaceRadialMaterial, GuyWireMaterial, NorthMaterial, PullMaterial, PushMaterial, SouthMaterial, SpringMaterial};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Material {
    PushMaterial = 0,
    PullMaterial = 1,
    BowTieMaterial = 2,
    NorthMaterial = 3,
    SouthMaterial = 4,
    SpringMaterial = 5,
    FaceRadialMaterial = 6,
    GuyWireMaterial = 7,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntervalMaterial {
    pub name: Material,
    pub label: &'static str,
    pub role: Role,
    pub stiffness: f32,
    pub mass: f32,
}

const PUSH_MATERIAL: IntervalMaterial = IntervalMaterial {
    name: PushMaterial,
    label: ":push",
    role: Push,
    stiffness: 100.0,
    mass: 1.0,
};

const PULL_MATERIAL: IntervalMaterial = IntervalMaterial {
    name: PullMaterial,
    label: ":pull",
    role: Pull,
    stiffness: 1.0,
    mass: 0.1,
};

const BOW_TIE_MATERIAL: IntervalMaterial = IntervalMaterial {
    name: BowTieMaterial,
    label: ":bow-tie",
    role: Pull,
    stiffness: 1.0,
    mass: 0.1,
};

const NORTH_MATERIAL: IntervalMaterial = IntervalMaterial {
    name: NorthMaterial,
    label: ":north",
    role: Pull,
    stiffness: 0.5,
    mass: 0.01,
};

const SOUTH_MATERIAL: IntervalMaterial = IntervalMaterial {
    name: SouthMaterial,
    label: ":south",
    role: Pull,
    stiffness: 0.5,
    mass: 0.01,
};

const SPRING_MATERIAL: IntervalMaterial = IntervalMaterial {
    name: SpringMaterial,
    label: ":spring",
    role: Spring,
    stiffness: 0.5,
    mass: 0.01,
};

const FACE_RADIAL_MATERIAL: IntervalMaterial = IntervalMaterial {
    name: FaceRadialMaterial,
    label: ":pull",
    role: Pull,
    stiffness: 1.0,
    mass: 0.1,
};

const GUY_WIRE_MATERIAL: IntervalMaterial = IntervalMaterial {
    name: GuyWireMaterial,
    label: ":pull",
    role: Pull,
    stiffness: 1.0,
    mass: 0.1,
};

const MATERIALS: [IntervalMaterial; 8] = [
    PUSH_MATERIAL,
    PULL_MATERIAL,
    BOW_TIE_MATERIAL,
    NORTH_MATERIAL,
    SOUTH_MATERIAL,
    SPRING_MATERIAL,
    FACE_RADIAL_MATERIAL,
    GUY_WIRE_MATERIAL,
];

pub fn interval_material(material: Material) -> &'static IntervalMaterial {
    &MATERIALS[material as usize]
}

pub fn material_by_label(sought_label: String) -> Material {
    MATERIALS
        .iter()
        .find_map(|&IntervalMaterial { name, label, .. }|
            if sought_label.as_str() == label { Some(name) } else { None })
        .unwrap()
}
