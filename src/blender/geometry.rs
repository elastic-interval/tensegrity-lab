/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use std::fmt::{Display, Formatter, Result};

use cgmath::{InnerSpace, Point3, Vector3};

use super::usd::{TimeSamples, UsdMatrix};

pub struct AnimatedSphere {
    pub name: String,
    pub radius: f32,
    pub transforms: TimeSamples<UsdMatrix>,
}

impl AnimatedSphere {
    pub fn new(name: &str, radius: f32) -> Self {
        Self {
            name: name.to_string(),
            radius,
            transforms: TimeSamples::new(),
        }
    }

    pub fn add_position(&mut self, frame: usize, position: Point3<f32>) {
        let matrix = UsdMatrix::from_scale_and_translation(
            self.radius,
            [position.x, position.y, position.z],
        );
        self.transforms.add(frame, matrix);
    }
}

impl Display for AnimatedSphere {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "        def Sphere \"{}\"", self.name)?;
        writeln!(f, "        {{")?;
        writeln!(
            f,
            "            matrix4d xformOp:transform.timeSamples = {}",
            self.transforms
        )?;
        writeln!(f, "            uniform token[] xformOpOrder = [\"xformOp:transform\"]")?;
        writeln!(f, "        }}")?;
        Ok(())
    }
}

pub struct AnimatedCylinder {
    pub name: String,
    pub radius: f32,
    pub material_binding: Option<String>,
    pub transforms: TimeSamples<UsdMatrix>,
}

impl AnimatedCylinder {
    pub fn new(name: &str, radius: f32) -> Self {
        Self {
            name: name.to_string(),
            radius,
            material_binding: None,
            transforms: TimeSamples::new(),
        }
    }

    pub fn with_material(mut self, material_path: &str) -> Self {
        self.material_binding = Some(material_path.to_string());
        self
    }

    pub fn add_endpoints(&mut self, frame: usize, alpha: Point3<f32>, omega: Point3<f32>) {
        let mid = Point3::new(
            (alpha.x + omega.x) / 2.0,
            (alpha.y + omega.y) / 2.0,
            (alpha.z + omega.z) / 2.0,
        );

        let delta = omega - alpha;
        let length = delta.magnitude();

        if length < 1e-6 {
            return;
        }

        let y_axis = delta / length;
        let arbitrary = if y_axis.y.abs() < 0.9 {
            Vector3::new(0.0, 1.0, 0.0)
        } else {
            Vector3::new(1.0, 0.0, 0.0)
        };

        let x_axis = y_axis.cross(arbitrary).normalize();
        let z_axis = x_axis.cross(y_axis).normalize();

        let c0 = x_axis * self.radius;
        let c1 = y_axis * (length / 2.0);
        let c2 = z_axis * self.radius;

        let matrix = UsdMatrix::from_basis_and_translation(
            [c0.x, c0.y, c0.z],
            [c1.x, c1.y, c1.z],
            [c2.x, c2.y, c2.z],
            [mid.x, mid.y, mid.z],
        );

        self.transforms.add(frame, matrix);
    }

    pub fn add_endpoints_with_inset(
        &mut self,
        frame: usize,
        alpha: Point3<f32>,
        omega: Point3<f32>,
        inset: f32,
    ) {
        let delta = omega - alpha;
        let full_length = delta.magnitude();

        if full_length < 1e-6 || full_length < 2.0 * inset {
            return;
        }

        let dir = delta / full_length;
        let length = full_length - 2.0 * inset;

        let mid = Point3::new(
            (alpha.x + omega.x) / 2.0,
            (alpha.y + omega.y) / 2.0,
            (alpha.z + omega.z) / 2.0,
        );

        let y_axis = dir;
        let arbitrary = if y_axis.y.abs() < 0.9 {
            Vector3::new(0.0, 1.0, 0.0)
        } else {
            Vector3::new(1.0, 0.0, 0.0)
        };

        let x_axis = y_axis.cross(arbitrary).normalize();
        let z_axis = x_axis.cross(y_axis).normalize();

        let c0 = x_axis * self.radius;
        let c1 = y_axis * (length / 2.0);
        let c2 = z_axis * self.radius;

        let matrix = UsdMatrix::from_basis_and_translation(
            [c0.x, c0.y, c0.z],
            [c1.x, c1.y, c1.z],
            [c2.x, c2.y, c2.z],
            [mid.x, mid.y, mid.z],
        );

        self.transforms.add(frame, matrix);
    }
}

impl Display for AnimatedCylinder {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "            def Cylinder \"{}\"", self.name)?;
        writeln!(f, "            {{")?;
        writeln!(
            f,
            "                matrix4d xformOp:transform.timeSamples = {}",
            self.transforms.to_usd_string("            ")
        )?;
        writeln!(f, "                uniform token[] xformOpOrder = [\"xformOp:transform\"]")?;
        if let Some(ref binding) = self.material_binding {
            writeln!(f, "                rel material:binding = <{binding}>")?;
        }
        writeln!(f, "            }}")?;
        Ok(())
    }
}

pub struct GroundPlane {
    pub size: f32,
    pub material_binding: Option<String>,
}

impl GroundPlane {
    pub fn new(size: f32) -> Self {
        Self {
            size,
            material_binding: None,
        }
    }

    pub fn with_material(mut self, material_path: &str) -> Self {
        self.material_binding = Some(material_path.to_string());
        self
    }
}

impl Display for GroundPlane {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let half = self.size / 2.0;
        writeln!(f, "    def Mesh \"Ground\"")?;
        writeln!(f, "    {{")?;
        writeln!(f, "        int[] faceVertexCounts = [4]")?;
        writeln!(f, "        int[] faceVertexIndices = [0, 1, 2, 3]")?;
        writeln!(
            f,
            "        point3f[] points = [({}, 0, {}), ({}, 0, {}), ({}, 0, {}), ({}, 0, {})]",
            -half, -half, half, -half, half, half, -half, half
        )?;
        if let Some(ref binding) = self.material_binding {
            writeln!(f, "        rel material:binding = <{binding}>")?;
        }
        writeln!(f, "    }}")?;
        Ok(())
    }
}

pub struct Environment {
    pub ground: GroundPlane,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            ground: GroundPlane::new(100.0).with_material("/Materials/GroundMaterial"),
        }
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for Environment {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "def Xform \"Environment\"")?;
        writeln!(f, "{{")?;
        write!(f, "{}", self.ground)?;
        writeln!(f, "}}")?;
        Ok(())
    }
}
