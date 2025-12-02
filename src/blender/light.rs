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
}

impl SphereLight {
    pub fn new(name: &str, intensity_watts: f32) -> Self {
        Self {
            name: name.to_string(),
            intensity_watts,
            radius: 0.1,
            color: Color3::new(1.0, 0.98, 0.95),
            translate: Vec3::zero(),
        }
    }

    pub fn with_translate(mut self, x: f32, y: f32, z: f32) -> Self {
        self.translate = Vec3::new(x, y, z);
        self
    }

    fn usd_intensity(&self) -> f32 {
        self.intensity_watts / BLENDER_INTENSITY_FACTOR
    }

    pub fn headlight_left(intensity_watts: f32) -> Self {
        Self::new("HeadlightLeft", intensity_watts).with_translate(-0.3, -0.1, -0.3)
    }

    pub fn headlight_right(intensity_watts: f32) -> Self {
        Self::new("HeadlightRight", intensity_watts).with_translate(0.3, -0.1, -0.3)
    }
}

impl Display for SphereLight {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "    def SphereLight \"{}\"", self.name)?;
        writeln!(f, "    {{")?;
        writeln!(f, "        float inputs:intensity = {:.1}", self.usd_intensity())?;
        writeln!(f, "        float inputs:radius = {:.1}", self.radius)?;
        writeln!(f, "        color3f inputs:color = {}", self.color)?;
        writeln!(f, "        double3 xformOp:translate = {}", self.translate)?;
        writeln!(f, "        uniform token[] xformOpOrder = [\"xformOp:translate\"]")?;
        writeln!(f, "    }}")?;
        Ok(())
    }
}
