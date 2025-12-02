/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use std::fmt::{Display, Formatter, Result};

use super::usd::Color3;

pub struct Material {
    pub name: String,
    pub diffuse_color: Color3,
    pub roughness: f32,
    pub metallic: f32,
}

impl Material {
    pub fn new(name: &str, diffuse_color: Color3) -> Self {
        Self {
            name: name.to_string(),
            diffuse_color,
            roughness: 0.5,
            metallic: 0.0,
        }
    }

    pub fn with_roughness(mut self, roughness: f32) -> Self {
        self.roughness = roughness;
        self
    }

    pub fn with_metallic(mut self, metallic: f32) -> Self {
        self.metallic = metallic;
        self
    }

    pub fn grass() -> Self {
        Self::new("GroundMaterial", Color3::new(0.08, 0.2, 0.04)).with_roughness(0.9)
    }

    pub fn aluminum() -> Self {
        Self::new("AluminumMaterial", Color3::new(0.91, 0.92, 0.92))
            .with_roughness(0.3)
            .with_metallic(1.0)
    }

    pub fn rope() -> Self {
        Self::new("RopeMaterial", Color3::new(0.95, 0.95, 0.92)).with_roughness(0.9)
    }
}

impl Display for Material {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "    def Material \"{}\"", self.name)?;
        writeln!(f, "    {{")?;
        writeln!(
            f,
            "        token outputs:surface.connect = </Materials/{}/Shader.outputs:surface>",
            self.name
        )?;
        writeln!(f, "        def Shader \"Shader\"")?;
        writeln!(f, "        {{")?;
        writeln!(f, "            uniform token info:id = \"UsdPreviewSurface\"")?;
        writeln!(f, "            color3f inputs:diffuseColor = {}", self.diffuse_color)?;
        writeln!(f, "            float inputs:roughness = {:.1}", self.roughness)?;
        writeln!(f, "            float inputs:metallic = {:.1}", self.metallic)?;
        writeln!(f, "            token outputs:surface")?;
        writeln!(f, "        }}")?;
        writeln!(f, "    }}")?;
        Ok(())
    }
}

pub struct MaterialScope {
    pub materials: Vec<Material>,
}

impl MaterialScope {
    pub fn new() -> Self {
        Self { materials: Vec::new() }
    }

    pub fn add(mut self, material: Material) -> Self {
        self.materials.push(material);
        self
    }

    pub fn fabric_defaults() -> Self {
        Self::new()
            .add(Material::grass())
            .add(Material::aluminum())
            .add(Material::rope())
    }
}

impl Default for MaterialScope {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for MaterialScope {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "def Scope \"Materials\"")?;
        writeln!(f, "{{")?;
        for material in &self.materials {
            write!(f, "{material}")?;
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}
