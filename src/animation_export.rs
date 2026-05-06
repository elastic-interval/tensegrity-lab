/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;

use glam::Vec3;
use serde::Serialize;
use slotmap::Key;

use crate::fabric::interval::Role;
use crate::fabric::{Fabric, JointKey};
use crate::units::Unit;

const DEFAULT_EXPORT_FPS: f64 = 100.0;

const JOINT_RADIUS: f32 = 0.015;
const HOLDER_RADIUS_RATIO: f32 = 1.0 / 5.0;

#[derive(Serialize)]
struct ExportData {
    /// Frames per second for animation playback
    fps: f64,
    /// Prototype dimensions for reference
    prototypes: PrototypeDimensions,
    /// All captured frames
    frames: Vec<FrameExport>,
}

#[derive(Serialize)]
struct PrototypeDimensions {
    joint_radius: f32,
    push_radius: f32,
    holder_radius: f32,
    pull_radius: f32,
}

#[derive(Serialize)]
struct FrameExport {
    joints: Vec<JointExport>,
    intervals: IntervalsExport,
}

#[derive(Serialize)]
struct JointExport {
    name: String,
    matrix: [f32; 16],
}

#[derive(Serialize)]
struct IntervalsExport {
    push: Vec<IntervalExport>,
    pull: Vec<IntervalExport>,
}

#[derive(Serialize)]
struct IntervalExport {
    name: String,
    matrix: [f32; 16],
}

struct FrameData {
    /// (joint_key, position) - uses actual slotmap keys for stable identity
    joints: Vec<(JointKey, Vec3)>,
    /// (alpha_joint_key, omega_joint_key, role) - uses actual slotmap keys for stable identity
    interval_data: Vec<(JointKey, JointKey, Role)>,
}

pub struct AnimationExporter {
    output_dir: PathBuf,
    frame_count: usize,
    enabled: bool,
    frames: Vec<FrameData>,
    iteration_count: usize,
    iterations_per_frame: usize,
    fps: f64,
    push_radius: f32,
    pull_radius: f32,
}

impl AnimationExporter {
    pub fn new<P: Into<PathBuf>>(output_dir: P, fps: f64) -> Self {
        let fps = if fps > 0.0 { fps } else { DEFAULT_EXPORT_FPS };
        let iterations_per_frame = (1.0 / fps / 0.00005) as usize;
        Self {
            output_dir: output_dir.into(),
            frame_count: 0,
            enabled: false,
            frames: Vec::new(),
            iteration_count: 0,
            iterations_per_frame,
            fps,
            push_radius: 0.0,
            pull_radius: 0.0,
        }
    }

    pub fn start(&mut self) {
        self.enabled = true;
        self.frame_count = 0;
        self.frames.clear();
        self.iteration_count = 0;
        println!(
            "Animation export started at {} FPS ({} iterations/frame)",
            self.fps, self.iterations_per_frame
        );
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

        println!(
            "Creating animation JSON with {} frames...",
            self.frame_count
        );

        let json_path = self.output_dir.with_file_name("animation.json");
        let export_data = self.create_export_data();
        let json = serde_json::to_string_pretty(&export_data)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let mut file = File::create(&json_path)?;
        file.write_all(json.as_bytes())?;

        self.frames.clear();

        println!("Saved: {:?}", json_path);
        println!("Frames: {}", self.frame_count);
        Ok(())
    }

    fn create_export_data(&self) -> ExportData {
        // Coordinates are already in meters
        let frames: Vec<FrameExport> = self
            .frames
            .iter()
            .map(|frame| self.export_frame(frame))
            .collect();

        ExportData {
            fps: self.fps,
            prototypes: PrototypeDimensions {
                joint_radius: JOINT_RADIUS,
                push_radius: self.push_radius,
                holder_radius: self.push_radius * HOLDER_RADIUS_RATIO,
                pull_radius: self.pull_radius,
            },
            frames,
        }
    }

