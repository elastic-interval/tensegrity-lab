/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

mod camera;
mod geometry;
mod light;
mod material;
mod usd;

use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;

use cgmath::Point3;

use crate::fabric::interval::{Interval, Role};
use crate::fabric::Fabric;

use camera::CameraRig;
use geometry::{AnimatedCylinder, AnimatedSphere, Environment};
use material::MaterialScope;
use usd::UsdHeader;

const HEADLIGHT_WATTS: f32 = 600.0;
const JOINT_RADIUS: f32 = 0.01;
const PUSH_RADIUS: f32 = 0.030;
const HOLDER_RADIUS: f32 = PUSH_RADIUS / 5.0;
const PULL_RADIUS: f32 = 0.005;

struct FrameData {
    joint_positions: Vec<Point3<f32>>,
    interval_endpoints: Vec<(usize, usize, Role)>,
    camera_pos: Option<Point3<f32>>,
    camera_target: Option<Point3<f32>>,
}

pub struct AnimationExporter {
    output_dir: PathBuf,
    frame_count: usize,
    fps: f64,
    enabled: bool,
    frames: Vec<FrameData>,
    scale: f32,
}

impl AnimationExporter {
    pub fn new<P: Into<PathBuf>>(output_dir: P, fps: f64) -> Self {
        Self {
            output_dir: output_dir.into(),
            frame_count: 0,
            fps,
            enabled: false,
            frames: Vec::new(),
            scale: 1.0,
        }
    }

    pub fn start(&mut self) {
        self.enabled = true;
        self.frame_count = 0;
        self.frames.clear();
        println!("Animation export started");
    }

    pub fn stop(&mut self) -> io::Result<()> {
        if !self.enabled {
            return Ok(());
        }
        self.enabled = false;

        if self.frame_count == 0 {
            println!("No frames captured");
            return Ok(());
        }

        println!("Creating USD animation with {} frames...", self.frame_count);

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let usd_path = self.output_dir.with_file_name(format!("animation_{}.usda", timestamp));

        let usd_content = self.create_usd()?;

        let mut file = File::create(&usd_path)?;
        file.write_all(usd_content.as_bytes())?;

        self.frames.clear();

        println!("Saved: {:?}", usd_path);
        println!("Frames: {}", self.frame_count);
        Ok(())
    }

    fn create_usd(&self) -> io::Result<String> {
        let mut output = String::new();
        let export_scale = self.scale / 1000.0;

        let header = UsdHeader::new("Animation")
            .with_animation(0, self.frame_count.saturating_sub(1), self.fps);
        output.push_str(&header.to_string());
        output.push('\n');

        output.push_str(&Environment::new().to_string());
        output.push('\n');

        output.push_str(&MaterialScope::fabric_defaults().to_string());
        output.push('\n');

        let mut camera_rig = CameraRig::new("CameraRig").with_headlights(HEADLIGHT_WATTS);
        for (frame_num, frame) in self.frames.iter().enumerate() {
            if let (Some(pos), Some(target)) = (frame.camera_pos, frame.camera_target) {
                camera_rig.add_look_at_frame(frame_num, pos, target, export_scale);
            }
        }
        output.push_str(&camera_rig.to_string());
        output.push('\n');

        output.push_str("def Xform \"Animation\" (\n");
        output.push_str("    kind = \"component\"\n");
        output.push_str(")\n");
        output.push_str("{\n");

        if let Some(first_frame) = self.frames.first() {
            self.write_joints(&mut output, first_frame.joint_positions.len(), export_scale);
            self.write_intervals(&mut output, first_frame, export_scale);
        }

        output.push_str("}\n");
        Ok(output)
    }

    fn write_joints(&self, output: &mut String, num_joints: usize, export_scale: f32) {
        output.push_str("    def Scope \"Joints\"\n");
        output.push_str("    {\n");

        for joint_idx in 0..num_joints {
            let mut sphere = AnimatedSphere::new(&format!("Joint_{:04}", joint_idx), JOINT_RADIUS);

            for (frame_num, frame) in self.frames.iter().enumerate() {
                if joint_idx < frame.joint_positions.len() {
                    let pos = frame.joint_positions[joint_idx] * export_scale;
                    sphere.add_position(frame_num, pos);
                }
            }

            output.push_str(&sphere.to_string());
        }

        output.push_str("    }\n\n");
    }

    fn write_intervals(&self, output: &mut String, first_frame: &FrameData, export_scale: f32) {
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

        output.push_str("    def Scope \"Intervals\"\n");
        output.push_str("    {\n");

        if !push_indices.is_empty() {
            output.push_str("        def Scope \"Push\"\n");
            output.push_str("        {\n");

            for (local_idx, (_, alpha, omega)) in push_indices.iter().enumerate() {
                let mut push_bar = AnimatedCylinder::new(&format!("Push_{:04}", local_idx), PUSH_RADIUS)
                    .with_material("/Materials/AluminumMaterial");

                let mut holder = AnimatedCylinder::new(&format!("PushHolder_{:04}", local_idx), HOLDER_RADIUS)
                    .with_material("/Materials/AluminumMaterial");

                for (frame_num, frame) in self.frames.iter().enumerate() {
                    if *alpha < frame.joint_positions.len() && *omega < frame.joint_positions.len() {
                        let alpha_pos = frame.joint_positions[*alpha] * export_scale;
                        let omega_pos = frame.joint_positions[*omega] * export_scale;

                        push_bar.add_endpoints_with_inset(frame_num, alpha_pos, omega_pos, JOINT_RADIUS);
                        holder.add_endpoints(frame_num, alpha_pos, omega_pos);
                    }
                }

                output.push_str(&push_bar.to_string());
                output.push_str(&holder.to_string());
            }

            output.push_str("        }\n\n");
        }

        if !pull_indices.is_empty() {
            output.push_str("        def Scope \"Pull\"\n");
            output.push_str("        {\n");

            for (local_idx, (_, alpha, omega)) in pull_indices.iter().enumerate() {
                let mut pull = AnimatedCylinder::new(&format!("Pull_{:04}", local_idx), PULL_RADIUS)
                    .with_material("/Materials/RopeMaterial");

                for (frame_num, frame) in self.frames.iter().enumerate() {
                    if *alpha < frame.joint_positions.len() && *omega < frame.joint_positions.len() {
                        let alpha_pos = frame.joint_positions[*alpha] * export_scale;
                        let omega_pos = frame.joint_positions[*omega] * export_scale;
                        pull.add_endpoints(frame_num, alpha_pos, omega_pos);
                    }
                }

                output.push_str(&pull.to_string());
            }

            output.push_str("        }\n");
        }

        output.push_str("    }\n");
    }

    pub fn capture_frame(
        &mut self,
        fabric: &Fabric,
        camera_pos: Option<Point3<f32>>,
        camera_target: Option<Point3<f32>>,
    ) -> io::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        if self.frames.is_empty() {
            self.scale = fabric.scale;
        }

        let joint_positions: Vec<Point3<f32>> = fabric.joints.iter().map(|j| j.location).collect();

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

        Ok(())
    }

    pub fn frame_count(&self) -> usize {
        self.frame_count
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

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
