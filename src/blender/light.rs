/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use std::fmt::{Display, Formatter, Result};

use super::usd::{Color3, Vec3};

const BLENDER_INTENSITY_FACTOR: f32 = 9.87;

pub struct SphereLight {
    pub name: String,
    pub intensity_watts: f32,
    pub radius: f32,
    pub color: Color3,
    pub translate: Vec3,
    pub treat_as_point: bool,
}

impl SphereLight {
    pub fn new(name: &str, intensity_watts: f32) -> Self {
        Self {
            name: name.to_string(),
            intensity_watts,
            radius: 0.1,
            color: Color3::new(1.0, 0.98, 0.95),
            translate: Vec3::zero(),
            treat_as_point: false,
        }
    }

    pub fn with_translate(mut self, x: f32, y: f32, z: f32) -> Self {
        self.translate = Vec3::new(x, y, z);
        self
    }

    pub fn treat_as_point_light(mut self) -> Self {
        self.treat_as_point = true;
        self
    }

    fn usd_intensity(&self) -> f32 {
        self.intensity_watts / BLENDER_INTENSITY_FACTOR
    }

    pub fn headlight_left(intensity_watts: f32) -> Self {
        Self::new("HeadlightLeft", intensity_watts)
            .with_translate(-0.3, -0.1, -0.3)
            .treat_as_point_light()
    }

    pub fn headlight_right(intensity_watts: f32) -> Self {
        Self::new("HeadlightRight", intensity_watts)
            .with_translate(0.3, -0.1, -0.3)
            .treat_as_point_light()
    }
}

impl Display for SphereLight {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "    def SphereLight \"{}\"", self.name)?;
        writeln!(f, "    {{")?;
        writeln!(f, "        float inputs:intensity = {:.1}", self.usd_intensity())?;
        writeln!(f, "        float inputs:radius = {:.1}", self.radius)?;
        writeln!(f, "        color3f inputs:color = {}", self.color)?;
        if self.treat_as_point {
            writeln!(f, "        bool treatAsPoint = true")?;
        }
        writeln!(f, "        double3 xformOp:translate = {}", self.translate)?;
        writeln!(f, "        uniform token[] xformOpOrder = [\"xformOp:translate\"]")?;
        writeln!(f, "    }}")?;
        Ok(())
    }
}

pub struct DomeLight {
    pub name: String,
    pub intensity: f32,
    pub color: Color3,
}

impl DomeLight {
    pub fn new(name: &str, intensity: f32, color: Color3) -> Self {
        Self {
            name: name.to_string(),
            intensity,
            color,
        }
    }

    pub fn sky_ambient() -> Self {
        Self::new("SkyLight", 1.0, Color3::new(0.05, 0.07, 0.12))
    }
}

impl Display for DomeLight {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "def DomeLight \"{}\"", self.name)?;
        writeln!(f, "{{")?;
        writeln!(f, "    float inputs:intensity = {:.2}", self.intensity)?;
        writeln!(f, "    color3f inputs:color = {}", self.color)?;
        writeln!(f, "}}")?;
        Ok(())
    }
}

pub struct DistantLight {
    pub name: String,
    pub intensity: f32,
    pub color: Color3,
    pub angle: f32,
    pub rotation: Vec3,
}

impl DistantLight {
    pub fn new(name: &str, intensity: f32) -> Self {
        Self {
            name: name.to_string(),
            intensity,
            color: Color3::new(1.0, 0.98, 0.9),
            angle: 0.53,
            rotation: Vec3::new(-45.0, 30.0, 0.0),
        }
    }

    pub fn sun() -> Self {
        Self::new("Sun", 1.5)
    }
}

impl Display for DistantLight {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "def DistantLight \"{}\"", self.name)?;
        writeln!(f, "{{")?;
        writeln!(f, "    float inputs:intensity = {:.2}", self.intensity)?;
        writeln!(f, "    color3f inputs:color = {}", self.color)?;
        writeln!(f, "    float inputs:angle = {:.2}", self.angle)?;
        writeln!(f, "    double3 xformOp:rotateXYZ = {}", self.rotation)?;
        writeln!(f, "    uniform token[] xformOpOrder = [\"xformOp:rotateXYZ\"]")?;
        writeln!(f, "}}")?;
        Ok(())
    }
}