    fn export_frame(&self, frame: &FrameData) -> FrameExport {
        // Build key-to-position map for interval lookups
        let key_to_pos: std::collections::HashMap<JointKey, Vec3> =
            frame.joints.iter().map(|&(key, pos)| (key, pos)).collect();

        // Export joints using stable key identifiers (as_ffi() gives unique u64 per key)
        let joints: Vec<JointExport> = frame
            .joints
            .iter()
            .map(|&(key, pos)| JointExport {
                name: format!("Joint_{}", key.data().as_ffi()),
                matrix: create_sphere_matrix(pos, JOINT_RADIUS),
            })
            .collect();

        let mut push_intervals = Vec::new();
        let mut pull_intervals = Vec::new();

        for &(alpha_key, omega_key, role) in &frame.interval_data {
            if role == Role::Support {
                continue;
            }
            match role {
                Role::Pushing => push_intervals.push((alpha_key, omega_key)),
                _ if role.is_pull_like() => pull_intervals.push((alpha_key, omega_key)),
                _ => {}
            }
        }

        let push: Vec<IntervalExport> = push_intervals
            .iter()
            .filter_map(|(alpha_key, omega_key)| {
                let alpha_pos = *key_to_pos.get(alpha_key)?;
                let omega_pos = *key_to_pos.get(omega_key)?;

                let delta = omega_pos - alpha_pos;
                let full_length = delta.length();
                if full_length < 1e-6 {
                    return None;
                }

                let mid = (alpha_pos + omega_pos) / 2.0;

                let (x_axis, y_axis, z_axis) = compute_cylinder_axes(delta, full_length);

                let matrix = create_cylinder_matrix(
                    mid,
                    x_axis,
                    y_axis,
                    z_axis,
                    self.push_radius,
                    full_length,
                );

                // Name by stable joint key identifiers for consistent identity across frames
                Some(IntervalExport {
                    name: format!(
                        "Push_{}_{}",
                        alpha_key.data().as_ffi(),
                        omega_key.data().as_ffi()
                    ),
                    matrix,
                })
            })
            .collect();

        let pull: Vec<IntervalExport> = pull_intervals
            .iter()
            .filter_map(|(alpha_key, omega_key)| {
                let alpha_pos = *key_to_pos.get(alpha_key)?;
                let omega_pos = *key_to_pos.get(omega_key)?;

                let delta = omega_pos - alpha_pos;
                let full_length = delta.length();
                if full_length < 1e-6 {
                    return None;
                }

                let mid = (alpha_pos + omega_pos) / 2.0;

                let (x_axis, y_axis, z_axis) = compute_cylinder_axes(delta, full_length);

                let matrix = create_cylinder_matrix(
                    mid,
                    x_axis,
                    y_axis,
                    z_axis,
                    self.pull_radius,
                    full_length,
                );

                // Name by stable joint key identifiers for consistent identity across frames
                Some(IntervalExport {
                    name: format!(
                        "Pull_{}_{}",
                        alpha_key.data().as_ffi(),
                        omega_key.data().as_ffi()
                    ),
                    matrix,
                })
            })
            .collect();

        FrameExport {
            joints,
            intervals: IntervalsExport { push, pull },
        }
    }

