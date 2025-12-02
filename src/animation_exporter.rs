/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::fabric::interval::{Interval, Role};
use crate::fabric::Fabric;
use cgmath::{InnerSpace, Point3, Vector3};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;

/// Stores transform data for a single object across all frames
#[derive(Clone)]
struct TransformTimeSamples {
    /// Frame number -> 4x4 matrix as 16 floats (row-major)
    samples: HashMap<usize, [f32; 16]>,
}

impl TransformTimeSamples {
    fn new() -> Self {
        Self {
            samples: HashMap::new(),
        }
    }

    fn add_sample(&mut self, frame: usize, matrix: [f32; 16]) {
        self.samples.insert(frame, matrix);
    }

    /// Format as USD time samples string
    fn to_usd_string(&self) -> String {
        let mut frames: Vec<_> = self.samples.keys().collect();
        frames.sort();

        let mut output = String::new();
        output.push_str("{\n");

        for (i, &frame) in frames.iter().enumerate() {
            let m = &self.samples[frame];
            // Format matrix in USD row-vector format
            output.push_str(&format!(
                "                {}: ( ({:.6}, {:.6}, {:.6}, {:.6}), ({:.6}, {:.6}, {:.6}, {:.6}), ({:.6}, {:.6}, {:.6}, {:.6}), ({:.6}, {:.6}, {:.6}, {:.6}) )",
                frame,
                m[0], m[1], m[2], m[3],
                m[4], m[5], m[6], m[7],
                m[8], m[9], m[10], m[11],
                m[12], m[13], m[14], m[15]
            ));
            if i < frames.len() - 1 {
                output.push(',');
            }
            output.push('\n');
        }
        output.push_str("            }");
        output
    }
}

/// Frame data captured for time-sampled animation
struct FrameData {
    /// Joint positions at this frame
    joint_positions: Vec<Point3<f32>>,
    /// Interval endpoint indices (alpha, omega) - same across all frames
    interval_endpoints: Vec<(usize, usize, Role)>,
    /// Camera position for this frame
    camera_pos: Option<Point3<f32>>,
    /// Camera look-at target for this frame
    camera_target: Option<Point3<f32>>,
}

/// Manages the export of animation frames to USD format with time samples
pub struct AnimationExporter {
    output_dir: PathBuf,
    frame_count: usize,
    fps: f64,
    capture_interval: usize,
    iteration_count: usize,
    enabled: bool,
    /// All captured frames
    frames: Vec<FrameData>,
    /// Scale factor from fabric
    scale: f32,
    /// Fabric name
    fabric_name: String,
}

impl AnimationExporter {
    /// Create a new animation exporter
    pub fn new<P: Into<PathBuf>>(output_dir: P, fps: f64, capture_interval: usize) -> Self {
        Self {
            output_dir: output_dir.into(),
            frame_count: 0,
            fps,
            capture_interval,
            iteration_count: 0,
            enabled: false,
            frames: Vec::new(),
            scale: 1.0,
            fabric_name: String::new(),
        }
    }

    /// Start capturing frames
    pub fn start(&mut self) {
        self.enabled = true;
        self.frame_count = 0;
        self.iteration_count = 0;
        self.frames.clear();
        println!("Animation export started (will save time-sampled USD when stopped)");
    }

    /// Stop capturing frames and finalize the animation
    pub fn stop(&mut self) -> io::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        self.enabled = false;

        if self.frame_count == 0 {
            println!("No frames captured");
            return Ok(());
        }

        println!(
            "Creating time-sampled USD animation with {} frames...",
            self.frame_count
        );

        // Create USD file with timestamp
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let usd_path = self.output_dir.with_file_name(format!(
            "animation_{}.usda",
            timestamp
        ));

        let usd_content = self.create_time_sampled_usd()?;

        let mut file = File::create(&usd_path)?;
        file.write_all(usd_content.as_bytes())?;

        // Clear frame data from memory
        self.frames.clear();

