/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use std::fmt::{Display, Formatter, Result};

use cgmath::{InnerSpace, Point3, Vector3};

use super::light::SphereLight;
use super::usd::{TimeSamples, UsdMatrix};

pub struct CameraRig {
    pub name: String,
    pub focal_length: f32,
    pub horizontal_aperture: f32,
    pub vertical_aperture: f32,
    pub clipping_range: (f32, f32),
    pub headlights: Vec<SphereLight>,
    pub transforms: TimeSamples<UsdMatrix>,
}

impl CameraRig {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            focal_length: 50.0,
            horizontal_aperture: 36.0,
            vertical_aperture: 24.0,
            clipping_range: (0.1, 1000.0),
            headlights: Vec::new(),
            transforms: TimeSamples::new(),
        }
    }

    pub fn with_headlights(mut self, intensity_watts: f32) -> Self {
        self.headlights.push(SphereLight::headlight_left(intensity_watts));
        self.headlights.push(SphereLight::headlight_right(intensity_watts));
        self
    }

    pub fn add_look_at_frame(
        &mut self,
        frame: usize,
        position: Point3<f32>,
        target: Point3<f32>,
        export_scale: f32,
    ) {
        let pos = position * export_scale;
        let target = target * export_scale;

        let forward = (target - pos).normalize();
        let world_up = Vector3::new(0.0f32, 1.0, 0.0);
        let right = forward.cross(world_up).normalize();
        let up = right.cross(forward).normalize();
        let neg_forward = -forward;

        let matrix = UsdMatrix::from_basis_and_translation(
            [right.x, right.y, right.z],
            [up.x, up.y, up.z],
            [neg_forward.x, neg_forward.y, neg_forward.z],
            [pos.x, pos.y, pos.z],
        );

        self.transforms.add(frame, matrix);
    }

    pub fn has_animation(&self) -> bool {
        !self.transforms.is_empty()
    }
}

impl Display for CameraRig {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        if !self.has_animation() {
            return Ok(());
        }

        writeln!(f, "def Xform \"{}\"", self.name)?;
        writeln!(f, "{{")?;
        writeln!(
            f,
            "    matrix4d xformOp:transform.timeSamples = {}",
            self.transforms.to_usd_string("    ")
        )?;
        writeln!(f, "    uniform token[] xformOpOrder = [\"xformOp:transform\"]")?;
        writeln!(f)?;

        writeln!(f, "    def Camera \"Camera\"")?;
        writeln!(f, "    {{")?;
        writeln!(f, "        float focalLength = {:.1}", self.focal_length)?;
        writeln!(f, "        float horizontalAperture = {:.1}", self.horizontal_aperture)?;
        writeln!(f, "        float verticalAperture = {:.1}", self.vertical_aperture)?;
        writeln!(
            f,
            "        float2 clippingRange = ({:.1}, {:.1})",
            self.clipping_range.0, self.clipping_range.1
        )?;
        writeln!(f, "    }}")?;

        for light in &self.headlights {
            writeln!(f)?;
            write!(f, "{light}")?;
        }

        writeln!(f, "}}")?;
        Ok(())
    }
}