    pub fn tick(&mut self, fabric: &Fabric, iterations: usize) {
        if !self.enabled || iterations == 0 {
            return;
        }

        self.push_radius = fabric.dimensions.hinge.push_radius.f32();
        self.pull_radius = fabric.dimensions.pull_radius.f32();

        let prev_frame = self.iteration_count / self.iterations_per_frame;
        self.iteration_count += iterations;
        let curr_frame = self.iteration_count / self.iterations_per_frame;

        if curr_frame == prev_frame {
            return;
        }

        // Collect joints with their stable keys
        let joints: Vec<(JointKey, Vec3)> = fabric
            .joints
            .iter()
            .map(|(key, joint)| (key, joint.location))
            .collect();

        // Collect interval data with stable keys
        let interval_data: Vec<(JointKey, JointKey, Role)> = fabric
            .intervals
            .values()
            .map(|interval| (interval.alpha_key, interval.omega_key, interval.role))
            .collect();

        self.frames.push(FrameData {
            joints,
            interval_data,
        });

        self.frame_count += 1;

        if self.frame_count % self.fps as usize == 0 {
            let real_seconds = self.iteration_count as f64 * 0.00005;
            println!(
                "Captured {} frames ({:.1}s)",
                self.frame_count, real_seconds
            );
        }
    }

    pub fn frame_count(&self) -> usize {
        self.frame_count
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_fps(&mut self, fps: f64) {
        let fps = if fps > 0.0 { fps } else { DEFAULT_EXPORT_FPS };
        self.fps = fps;
        self.iterations_per_frame = (1.0 / fps / 0.00005) as usize;
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

    pub fn snapshot(&mut self, fabric: &Fabric) -> io::Result<PathBuf> {
        self.push_radius = fabric.dimensions.hinge.push_radius.f32();
        self.pull_radius = fabric.dimensions.pull_radius.f32();
        self.frames.clear();
        self.frame_count = 0;

        // Collect joints with their stable keys
        let joints: Vec<(JointKey, Vec3)> = fabric
            .joints
            .iter()
            .map(|(key, joint)| (key, joint.location))
            .collect();

        // Collect interval data with stable keys
        let interval_data: Vec<(JointKey, JointKey, Role)> = fabric
            .intervals
            .values()
            .map(|interval| (interval.alpha_key, interval.omega_key, interval.role))
            .collect();

        self.frames.push(FrameData {
            joints,
            interval_data,
        });
        self.frame_count = 1;

        let json_path = self.output_dir.with_file_name("snapshot.json");

        let export_data = self.create_export_data();
        let json = serde_json::to_string_pretty(&export_data)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let mut file = File::create(&json_path)?;
        file.write_all(json.as_bytes())?;

        self.frames.clear();

        println!("Snapshot saved: {:?}", json_path);
        Ok(json_path)
    }
}

fn compute_cylinder_axes(delta: Vec3, length: f32) -> (Vec3, Vec3, Vec3) {
    let dir = delta / length;
    let y_axis = dir;
    let arbitrary = if y_axis.y.abs() < 0.9 {
        Vec3::Y
    } else {
        Vec3::X
    };
    let x_axis = y_axis.cross(arbitrary).normalize();
    let z_axis = x_axis.cross(y_axis).normalize();
    (x_axis, y_axis, z_axis)
}

fn create_sphere_matrix(pos: Vec3, radius: f32) -> [f32; 16] {
    // Column-major 4x4 matrix for uniform scale + translation
    [
        radius, 0.0, 0.0, 0.0, // column 0
        0.0, radius, 0.0, 0.0, // column 1
        0.0, 0.0, radius, 0.0, // column 2
        pos.x, pos.y, pos.z, 1.0, // column 3
    ]
}

fn create_cylinder_matrix(
    mid: Vec3,
    x_axis: Vec3,
    y_axis: Vec3,
    z_axis: Vec3,
    radius: f32,
    length: f32,
) -> [f32; 16] {
    // Blender cylinder: height=2 (from -1 to +1), radius=1
    // Scale: x,z by radius, y by length/2
    let c0 = x_axis * radius;
    let c1 = y_axis * (length / 2.0);
    let c2 = z_axis * radius;

    // Column-major 4x4 matrix
    [
        c0.x, c0.y, c0.z, 0.0, // column 0
        c1.x, c1.y, c1.z, 0.0, // column 1
        c2.x, c2.y, c2.z, 0.0, // column 2
        mid.x, mid.y, mid.z, 1.0, // column 3
    ]
}
