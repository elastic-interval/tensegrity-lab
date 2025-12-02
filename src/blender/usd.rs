/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use std::fmt::{Display, Formatter, Result};

#[derive(Clone, Copy, Debug)]
pub struct UsdMatrix(pub [f32; 16]);

impl UsdMatrix {
    pub fn from_basis_and_translation(
        x_axis: [f32; 3],
        y_axis: [f32; 3],
        z_axis: [f32; 3],
        translation: [f32; 3],
    ) -> Self {
        Self([
            x_axis[0], x_axis[1], x_axis[2], 0.0,
            y_axis[0], y_axis[1], y_axis[2], 0.0,
            z_axis[0], z_axis[1], z_axis[2], 0.0,
            translation[0], translation[1], translation[2], 1.0,
        ])
    }

    pub fn from_scale_and_translation(scale: f32, translation: [f32; 3]) -> Self {
        Self([
            scale, 0.0, 0.0, 0.0,
            0.0, scale, 0.0, 0.0,
            0.0, 0.0, scale, 0.0,
            translation[0], translation[1], translation[2], 1.0,
        ])
    }
}

impl Display for UsdMatrix {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let m = &self.0;
        write!(
            f,
            "( ({:.6}, {:.6}, {:.6}, {:.6}), ({:.6}, {:.6}, {:.6}, {:.6}), ({:.6}, {:.6}, {:.6}, {:.6}), ({:.6}, {:.6}, {:.6}, {:.6}) )",
            m[0], m[1], m[2], m[3],
            m[4], m[5], m[6], m[7],
            m[8], m[9], m[10], m[11],
            m[12], m[13], m[14], m[15]
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Color3 {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Color3 {
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }
}

impl Display for Color3 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "({:.3}, {:.3}, {:.3})", self.r, self.g, self.b)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }
}

impl Display for Vec3 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "({:.6}, {:.6}, {:.6})", self.x, self.y, self.z)
    }
}

#[derive(Clone, Debug, Default)]
pub struct TimeSamples<T> {
    samples: Vec<(usize, T)>,
}

impl<T> TimeSamples<T> {
    pub fn new() -> Self {
        Self { samples: Vec::new() }
    }

    pub fn add(&mut self, frame: usize, value: T) {
        self.samples.push((frame, value));
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}

impl<T: Display> TimeSamples<T> {
    pub fn to_usd_string(&self, indent: &str) -> String {
        let mut output = String::new();
        output.push_str("{\n");

        for (i, (frame, value)) in self.samples.iter().enumerate() {
            output.push_str(&format!("{indent}    {frame}: {value}"));
            if i < self.samples.len() - 1 {
                output.push(',');
            }
            output.push('\n');
        }

        output.push_str(&format!("{indent}}}"));
        output
    }
}

impl<T: Display> Display for TimeSamples<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.to_usd_string("            "))
    }
}

pub struct UsdHeader {
    pub default_prim: String,
    pub meters_per_unit: f64,
    pub start_time_code: Option<usize>,
    pub end_time_code: Option<usize>,
    pub time_codes_per_second: Option<f64>,
    pub frames_per_second: Option<f64>,
}

impl UsdHeader {
    pub fn new(default_prim: &str) -> Self {
        Self {
            default_prim: default_prim.to_string(),
            meters_per_unit: 1.0,
            start_time_code: None,
            end_time_code: None,
            time_codes_per_second: None,
            frames_per_second: None,
        }
    }

    pub fn with_animation(mut self, start: usize, end: usize, fps: f64) -> Self {
        self.start_time_code = Some(start);
        self.end_time_code = Some(end);
        self.time_codes_per_second = Some(fps);
        self.frames_per_second = Some(fps);
        self
    }
}

impl Display for UsdHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "#usda 1.0")?;
        writeln!(f, "(")?;
        writeln!(f, "    defaultPrim = \"{}\"", self.default_prim)?;
        writeln!(f, "    metersPerUnit = {}", self.meters_per_unit)?;
        writeln!(f, "    upAxis = \"Y\"")?;
        if let Some(start) = self.start_time_code {
            writeln!(f, "    startTimeCode = {start}")?;
        }
        if let Some(end) = self.end_time_code {
            writeln!(f, "    endTimeCode = {end}")?;
        }
        if let Some(tps) = self.time_codes_per_second {
            writeln!(f, "    timeCodesPerSecond = {tps}")?;
        }
        if let Some(fps) = self.frames_per_second {
            writeln!(f, "    framesPerSecond = {fps}")?;
        }
        writeln!(f, ")")?;
        Ok(())
    }
}
