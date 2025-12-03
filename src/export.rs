/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;

use cgmath::{InnerSpace, Point3, Vector3};
use serde::Serialize;

use crate::fabric::interval::{Interval, Role};
use crate::fabric::Fabric;

const JOINT_RADIUS: f32 = 0.015;
const PUSH_RADIUS: f32 = 0.04;
const HOLDER_RADIUS: f32 = PUSH_RADIUS / 5.0;
const PULL_RADIUS: f32 = 0.007;

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
    camera: Option<CameraExport>,
    joints: Vec<JointExport>,
    intervals: IntervalsExport,
}

#[derive(Serialize)]
struct CameraExport {
    position: [f32; 3],
    target: [f32; 3],
}

#[derive(Serialize)]
struct JointExport {
    name: String,
    matrix: [f32; 16],
}

#[derive(Serialize)]
struct IntervalsExport {
    push: Vec<PushExport>,
    pull: Vec<PullExport>,
}

#[derive(Serialize)]
struct PushExport {
    name: String,
    bar: TransformExport,
    holder: TransformExport,
}

#[derive(Serialize)]
struct PullExport {
    name: String,
    matrix: [f32; 16],
}

#[derive(Serialize)]
struct TransformExport {
    matrix: [f32; 16],
}

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

        println!("Creating animation JSON with {} frames...", self.frame_count);

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
        let export_scale = self.scale / 1000.0;

        let frames: Vec<FrameExport> = self
            .frames
            .iter()
            .map(|frame| self.export_frame(frame, export_scale))
            .collect();

        ExportData {
            fps: self.fps,
            prototypes: PrototypeDimensions {
                joint_radius: JOINT_RADIUS,
                push_radius: PUSH_RADIUS,
                holder_radius: HOLDER_RADIUS,
                pull_radius: PULL_RADIUS,
            },
            frames,
        }
    }

    fn export_frame(&self, frame: &FrameData, export_scale: f32) -> FrameExport {
        let camera = frame.camera_pos.zip(frame.camera_target).map(|(pos, target)| {
            CameraExport {
                position: [pos.x * export_scale, pos.y * export_scale, pos.z * export_scale],
                target: [target.x * export_scale, target.y * export_scale, target.z * export_scale],
            }
        });

        let joints: Vec<JointExport> = frame
            .joint_positions
            .iter()
            .enumerate()
            .map(|(idx, pos)| {
                let scaled = *pos * export_scale;
                JointExport {
                    name: format!("Joint_{:04}", idx),
                    matrix: create_sphere_matrix(scaled, JOINT_RADIUS),
                }
            })
            .collect();

        let mut push_intervals = Vec::new();
        let mut pull_intervals = Vec::new();

        for &(alpha, omega, role) in &frame.interval_endpoints {
            if role == Role::Support {
                continue;
            }
            match role {
                Role::Pushing => push_intervals.push((alpha, omega)),
                _ if role.is_pull_like() => pull_intervals.push((alpha, omega)),
                _ => {}
            }
        }

        let push: Vec<PushExport> = push_intervals
            .iter()
            .enumerate()
            .filter_map(|(idx, (alpha, omega))| {
                if *alpha >= frame.joint_positions.len() || *omega >= frame.joint_positions.len() {
                    return None;
                }
                let alpha_pos = frame.joint_positions[*alpha] * export_scale;
                let omega_pos = frame.joint_positions[*omega] * export_scale;

                let delta = omega_pos - alpha_pos;
                let full_length = delta.magnitude();
                if full_length < 1e-6 {
                    return None;
                }

                let mid = Point3::new(
                    (alpha_pos.x + omega_pos.x) / 2.0,
                    (alpha_pos.y + omega_pos.y) / 2.0,
                    (alpha_pos.z + omega_pos.z) / 2.0,
                );

                let (x_axis, y_axis, z_axis) = compute_cylinder_axes(delta, full_length);

                // Bar: inset from joints
                let bar_length = full_length - 2.0 * JOINT_RADIUS;
                let bar_matrix = if bar_length > 0.0 {
                    create_cylinder_matrix(mid, x_axis, y_axis, z_axis, PUSH_RADIUS, bar_length)
                } else {
                    [0.0; 16] // Degenerate case
                };

                // Holder: full length
                let holder_matrix = create_cylinder_matrix(mid, x_axis, y_axis, z_axis, HOLDER_RADIUS, full_length);

                Some(PushExport {
                    name: format!("Push_{:04}", idx),
                    bar: TransformExport { matrix: bar_matrix },
                    holder: TransformExport { matrix: holder_matrix },
                })
            })
            .collect();

        let pull: Vec<PullExport> = pull_intervals
            .iter()
            .enumerate()
            .filter_map(|(idx, (alpha, omega))| {
                if *alpha >= frame.joint_positions.len() || *omega >= frame.joint_positions.len() {
                    return None;
                }
                let alpha_pos = frame.joint_positions[*alpha] * export_scale;
                let omega_pos = frame.joint_positions[*omega] * export_scale;

                let delta = omega_pos - alpha_pos;
                let full_length = delta.magnitude();
                if full_length < 1e-6 {
                    return None;
                }

                let mid = Point3::new(
                    (alpha_pos.x + omega_pos.x) / 2.0,
                    (alpha_pos.y + omega_pos.y) / 2.0,
                    (alpha_pos.z + omega_pos.z) / 2.0,
                );

                let (x_axis, y_axis, z_axis) = compute_cylinder_axes(delta, full_length);
                let matrix = create_cylinder_matrix(mid, x_axis, y_axis, z_axis, PULL_RADIUS, full_length);

                Some(PullExport {
                    name: format!("Pull_{:04}", idx),
                    matrix,
                })
            })
            .collect();

        FrameExport {
            camera,
            joints,
            intervals: IntervalsExport { push, pull },
        }
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

    pub fn snapshot(
        &mut self,
        fabric: &Fabric,
        camera_pos: Option<Point3<f32>>,
        camera_target: Option<Point3<f32>>,
    ) -> io::Result<PathBuf> {
        self.scale = fabric.scale;
        self.frames.clear();
        self.frame_count = 0;

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

fn compute_cylinder_axes(delta: Vector3<f32>, length: f32) -> (Vector3<f32>, Vector3<f32>, Vector3<f32>) {
    let dir = delta / length;
    let y_axis = dir;
    let arbitrary = if y_axis.y.abs() < 0.9 {
        Vector3::new(0.0, 1.0, 0.0)
    } else {
        Vector3::new(1.0, 0.0, 0.0)
    };
    let x_axis = y_axis.cross(arbitrary).normalize();
    let z_axis = x_axis.cross(y_axis).normalize();
    (x_axis, y_axis, z_axis)
}

fn create_sphere_matrix(pos: Point3<f32>, radius: f32) -> [f32; 16] {
    // Column-major 4x4 matrix for uniform scale + translation
    [
        radius, 0.0, 0.0, 0.0,      // column 0
        0.0, radius, 0.0, 0.0,      // column 1
        0.0, 0.0, radius, 0.0,      // column 2
        pos.x, pos.y, pos.z, 1.0,   // column 3
    ]
}

fn create_cylinder_matrix(
    mid: Point3<f32>,
    x_axis: Vector3<f32>,
    y_axis: Vector3<f32>,
    z_axis: Vector3<f32>,
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
        c0.x, c0.y, c0.z, 0.0,      // column 0
        c1.x, c1.y, c1.z, 0.0,      // column 1
        c2.x, c2.y, c2.z, 0.0,      // column 2
        mid.x, mid.y, mid.z, 1.0,   // column 3
    ]
}