        println!("Animation export completed!");
        println!("USD file: {:?}", usd_path);
        println!("Frames: {}", self.frame_count);
        println!("\nTo use in Blender:");
        println!("  1. File -> Import -> Universal Scene Description");
        println!("  2. Select the .usda file");
        println!("  3. Animation should play automatically in the timeline");

        Ok(())
    }

    /// Create a time-sampled USD file from all captured frames
    fn create_time_sampled_usd(&self) -> io::Result<String> {
        let mut output = String::new();

        // Convert scale from millimeters to meters
        let export_scale = self.scale / 1000.0;

        // USD header
        output.push_str("#usda 1.0\n");
        output.push_str("(\n");
        output.push_str("    defaultPrim = \"Animation\"\n");
        output.push_str("    metersPerUnit = 1.0\n");
        output.push_str("    upAxis = \"Y\"\n");
        output.push_str("    startTimeCode = 0\n");
        output.push_str(&format!(
            "    endTimeCode = {}\n",
            self.frame_count.saturating_sub(1)
        ));
        output.push_str(&format!("    timeCodesPerSecond = {}\n", self.fps));
        output.push_str(&format!("    framesPerSecond = {}\n", self.fps));
        output.push_str(")\n\n");

        // Add animated camera
        self.write_camera(&mut output);

        output.push_str("def Xform \"Animation\" (\n");
        output.push_str("    kind = \"component\"\n");
        output.push_str(")\n");
        output.push_str("{\n");

        // Build time samples for each joint
        let joint_radius = 0.01f32; // 1cm radius in meters

        if let Some(first_frame) = self.frames.first() {
            let num_joints = first_frame.joint_positions.len();

            // Joints scope
            output.push_str("    def Scope \"Joints\"\n");
            output.push_str("    {\n");

            for joint_idx in 0..num_joints {
                let mut samples = TransformTimeSamples::new();

                for (frame_num, frame) in self.frames.iter().enumerate() {
                    if joint_idx < frame.joint_positions.len() {
                        let pos = frame.joint_positions[joint_idx] * export_scale;
                        // Identity rotation/scale, just translation
                        // For spheres we use uniform scale for radius
                        let matrix = [
                            joint_radius, 0.0, 0.0, 0.0,
                            0.0, joint_radius, 0.0, 0.0,
                            0.0, 0.0, joint_radius, 0.0,
                            pos.x, pos.y, pos.z, 1.0,
                        ];
                        samples.add_sample(frame_num, matrix);
                    }
                }

                output.push_str(&format!("        def Sphere \"Joint_{:04}\"\n", joint_idx));
                output.push_str("        {\n");
                output.push_str(&format!(
                    "            matrix4d xformOp:transform.timeSamples = {}\n",
                    samples.to_usd_string()
                ));
                output.push_str(
                    "            uniform token[] xformOpOrder = [\"xformOp:transform\"]\n",
                );
                output.push_str("        }\n");
            }

            output.push_str("    }\n\n");

            // Intervals scope
            output.push_str("    def Scope \"Intervals\"\n");
            output.push_str("    {\n");

            // Group intervals by role
            let mut push_indices = Vec::new();
            let mut pull_indices = Vec::new();

            for (idx, &(alpha, omega, role)) in first_frame.interval_endpoints.iter().enumerate() {
                if role == Role::Support {
                    continue;
                }
                match role {
                    Role::Pushing => push_indices.push((idx, alpha, omega)),
                    _ if role.is_pull_like() => pull_indices.push((idx, alpha, omega)),
                    _ => {}
                }
            }

            // Push intervals
            if !push_indices.is_empty() {
                output.push_str("        def Scope \"Push\"\n");
                output.push_str("        {\n");

                for (local_idx, (_, alpha, omega)) in push_indices.iter().enumerate() {
                    let samples = self.build_cylinder_time_samples(
                        *alpha,
                        *omega,
                        0.012, // 1.2cm radius for push
                        export_scale,
                    );

                    output.push_str(&format!(
                        "            def Cylinder \"Push_{:04}\"\n",
                        local_idx
                    ));
                    output.push_str("            {\n");
                    output.push_str(&format!(
                        "                matrix4d xformOp:transform.timeSamples = {}\n",
                        samples.to_usd_string()
                    ));
                    output.push_str(
                        "                uniform token[] xformOpOrder = [\"xformOp:transform\"]\n",
                    );
                    output.push_str("            }\n");
                }

                output.push_str("        }\n\n");
            }

            // Pull intervals
            if !pull_indices.is_empty() {
                output.push_str("        def Scope \"Pull\"\n");
                output.push_str("        {\n");

                for (local_idx, (_, alpha, omega)) in pull_indices.iter().enumerate() {
                    let samples = self.build_cylinder_time_samples(
                        *alpha,
                        *omega,
                        0.005, // 5mm radius for pull
                        export_scale,
                    );

                    output.push_str(&format!(
                        "            def Cylinder \"Pull_{:04}\"\n",
                        local_idx
                    ));
                    output.push_str("            {\n");
                    output.push_str(&format!(
                        "                matrix4d xformOp:transform.timeSamples = {}\n",
                        samples.to_usd_string()
                    ));
                    output.push_str(
                        "                uniform token[] xformOpOrder = [\"xformOp:transform\"]\n",
                    );
                    output.push_str("            }\n");
                }

                output.push_str("        }\n");
            }

            output.push_str("    }\n");
        }

        output.push_str("}\n");

        Ok(output)
    }

    /// Write environment elements (sky dome with clouds, grass ground plane)
    fn write_environment(&self, output: &mut String) {
        // Ground plane with grass material
        output.push_str("def Xform \"Environment\"\n");
        output.push_str("{\n");

        // Large ground plane (100m x 100m)
        output.push_str("    def Mesh \"Ground\"\n");
        output.push_str("    {\n");
        output.push_str("        int[] faceVertexCounts = [4]\n");
        output.push_str("        int[] faceVertexIndices = [0, 1, 2, 3]\n");
        output.push_str("        point3f[] points = [(-50, 0, -50), (50, 0, -50), (50, 0, 50), (-50, 0, 50)]\n");
        output.push_str("        texCoord2f[] primvars:st = [(0, 0), (20, 0), (20, 20), (0, 20)] (\n");
        output.push_str("            interpolation = \"vertex\"\n");
        output.push_str("        )\n");
        output.push_str("        rel material:binding = </Environment/Materials/GrassMaterial>\n");
        output.push_str("    }\n\n");

        // Sky dome (large inverted sphere)
        output.push_str("    def Sphere \"SkyDome\"\n");
        output.push_str("    {\n");
        output.push_str("        double radius = 500\n");
        output.push_str("        rel material:binding = </Environment/Materials/SkyMaterial>\n");
        output.push_str("    }\n\n");

        // Materials scope
        output.push_str("    def Scope \"Materials\"\n");
        output.push_str("    {\n");

        // Grass material - green with slight variation
        output.push_str("        def Material \"GrassMaterial\"\n");
        output.push_str("        {\n");
        output.push_str("            token outputs:surface.connect = </Environment/Materials/GrassMaterial/GrassShader.outputs:surface>\n");
        output.push_str("            \n");
        output.push_str("            def Shader \"GrassShader\"\n");
        output.push_str("            {\n");
        output.push_str("                uniform token info:id = \"UsdPreviewSurface\"\n");
        // Grass green color
        output.push_str("                color3f inputs:diffuseColor = (0.15, 0.45, 0.12)\n");
        output.push_str("                float inputs:roughness = 0.9\n");
        output.push_str("                float inputs:metallic = 0.0\n");
        output.push_str("                token outputs:surface\n");
        output.push_str("            }\n");
        output.push_str("        }\n\n");

        // Sky material - gradient from light blue to white (simulating clouds)
        output.push_str("        def Material \"SkyMaterial\"\n");
        output.push_str("        {\n");
        output.push_str("            token outputs:surface.connect = </Environment/Materials/SkyMaterial/SkyShader.outputs:surface>\n");
        output.push_str("            \n");
        output.push_str("            def Shader \"SkyShader\"\n");
        output.push_str("            {\n");
        output.push_str("                uniform token info:id = \"UsdPreviewSurface\"\n");
        // Light sky blue - partly cloudy feel
        output.push_str("                color3f inputs:diffuseColor = (0.53, 0.73, 0.87)\n");
        // Make it emissive so it glows like a sky
        output.push_str("                color3f inputs:emissiveColor = (0.6, 0.78, 0.92)\n");
        output.push_str("                float inputs:roughness = 1.0\n");
        output.push_str("                float inputs:metallic = 0.0\n");
        output.push_str("                token outputs:surface\n");
        output.push_str("            }\n");
        output.push_str("        }\n");

        output.push_str("    }\n");  // Close Materials scope
        output.push_str("}\n\n");     // Close Environment xform

        // Add a sun light for better illumination
        output.push_str("def DistantLight \"Sun\"\n");
        output.push_str("{\n");
        output.push_str("    float inputs:angle = 0.53\n");  // Sun's angular diameter
        output.push_str("    color3f inputs:color = (1.0, 0.98, 0.95)\n");  // Warm white
        output.push_str("    float inputs:intensity = 5000\n");
        // Position sun at an angle (morning/afternoon feel)
        output.push_str("    float3 xformOp:rotateXYZ = (-45, 30, 0)\n");
        output.push_str("    uniform token[] xformOpOrder = [\"xformOp:rotateXYZ\"]\n");
        output.push_str("}\n\n");
    }

    /// Write animated camera with time-sampled look-at transform
    fn write_camera(&self, output: &mut String) {
        // Check if we have camera data
        let has_camera = self.frames.iter().any(|f| f.camera_pos.is_some());
        if !has_camera {
            return;
        }

        let export_scale = self.scale / 1000.0;

        output.push_str("def Camera \"Camera\"\n");
        output.push_str("{\n");
        output.push_str("    float focalLength = 50.0\n");
        output.push_str("    float horizontalAperture = 36.0\n");
        output.push_str("    float verticalAperture = 24.0\n");
        output.push_str("    float2 clippingRange = (0.1, 1000.0)\n");

        // Build time-sampled transform
        let mut frames_str = String::new();
        frames_str.push_str("{\n");

        let mut first = true;
        for (frame_num, frame) in self.frames.iter().enumerate() {
            if let (Some(pos), Some(target)) = (frame.camera_pos, frame.camera_target) {
                let pos = pos * export_scale;
                let target = target * export_scale;

                // Build look-at matrix
                let forward = (target - pos).normalize();
                let world_up = Vector3::new(0.0f32, 1.0, 0.0);
                let right = forward.cross(world_up).normalize();
                let up = right.cross(forward).normalize();

                // Camera in USD/Blender looks down -Z, so we need to adjust
                // The matrix transforms from camera space to world space
                // Camera's local -Z should point at target (forward direction)
                // So we use: X = right, Y = up, Z = -forward
                let neg_forward = -forward;

                // Build 4x4 matrix (row-vector convention)
                // Row 0: right (X axis)
                // Row 1: up (Y axis)
                // Row 2: -forward (Z axis, camera looks down -Z)
                // Row 3: position
                if !first {
                    frames_str.push_str(",\n");
                }
                first = false;

                frames_str.push_str(&format!(
                    "        {}: ( ({:.6}, {:.6}, {:.6}, 0), ({:.6}, {:.6}, {:.6}, 0), ({:.6}, {:.6}, {:.6}, 0), ({:.6}, {:.6}, {:.6}, 1) )",
                    frame_num,
                    right.x, right.y, right.z,
                    up.x, up.y, up.z,
                    neg_forward.x, neg_forward.y, neg_forward.z,
                    pos.x, pos.y, pos.z
                ));
            }
        }

        frames_str.push_str("\n    }");

        output.push_str(&format!("    matrix4d xformOp:transform.timeSamples = {}\n", frames_str));
        output.push_str("    uniform token[] xformOpOrder = [\"xformOp:transform\"]\n");
        output.push_str("}\n\n");
    }

    /// Build time samples for a cylinder connecting two joints
    fn build_cylinder_time_samples(
        &self,
        alpha_idx: usize,
        omega_idx: usize,
        radius: f32,
        export_scale: f32,
    ) -> TransformTimeSamples {
        let mut samples = TransformTimeSamples::new();

        for (frame_num, frame) in self.frames.iter().enumerate() {
            if alpha_idx >= frame.joint_positions.len()
                || omega_idx >= frame.joint_positions.len()
            {
                continue;
            }

            let alpha_loc = frame.joint_positions[alpha_idx] * export_scale;
            let omega_loc = frame.joint_positions[omega_idx] * export_scale;

            // Midpoint
            let mid_x = (alpha_loc.x + omega_loc.x) / 2.0;
            let mid_y = (alpha_loc.y + omega_loc.y) / 2.0;
            let mid_z = (alpha_loc.z + omega_loc.z) / 2.0;

            // Direction and length
            let dx = omega_loc.x - alpha_loc.x;
            let dy = omega_loc.y - alpha_loc.y;
            let dz = omega_loc.z - alpha_loc.z;
            let length = (dx * dx + dy * dy + dz * dz).sqrt();

            if length < 1e-6 {
                continue; // Skip degenerate intervals
            }

            // Build orthonormal basis
            let y_axis = Vector3::new(dx / length, dy / length, dz / length);
            let arbitrary = if y_axis.y.abs() < 0.9 {
                Vector3::new(0.0, 1.0, 0.0)
            } else {
                Vector3::new(1.0, 0.0, 0.0)
            };

            let x_axis = y_axis.cross(arbitrary).normalize();
            let z_axis = x_axis.cross(y_axis).normalize();

            // Scale columns: X and Z by radius, Y by length/2 (USD cylinder height=2)
            let c0 = x_axis * radius;
            let c1 = y_axis * (length / 2.0);
            let c2 = z_axis * radius;

            // USD row-vector format: translation in last row
            let matrix = [
                c0.x, c0.y, c0.z, 0.0,
                c1.x, c1.y, c1.z, 0.0,
                c2.x, c2.y, c2.z, 0.0,
                mid_x, mid_y, mid_z, 1.0,
            ];

            samples.add_sample(frame_num, matrix);
        }

        samples
    }

    /// Check if we should capture this iteration
    fn should_capture(&self) -> bool {
        self.enabled && (self.iteration_count % self.capture_interval == 0)
    }

    /// Capture a frame from the current fabric state
    pub fn capture_frame(
        &mut self,
        fabric: &Fabric,
        camera_pos: Option<Point3<f32>>,
        camera_target: Option<Point3<f32>>,
    ) -> io::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        self.iteration_count += 1;

        if self.should_capture() {
            // Store scale and name on first frame
            if self.frames.is_empty() {
                self.scale = fabric.scale;
                self.fabric_name = fabric.name.clone();
            }

            // Capture joint positions
            let joint_positions: Vec<Point3<f32>> = fabric
                .joints
                .iter()
                .map(|j| j.location)
                .collect();

            // Capture interval topology (only need this once, but we store per frame for safety)
            let interval_endpoints: Vec<(usize, usize, Role)> = fabric
                .interval_values()
                .map(|interval: &Interval| (interval.alpha_index, interval.omega_index, interval.role))
                .collect();

            self.frames.push(FrameData {
                joint_positions,
                interval_endpoints,
                camera_pos,
                camera_target,
            });

            self.frame_count += 1;

            if self.frame_count % 10 == 0 {
                println!("Captured {} frames...", self.frame_count);
            }
        }

        Ok(())
    }

    /// Get the current frame count
    pub fn frame_count(&self) -> usize {
        self.frame_count
    }

    /// Check if export is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Toggle export on/off, returning new state
    pub fn toggle(&mut self) -> io::Result<bool> {
        if self.enabled {
            self.stop()?;
            Ok(false)
        } else {
            self.start();
            Ok(true)
        }
    }
}
